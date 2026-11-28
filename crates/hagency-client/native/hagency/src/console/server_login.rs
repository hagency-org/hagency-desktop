//! Pasion login and user-owned device authorization. All OAuth credentials stay
//! in the Rust host. The browser receives navigation URLs and a local cookie.
use super::{COOKIE, Error, console, current, same_origin};
use super::native::{NativeError, Said};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Url};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

pub(super) mod matrix_creations;
mod profiles;
mod proof_clock;
use proof_clock::ProofClock;
pub(super) use proof_clock::server_date_ms;
mod sdk_resume;
mod shared_matrix;
mod room_binding;
pub(super) fn matrix_creation_router() -> Router {
    matrix_creations::router()
}

const CALLBACK: &str = "/console/server-login/callback";
const NONCE_COOKIE: &str = "hagency_server_login";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    origin: String,
    client_id: String,
    installation_id: String,
    name: String,
    owner: Option<String>,
    subject: Option<String>,
}
struct Pending {
    state: String,
    verifier: String,
    nonce: String,
    redirect: String,
    binding: Binding,
    known_identity_only: bool,
    expires: Instant,
}
#[derive(Clone)]
struct RemoteSession {
    matrix_source: Option<std::sync::Arc<dyn super::native::MatrixTokenSource>>,
    token: String,
    oauth_token: String,
    user_id: String,
    device: Option<AuthorizedDevice>,
    refresh_token: Option<String>,
    oauth_expires: Instant,
    authorized_until: Instant,
    /// The end of the proof in hand as the server stated it, by the wall
    /// clock. `authorized_until` is read on the monotonic clock, which stands
    /// still while the computer sleeps; after a sleep past the end it still
    /// lay ahead. The proof is held to both (see `follow_wall_clock`).
    authorized_until_ms: u64,
    /// How far the end of the window, as the local clock reads it, lies beyond
    /// `authorized_until`: what the proof clock left out. Each renewal sets it;
    /// it is zero before the first one, which keeps the lease comparison the
    /// stricter one.
    clock_allowance: Duration,
    /// The longest window a renewed proof has had since the SDK last refreshed
    /// its token for this session: what the server grants when the token's
    /// own expiry does not cut the proof short. Zero before the first renewal.
    full_window: Duration,
    /// When this session last had the SDK refresh its token.
    sdk_refreshed: Option<Instant>,
    /// What the answers to its renewals said of the server's clock.
    server_clock: ServerClock,
    invalidated: bool,
    binding: Binding,
    checked: Instant,
    expires: Instant,
}
/// What the answers to a session's renewals said of the server's clock.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ServerClock {
    /// The last one carried the server's date, or none came yet: a renewal
    /// asks for the longer window.
    Dated,
    /// The last one came without it: a renewal asks for the default window
    /// (see `proof_clock::requested_window_ms`).
    Undated,
    /// By its date this computer's clock is ahead of the server's by the whole
    /// window, as found at `at`, when the wall clock read `wall_ms`. Asked
    /// again the server answers the same until the clock is set, so the
    /// session is not asked again at once (see `clock_ahead_now`).
    Ahead { at: Instant, wall_ms: u64 },
}
/// A host-only credential; never serialized into a browser response.
#[derive(Clone)]
pub struct AuthorizedDevice {
    origin: String,
    device_id: String,
    generation: u64,
    token: String,
    owner_mxid: String,
    issuer: String,
    subject: String,
    user_id: String,
    valid_until: Instant,
    /// The proof's end by the wall clock, as on the session it came from.
    valid_until_ms: u64,
    clock_allowance: Duration,
    revoked: std::sync::Arc<AtomicBool>,
    stopped: std::sync::Arc<AtomicBool>,
}
#[cfg(test)]
pub(super) fn fixture_device(
    origin: &str,
    subject: &str,
    mxid: &str,
) -> (AuthorizedDevice, std::sync::Arc<AtomicBool>) {
    let revoked = std::sync::Arc::new(AtomicBool::new(false));
    (
        AuthorizedDevice {
            origin: origin.into(),
            issuer: format!("{}auth/", origin),
            subject: subject.into(),
            owner_mxid: mxid.into(),
            user_id: subject.into(),
            device_id: "fixture_device".into(),
            generation: 1,
            token: "e".repeat(64),
            valid_until: Instant::now() + Duration::from_secs(30),
            valid_until_ms: u64::MAX,
            clock_allowance: Duration::ZERO,
            revoked: revoked.clone(),
            stopped: std::sync::Arc::new(AtomicBool::new(false)),
        },
        revoked,
    )
}
impl AuthorizedDevice {
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn issuer(&self) -> &str {
        &self.issuer
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Check immediately before sending; callers must not persist snapshots.
    /// The proof ends at the earlier of its two ends: the monotonic one, and
    /// the one the server stated, read with the wall clock, which runs on
    /// while the computer sleeps.
    pub fn bearer(&self) -> Result<&str, Error> {
        if self.stopped.load(Ordering::Acquire)
            || self.revoked.load(Ordering::Acquire)
            || self.valid_until <= Instant::now()
            || wall_now_ms() >= self.valid_until_ms
        {
            Err(Error::Unauthorized)
        } else {
            Ok(&self.token)
        }
    }
    pub fn user_id(&self) -> &str {
        &self.user_id
    }
    pub fn owner_mxid(&self) -> &str {
        &self.owner_mxid
    }
    pub fn valid_until(&self) -> Instant {
        self.valid_until
    }
    /// The end of the authorization window for a deadline the server derived
    /// from that window and the host reads with its local clock, such as an
    /// execution lease: `valid_until` plus what the proof clock left out of
    /// the window. Every use of the credential itself keeps to `valid_until`.
    pub fn window_until(&self) -> Instant {
        self.valid_until + self.clock_allowance
    }
    /// A device whose window ends `window` from now and whose proof clock
    /// left `taken` of it out.
    #[cfg(test)]
    pub(super) fn with_fixture_window(mut self, window: Duration, taken: Duration) -> Self {
        self.valid_until = Instant::now() + window - taken;
        self.clock_allowance = taken;
        self
    }
}
impl RemoteSession {
    /// Holds the proof in hand to the end the server stated for it, read with
    /// the wall clock, as well as to its monotonic end: the earlier one ends
    /// it. After a sleep past that end the proof is over and is renewed at
    /// once, where on the monotonic clock alone it still lay ahead. It only
    /// ever shortens the proof, and leaves it as it is while the two ends lie
    /// within `WALL_CLOCK_SLACK` of each other.
    fn follow_wall_clock(&mut self) {
        let left = Duration::from_millis(self.authorized_until_ms.saturating_sub(wall_now_ms()));
        if let Some(by_wall) = Instant::now().checked_add(left)
            && by_wall + WALL_CLOCK_SLACK < self.authorized_until
        {
            self.authorized_until = by_wall;
        }
    }
    /// Whether the last renewal found this computer's clock ahead of the
    /// server's by the whole window, and nothing since says that changed: the
    /// wall clock has not been set, and `CLOCK_AHEAD_SPACING` has not passed.
    fn clock_ahead_now(&self) -> bool {
        let ServerClock::Ahead { at, wall_ms } = self.server_clock else {
            return false;
        };
        let since = at.elapsed();
        let expected = wall_ms.saturating_add(u64::try_from(since.as_millis()).unwrap_or(u64::MAX));
        since < CLOCK_AHEAD_SPACING
            && wall_now_ms().abs_diff(expected) <= WALL_CLOCK_SLACK.as_millis() as u64
    }
}
/// How long a session whose renewal found this computer's clock ahead of the
/// server's by the whole window is not asked again, unless the wall clock is
/// set meanwhile.
const CLOCK_AHEAD_SPACING: Duration = Duration::from_secs(30);
/// How far apart the two ends of a proof may lie before the wall clock cuts
/// it: less is the rounding of the two readings to the millisecond, or a
/// clock that is slewed, not a sleep or a clock that was set.
const WALL_CLOCK_SLACK: Duration = Duration::from_secs(1);
/// This computer's wall clock, in milliseconds since 1970; zero before then.
fn wall_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}
/// The earliest end the server stated for the proofs in `bodies`, by its
/// clock. A body that states none sets no end.
fn stated_end(bodies: &[&Value]) -> u64 {
    bodies
        .iter()
        .filter_map(|body| body["validUntilMs"].as_u64())
        .min()
        .unwrap_or(u64::MAX)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Revocation {
    origin: String,
    client_id: String,
    session_token: String,
    oauth_token: String,
    hint: String,
    session_done: bool,
    oauth_done: bool,
}
pub(super) struct ServerLogin {
    sdk_resume: sdk_resume::Journal,
    pub(super) native_expected: std::sync::Mutex<Option<(String, String)>>,
    path: Option<PathBuf>,
    profiles_path: Option<PathBuf>,
    profiles: Mutex<profiles::Profiles>,
    transition: Mutex<()>,
    renewal: Mutex<()>,
    /// The session whose last renewal got no answer, and when.
    unanswered: std::sync::Mutex<Option<(String, Instant)>>,
    worker_started: AtomicBool,
    stopped: std::sync::Arc<AtomicBool>,
    revocations_path: Option<PathBuf>,
    revocations: Mutex<Vec<Revocation>>,
    binding: Mutex<Option<Binding>>,
    pending: Mutex<Option<Pending>>,
    status: Mutex<Value>,
    sessions: Mutex<std::collections::HashMap<String, RemoteSession>>,
}
pub(in crate::console) fn read_private_json(path: &Path, limit: u64) -> Result<Vec<u8>, Error> {
    use std::io::Read;
    let file = hagency_store::private::open(path, false).map_err(|_| Error::Unavailable)?;
    if file.metadata().map_err(|_| Error::Unavailable)?.len() > limit {
        return Err(Error::Unavailable);
    }
    let mut raw = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut raw)
        .map_err(|_| Error::Unavailable)?;
    if raw.len() as u64 > limit {
        return Err(Error::Unavailable);
    }
    Ok(raw)
}
impl ServerLogin {
    pub(super) fn new(state: Option<&Path>) -> Result<Self, Error> {
        let path = state.map(|dir| dir.join("server-login.json"));
        let binding = match &path {
            Some(path) if path.exists() => {
                let raw = read_private_json(path, 16384)?;
                if raw.len() > 16384 {
                    return Err(Error::Unavailable);
                }
                let value: Binding =
                    serde_json::from_slice(&raw).map_err(|_| Error::Unavailable)?;
                origin(&value.origin).map_err(|_| Error::Unavailable)?;
                if value.client_id.is_empty()
                    || value.client_id.len() > 128
                    || value.installation_id.len() < 32
                    || value.installation_id.len() > 128
                    || !value
                        .installation_id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    || value.name.trim().is_empty()
                    || value.name.chars().count() > 128
                    || value.owner.is_some() != value.subject.is_some()
                    || value
                        .owner
                        .as_ref()
                        .is_some_and(|v| v.is_empty() || v.len() > 255)
                    || value
                        .subject
                        .as_ref()
                        .is_some_and(|v| v.is_empty() || v.len() > 255)
                {
                    return Err(Error::Unavailable);
                }

                Some(value)
            }
            _ => None,
        };
        let profiles_path = state.map(|dir| dir.join("server-login-profiles.json"));
        let profiles = profiles::Profiles::load(profiles_path.as_deref(), binding.as_ref())?;
        let revocations_path = state.map(|dir| dir.join("server-login-revocations.json"));
        let revocations: Vec<Revocation> = match &revocations_path {
            Some(path) if path.exists() => {
                let raw = read_private_json(path, 1024 * 1024)?;
                if raw.len() > 1024 * 1024 {
                    return Err(Error::Unavailable);
                }
                let values: Vec<Revocation> =
                    serde_json::from_slice(&raw).map_err(|_| Error::Unavailable)?;
                if values.len() > 64
                    || values.iter().any(|v| {
                        origin(&v.origin).is_err()
                            || v.client_id.is_empty()
                            || v.client_id.len() > 128
                            || (v.session_token.is_empty() && !v.session_done)
                            || (!v.session_token.is_empty() && v.session_token.len() != 64)
                            || (!v.oauth_done && v.oauth_token.is_empty())
                            || v.oauth_token.len() > 4096
                            || !["access_token", "refresh_token"].contains(&v.hint.as_str())
                    })
                {
                    return Err(Error::Unavailable);
                }
                values
            }
            _ => vec![],
        };
        let revocation_pending = !revocations.is_empty();
        Ok(Self {
            sdk_resume: sdk_resume::Journal::open(state)?,
            native_expected: std::sync::Mutex::new(None),
            path,
            profiles_path,
            profiles: Mutex::new(profiles),
            transition: Mutex::new(()),
            renewal: Mutex::new(()),
            unanswered: std::sync::Mutex::new(None),
            revocations_path,
            revocations: Mutex::new(revocations),
            worker_started: AtomicBool::new(false),
            stopped: std::sync::Arc::new(AtomicBool::new(false)),
            binding: Mutex::new(binding),
            pending: Mutex::new(None),
            status: Mutex::new(if revocation_pending {
                json!({"state":"signed_out","deviceAuthorized":false,"transportOnline":false,"remoteRevocationPending":true})
            } else {
                json!({"state":"idle"})
            }),
            sessions: Mutex::new(std::collections::HashMap::new()),
        })
    }
    /// Start one bounded renewal task. Weak ownership ensures dropping the
    /// console ends the worker; UI activity is not required for authorization.
    pub(super) fn start_worker(console: &super::Console) {
        if console
            .0
            .server_login
            .worker_started
            .swap(true, Ordering::AcqRel)
        {
            return;
        }
        let weak = std::sync::Arc::downgrade(&console.0);
        tokio::spawn(async move {
            let mut last_revocations = Instant::now() - Duration::from_secs(10);
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let Some(inner) = weak.upgrade() else { break };
                let login = &inner.server_login;
                if login.stopped.load(Ordering::Acquire) {
                    break;
                }
                // Read against the wall clock as well, a proof that a sleep
                // ran out is renewed on the first tick after it.
                let keys: Vec<_> = login
                    .sessions
                    .lock()
                    .await
                    .iter_mut()
                    .filter_map(|(key, s)| {
                        s.follow_wall_clock();
                        (!s.invalidated
                            && (s.checked.elapsed() >= Duration::from_secs(10)
                                || s.authorized_until <= Instant::now()))
                        .then(|| key.clone())
                    })
                    .collect();
                for key in keys {
                    let _ = login.renew_session(&key).await;
                }
                if last_revocations.elapsed() >= Duration::from_secs(10) {
                    login.flush_revocations().await;
                    last_revocations = Instant::now();
                }
            }
        });
    }
    /// A remote cookie cannot turn into a local recovery login when revoked.
    pub(super) async fn validate(&self, cookie: &str) -> Result<(), Error> {
        let key = session_key(cookie);
        let session = self.sessions.lock().await.get(&key).cloned();
        let Some(session) = session else {
            return Ok(());
        };
        if session.invalidated || session.expires <= Instant::now() {
            return Err(Error::Unauthorized);
        }
        let _renewal = self.renewal.lock().await;
        self.renew_or_keep(&key).await.map(|_| ())
    }
    /// Stop snapshots immediately without waiting for network or session locks.
    pub(super) fn retire(&self) {
        self.stopped.store(true, Ordering::Release);
    }
    /// Refresh a snapshot outside the session map lock. Renewal is single-flight,
    /// while management requests and sign-out can proceed independently.
    async fn renew_session(&self, key: &str) -> Result<(), Error> {
        let _renewal = self.renewal.lock().await;
        self.renew_session_inner(key).await
    }
    // Caller holds the renewal guard through cookie-generation selection.
    async fn renew_session_inner(&self, key: &str) -> Result<(), Error> {
        let mut snapshot = self
            .sessions
            .lock()
            .await
            .get(key)
            .cloned()
            .filter(|s| !s.invalidated)
            .ok_or(Error::Unauthorized)?;
        // Neither a caller nor the worker asks the server again at once.
        if snapshot.clock_ahead_now() {
            return Err(Error::ClockAhead);
        }
        let started = Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(8), renew(&mut snapshot))
            .await
            .unwrap_or(Err(Error::Unavailable));
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(key)
            .filter(|s| !s.invalidated && s.token == snapshot.token)
            .ok_or(Error::Unauthorized)?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(Error::Unauthorized);
        }
        match result {
            Ok(()) => {
                *session = snapshot;
                self.note_renewal(key, true);
                let mut status = self.status.lock().await;
                if status["code"] == "matrix_authorization_unavailable"
                    || status["code"] == proof_clock::CLOCK_AHEAD
                {
                    *status = json!({"state":"device_authorized","deviceAuthorized":true,"transportOnline":false});
                    tracing::info!(stage = "owner_renewal", "Owner authorization recovered");
                }
                drop(status);
                tracing::debug!(
                    stage = "owner_renewal",
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    remaining_ms = session
                        .authorized_until
                        .saturating_duration_since(Instant::now())
                        .as_millis() as u64,
                    "Owner authorization checked"
                );
                Ok(())
            }
            Err(Error::Unavailable) => {
                // Preserve rotated OAuth credentials, but never extend a proof
                // after a failed request. A later SDK check can recover it.
                *session = snapshot;
                self.note_renewal(key, false);
                tracing::warn!(
                    stage = "owner_renewal",
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    remaining_ms = session
                        .authorized_until
                        .saturating_duration_since(Instant::now())
                        .as_millis() as u64,
                    code = "matrix_authorization_unavailable",
                    "Owner renewal will retry"
                );
                *self.status.lock().await = json!({"state":"failed","code":"matrix_authorization_unavailable","deviceAuthorized":false,"transportOnline":false});
                Err(Error::Unavailable)
            }
            Err(Error::ClockAhead) => {
                // The server answered, and by its date this computer's clock
                // is ahead of it by the whole window: asked again, it answers
                // the same until the clock is set. Nothing was refused, so the
                // session is kept, with the proof that answer ended.
                *session = snapshot;
                session.server_clock = ServerClock::Ahead {
                    at: Instant::now(),
                    wall_ms: wall_now_ms(),
                };
                self.note_renewal(key, true);
                tracing::warn!(
                    stage = "owner_renewal",
                    code = proof_clock::CLOCK_AHEAD,
                    "This computer's clock is ahead of the server's by the whole authorization window; it has to be set"
                );
                *self.status.lock().await = json!({"state":"failed","code":proof_clock::CLOCK_AHEAD,"deviceAuthorized":false,"transportOnline":false});
                Err(Error::ClockAhead)
            }
            Err(error) => {
                self.note_renewal(key, true);
                if session.matrix_source.is_some() {
                    // An SDK account switch can race local retirement. Renewal
                    // refusal does not prove that the saved original account
                    // was revoked: close this process's capability only. A
                    // fresh same-owner resume independently verifies the
                    // original device; explicit logout persists its tombstone.
                    invalidate(session);
                } else {
                    self.queue_revocation(session).await;
                }
                *self.status.lock().await = json!({"state":"failed","code":"sign_in_required","deviceAuthorized":false,"transportOnline":false});
                Err(error)
            }
        }
    }
    /// Renews the proof of a session for a caller that is about to use it, and
    /// tells whether the proof in hand was kept because the renewal got no
    /// answer. Caller holds the renewal guard.
    ///
    /// A renewal that got no answer says nothing about the proof in hand: the
    /// server trusts it until `authorized_until`, and nothing has taken that
    /// back. Only a refusal, or the end of that window, ends it. The proof is
    /// never extended here, and one the server has answered with a shorter
    /// proof is cut to that (see `renew`). After an unanswered renewal the
    /// next callers use the proof without asking again for a moment, so that
    /// they do not wait out the same unanswered request one after another;
    /// the background worker keeps asking meanwhile. A proof that is about to
    /// end is asked for by every caller, as before.
    pub(super) async fn renew_or_keep(&self, key: &str) -> Result<bool, Error> {
        let asked_just_now = self
            .unanswered
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|(session, at)| {
                session == key && at.elapsed() < UNANSWERED_RENEWAL_SPACING
            });
        if asked_just_now && self.proof_in_hand(key, EXPIRING_PROOF).await {
            return Ok(true);
        }
        match self.renew_session_inner(key).await {
            Ok(()) => Ok(false),
            Err(Error::Unavailable) if self.proof_in_hand(key, Duration::ZERO).await => Ok(true),
            Err(error) => Err(error),
        }
    }
    /// Whether the session holds a proof with more than `margin` of its
    /// window left.
    async fn proof_in_hand(&self, key: &str, margin: Duration) -> bool {
        !self.stopped.load(Ordering::Acquire)
            && self.sessions.lock().await.get(key).is_some_and(|s| {
                !s.invalidated && s.authorized_until > Instant::now() + margin
            })
    }
    /// Records whether the renewal of a session was answered, by a grant or a
    /// refusal alike.
    fn note_renewal(&self, key: &str, answered: bool) {
        let mut unanswered = self.unanswered.lock().unwrap_or_else(|e| e.into_inner());
        if !answered {
            *unanswered = Some((key.to_owned(), Instant::now()));
        } else if unanswered
            .as_ref()
            .is_some_and(|(session, _)| session == key)
        {
            *unanswered = None;
        }
    }
    /// Only the trusted Rust runtime can obtain a current device credential.
    pub(super) async fn authorized_device(&self) -> Result<AuthorizedDevice, Error> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(Error::Unauthorized);
        }
        let _renewal = self.renewal.lock().await;
        let key = self
            .sessions
            .lock()
            .await
            .iter_mut()
            // An expired finite proof can still be refreshed. Selecting the
            // grant never authorizes execution; renewal must succeed below.
            // The proof is first held to its end by the wall clock, so that
            // one a sleep ran out is not kept as if it were still in hand.
            .find(|(_, s)| !s.invalidated)
            .map(|(key, s)| {
                s.follow_wall_clock();
                key.clone()
            })
            .ok_or(Error::Unauthorized)?;
        let kept = self.renew_or_keep(&key).await?;
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(&key)
            .filter(|s| !s.invalidated)
            .ok_or(Error::Unauthorized)?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(Error::Unauthorized);
        }
        if session.authorized_until <= Instant::now() {
            // A window that ran out while the server could not be asked comes
            // back with the next answered renewal: it was not refused.
            return Err(if kept {
                Error::Unavailable
            } else {
                Error::Unauthorized
            });
        }
        let mut device = session.device.clone().ok_or(Error::Unauthorized)?;
        device.valid_until = session.authorized_until;
        device.valid_until_ms = session.authorized_until_ms;
        device.clock_allowance = session.clock_allowance;
        device.stopped = self.stopped.clone();
        Ok(device)
    }
    async fn queue_revocation(&self, session: &mut RemoteSession) {
        if session.invalidated {
            return;
        }
        let shared = session.matrix_source.is_some();
        if shared {
            // A later resume must not race an explicit or terminal revocation.
            if self.sdk_resume.revoke(Some(&session.token)).is_err() {
                tracing::error!(
                    code = "native_logout_persistence_failed",
                    "Native session revocation could not be persisted"
                );
            }
        }
        let (oauth_token, hint) = if shared {
            session.oauth_token.clear();
            session.refresh_token = None;
            (String::new(), "access_token")
        } else {
            match session.refresh_token.take() {
                Some(token) => (token, "refresh_token"),
                None => (std::mem::take(&mut session.oauth_token), "access_token"),
            }
        };
        let pending = Revocation {
            origin: session.binding.origin.clone(),
            client_id: session.binding.client_id.clone(),
            session_token: std::mem::take(&mut session.token),
            oauth_token,
            hint: hint.to_owned(),
            session_done: false,
            oauth_done: shared,
        };
        invalidate(session);
        let mut revocations = self.revocations.lock().await;
        // The saved SDK record may already have queued this exact native
        // credential before the in-memory session is retired. Preserve any
        // existing OAuth cleanup, and retry the original remote revoke once.
        if !shared
            || !revocations.iter().any(|r| {
                r.origin == pending.origin
                    && r.client_id == pending.client_id
                    && r.session_token == pending.session_token
            })
        {
            revocations.push(pending);
        }
        let _ = self.save_revocations(&revocations);
    }
    pub(super) async fn revoke_native_all(&self) {
        self.queue_saved_sdk_revocation().await;
        for session in self.sessions.lock().await.values_mut() {
            self.queue_revocation(session).await;
        }
        self.pending.lock().await.take();
    }
    pub(super) async fn flush_native_revocations(&self) {
        self.flush_revocations().await;
    }
    pub(super) async fn sign_out(&self, cookie: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(&session_key(cookie)) {
            // Persist only a pending revoke record, never a usable runtime
            // credential, before network operations. Stop snapshots immediately.
            self.queue_revocation(session).await;
            *self.status.lock().await = json!({"state":"signed_out","deviceAuthorized":false,"transportOnline":false,"remoteRevocationPending":true});
        }
        drop(sessions);
        self.flush_revocations().await;
    }
    fn save_revocations(&self, values: &[Revocation]) -> Result<(), Error> {
        let path = self.revocations_path.as_ref().ok_or(Error::Unavailable)?;
        let raw = serde_json::to_vec(values).map_err(|_| Error::Unavailable)?;
        hagency_store::private::replace(path, &raw).map_err(|_| Error::Unavailable)
    }
    async fn revoke_failed_login(
        &self,
        binding: &Binding,
        session_token: &str,
        token: &str,
        refresh_token: Option<&str>,
    ) -> Result<(), &'static str> {
        let mut revocations = self.revocations.lock().await;
        revocations.push(Revocation {
            origin: binding.origin.clone(),
            client_id: binding.client_id.clone(),
            session_token: session_token.into(),
            oauth_token: refresh_token.unwrap_or(token).into(),
            hint: if refresh_token.is_some() {
                "refresh_token"
            } else {
                "access_token"
            }
            .into(),
            session_done: session_token.is_empty(),
            oauth_done: false,
        });
        self.save_revocations(&revocations)
            .map_err(|_| "local_state_unavailable")?;
        drop(revocations);
        self.flush_revocations().await;
        Ok(())
    }
    async fn flush_revocations(&self) {
        let mut pending = self.revocations.lock().await;
        if pending.is_empty() {
            return;
        }
        let Ok(client) = http() else { return };
        for record in pending.iter_mut() {
            let Ok(server) = origin(&record.origin) else {
                continue;
            };
            if !record.session_done
                && let Ok(reply) = client
                    .delete(server.join("/hagency/api/v1/sessions/current").unwrap())
                    .bearer_auth(&record.session_token)
                    .send()
                    .await
            {
                // Expired/previously revoked sessions are already inactive.
                record.session_done = reply.status().is_success()
                    || reply.status() == reqwest::StatusCode::UNAUTHORIZED;
            }
            if !record.oauth_done
                && let Ok(reply) = client
                    .post(server.join("/auth/oauth2/revoke").unwrap())
                    .form(&[
                        ("client_id", record.client_id.as_str()),
                        ("token", record.oauth_token.as_str()),
                        ("token_type_hint", record.hint.as_str()),
                    ])
                    .send()
                    .await
            {
                record.oauth_done = reply.status().is_success();
            }
        }
        pending.retain(|p| !(p.session_done && p.oauth_done));
        let _ = self.save_revocations(&pending);
        if pending.is_empty() {
            let mut status = self.status.lock().await;
            if status["state"] == "signed_out" {
                status["remoteRevocationPending"] = json!(false);
            }
        }
    }
    async fn save(&self, binding: Binding) -> Result<(), &'static str> {
        let path = self.path.as_ref().ok_or("local_state_unavailable")?;
        let raw = serde_json::to_vec(&binding).map_err(|_| "local_state_unavailable")?;
        self.save_profile(&binding).await?;
        hagency_store::private::replace(path, &raw).map_err(|_| "local_state_unavailable")?;
        *self.binding.lock().await = Some(binding);
        Ok(())
    }
}
fn session_key(cookie: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(cookie.as_bytes()))
}
fn invalidate(session: &mut RemoteSession) {
    session.invalidated = true;
    session.authorized_until = Instant::now();
    if let Some(device) = session.device.take() {
        device.revoked.store(true, Ordering::Release);
    }
    session.oauth_token.clear();
    session.refresh_token = None;
    session.token.clear();
}
fn oauth_credentials(tokens: &Value) -> Result<(String, Option<String>, Duration), &'static str> {
    if tokens["token_type"]
        .as_str()
        .is_some_and(|t| !t.eq_ignore_ascii_case("bearer"))
    {
        return Err("invalid_server_response");
    }
    let token = tokens["access_token"]
        .as_str()
        .filter(|t| !t.is_empty() && t.len() <= 4096)
        .ok_or("invalid_server_response")?
        .to_owned();
    let refresh = match tokens.get("refresh_token") {
        Some(value) => Some(
            value
                .as_str()
                .filter(|t| !t.is_empty() && t.len() <= 4096)
                .ok_or("invalid_server_response")?
                .to_owned(),
        ),
        None => None,
    };
    let lifetime = tokens["expires_in"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= 31_536_000)
        .ok_or("invalid_server_response")?;
    Ok((token, refresh, Duration::from_secs(lifetime)))
}
fn may_stop_session(active: Option<&Binding>, session: &RemoteSession) -> bool {
    !session.invalidated && active.is_some_and(|binding| same_profile(binding, &session.binding))
}
fn same_profile(left: &Binding, right: &Binding) -> bool {
    left.origin == right.origin
        && left.owner.is_some()
        && left.owner == right.owner
        && left.subject.is_some()
        && left.subject == right.subject
}
fn identity_matches(identity: &Value, binding: &Binding) -> bool {
    identity["mxid"] == binding.owner.as_deref().unwrap_or("")
        && identity["subject"] == binding.subject.as_deref().unwrap_or("")
        && identity["clientId"] == binding.client_id
        && identity["issuer"] == format!("{}auth/", binding.origin)
}
fn renewal_error(code: &str) -> Error {
    // An answer that the session no longer belongs to this account is a
    // refusal like the others. Read as "no answer", it left the proof in hand
    // in use until its own end.
    if matches!(
        code,
        "sign_in_required"
            | "authorization_expired"
            | "authentication_required"
            | "identity_mapping_mismatch"
    ) {
        Error::Unauthorized
    } else {
        Error::Unavailable
    }
}
/// How the session renewal reads a step that failed: by `said()`, as every
/// failure of this host is read, with the status and code the server gave.
/// Of what is decided, a refusal of the credential (401, 403) and the codes
/// of `renewal_error` say that the session is gone, and end it. Any other
/// decision is about the request: the server still trusts the proof in hand
/// to its end, so it is kept, as it is after a request that got no answer;
/// a session is never cut by a refusal that is not about it. What was not
/// decided keeps it too, so an answer with no code of the server's in it,
/// such as a 401 from something on the way, signs no one out.
fn renewal_refused(refusal: NativeError) -> Error {
    match refusal.said() {
        Said::Decided if matches!(refusal.status, 401 | 403) => Error::Unauthorized,
        Said::Decided => {
            let read = renewal_error(&refusal.code);
            if matches!(read, Error::Unavailable) {
                tracing::warn!(
                    stage = "owner_renewal",
                    status = refusal.status,
                    code = %refusal.code,
                    "The server refused a renewal for a reason of the request; the proof in hand is kept to its end"
                );
            }
            read
        }
        Said::NotAsked | Said::ScopeReplaced | Said::NoAnswer => Error::Unavailable,
    }
}
/// A failure of a renewal step that this host makes itself.
fn renewal_failure(status: u16, code: &str) -> NativeError {
    NativeError {
        status,
        code: code.into(),
    }
}
/// Asks the server to trust a renewed proof for longer than its default
/// thirty seconds, so that a host that stalls for a while keeps its
/// authorization and the leases that end with it. A server that grants no more
/// ignores the header; one that does cuts every lease to the window it gave.
const AUTHORIZATION_WINDOW_HEADER: &str = "x-hagency-authorization-window-ms";
/// Tells the server that this host takes an activity lease as long as the
/// window of its session, not thirty seconds at most. The server then lets
/// the lease of a working process last as long as the window it granted, and
/// no longer; commands, tickets and delivery leases stay as they are. Every
/// renewal says it again. A server that does not know the header issues
/// thirty seconds, which this host takes as before.
const ACTIVITY_WINDOW_HEADER: &str = "x-hagency-activity-window";
const ACTIVITY_WINDOW_FOLLOWS_SESSION: &str = "session";
async fn renew(session: &mut RemoteSession) -> Result<(), Error> {
    if session.invalidated {
        return Err(Error::Unauthorized);
    }
    // A proof a sleep ran out is not taken as still in hand below.
    session.follow_wall_clock();
    // Every execution RPC still reaches the server's current lease/membership
    // gates. A finite SDK proof need not be re-requested before AND after each
    // poll: refresh it at most five seconds apart, or before its deadline.
    if session.matrix_source.is_some()
        && session.checked.elapsed() < Duration::from_secs(5)
        && session.authorized_until > Instant::now() + Duration::from_secs(10)
        && session.oauth_expires > Instant::now() + Duration::from_secs(10)
    {
        return Ok(());
    }
    if session.matrix_source.is_none()
        && session.checked.elapsed() < Duration::from_secs(20)
        && session.authorized_until > Instant::now()
        && session.oauth_expires > Instant::now() + Duration::from_secs(10)
    {
        return Ok(());
    }
    let server = origin(&session.binding.origin).map_err(|_| Error::Unauthorized)?;
    let client = http().map_err(|_| Error::Unavailable)?;
    let shared_token = if let Some(source) = &session.matrix_source {
        let snapshot = source.access_token().await.map_err(|error| {
            let code = super::native::sdk_source_failure(error);
            eprintln!("hagency owner renewal failure: stage=sdk_source code={code}");
            if code == "matrix_authorization_unavailable" {
                Error::Unavailable
            } else {
                Error::Unauthorized
            }
        })?;
        if snapshot.client_id != session.binding.client_id || snapshot.access_token.is_empty() {
            return Err(Error::Unauthorized);
        }
        Some(snapshot.access_token)
    } else {
        None
    };
    if shared_token.is_none() && session.oauth_expires <= Instant::now() + Duration::from_secs(10) {
        if let Some(refresh) = &session.refresh_token {
            let tokens = oauth_refresh_response(
                client
                    .post(server.join("/auth/oauth2/token").unwrap())
                    .form(&[
                        ("grant_type", "refresh_token"),
                        ("client_id", session.binding.client_id.as_str()),
                        ("refresh_token", refresh),
                    ])
                    .send()
                    .await
                    .map_err(|_| Error::Unavailable)?,
            )
            .await?;
            let (token, next_refresh, lifetime) =
                oauth_credentials(&tokens).map_err(|_| Error::Unavailable)?;
            session.oauth_token = token;
            if next_refresh.is_some() {
                session.refresh_token = next_refresh;
            }
            session.oauth_expires = Instant::now() + lifetime;
        } else if session.oauth_expires <= Instant::now() {
            return Err(Error::Unauthorized);
        }
    }
    let request_started = Instant::now();
    let renewal_response = client
        .post(
            server
                .join("/hagency/api/v1/sessions/current/renew")
                .unwrap(),
        )
        .bearer_auth(&session.token)
        .header(AUTHORIZATION_WINDOW_V2_HEADER, requested_window_ms(session))
        .header(AUTHORIZATION_WINDOW_HEADER, requested_window_ms(session))
        .header(ACTIVITY_WINDOW_HEADER, ACTIVITY_WINDOW_FOLLOWS_SESSION)
        .json(&json!({"accessToken":shared_token.as_ref().unwrap_or(&session.oauth_token)}))
        .send()
        .await
        .map_err(|_| {
            eprintln!("hagency owner renewal failure: stage=server_session_transport");
            Error::Unavailable
        })?;
    if !renewal_response.status().is_success() {
        eprintln!(
            "hagency owner renewal failure: stage=server_session status={}",
            renewal_response.status().as_u16()
        );
    }
    let (renewed, clock) =
        proof_answer(request_started, renewal_response).await.map_err(renewal_refused)?;
    // The next renewal asks for the window this answer could show.
    session.server_clock = if clock.dated() {
        ServerClock::Dated
    } else {
        ServerClock::Undated
    };
    // The server's session now ends with this proof, by its clock too.
    let mut stated = stated_end(&[&renewed]);
    session.authorized_until_ms = session.authorized_until_ms.min(stated);
    let (mut until, mut taken) = match clock.bounds(&renewed) {
        Ok(bounds) => bounds,
        Err(refusal) => {
            // The server answered with a proof this host cannot use, one that
            // has no time left for instance. Its session ends with that proof
            // all the same, so the one in hand is cut to it before the
            // renewal is reported as not made, or, for a clock ahead of the
            // server's, as decided.
            if let Some(end) = proof_clock::local_end(&renewed)
                && end < session.authorized_until
            {
                session.authorized_until = end;
                session.clock_allowance = Duration::ZERO;
            }
            return Err(unusable_proof(refusal));
        }
    };
    // The server has answered, and its session now ends with this proof. If
    // a later step gets no answer, the proof that stays in hand is the one
    // the server holds: an earlier one that ran longer no longer stands.
    if until < session.authorized_until {
        session.authorized_until = until;
        session.clock_allowance = taken;
    }
    let mut proof_window = window_left(until, taken);
    if let Some(source) = session.matrix_source.clone() {
        // The server never trusts a proof past the token's own expiry, so an
        // ageing token returns a shorter proof than the window the server
        // grants. Refresh it once: when the proof is about to end, and already
        // when it has lost a good part of that window, so that a host that
        // then stalls still has most of the window ahead of it.
        // Decided on the end the server counted, not on the one this
        // computer's clock cut short: a clock that is ahead shortens every
        // proof alike, and a fresh token gives none of that time back.
        let expiring =
            clock.counted_end(&renewed).unwrap_or(until) <= Instant::now() + EXPIRING_PROOF;
        if expiring
            || proof_shortened(
                proof_window,
                session.full_window,
                session.sdk_refreshed.map(|at| at.elapsed()),
            )
        {
            session.sdk_refreshed = Some(Instant::now());
            match renew_with_refreshed_token(
                &client,
                &server,
                &session.token,
                &session.binding.client_id,
                source.as_ref(),
                requested_window_ms(session),
            )
            .await
            {
                Ok((refreshed_until, refreshed_taken, refreshed_end)) => {
                    (until, taken) = (refreshed_until, refreshed_taken);
                    stated = refreshed_end;
                    proof_window = window_left(until, taken);
                    // The window a fresh token gets, also when the server now
                    // grants a shorter one than before.
                    session.full_window = Duration::ZERO;
                }
                // The proof in hand still holds. A refresh made ahead of the
                // token's expiry is asked again later; one that got no answer
                // must not take that proof away.
                Err(Error::Unavailable) if !expiring => {}
                Err(error) => return Err(error),
            }
        }
    }
    let (identity, clock) = identity_answer(
        &client,
        &server,
        &session.token,
    )
    .await
    .map_err(|refusal| {
        tracing::warn!(
            stage = "server_identity",
            status = refusal.status,
            code = %refusal.code,
            "Owner renewal failed"
        );
        renewal_refused(refusal)
    })?;
    if !identity_matches(&identity, &session.binding) || identity["userId"] != session.user_id {
        return Err(Error::Unauthorized);
    }
    let (identity_until, identity_taken) =
        clock.bounds(&identity).map_err(unusable_proof)?;
    // The same window as the local clock alone reads it. A lease the server
    // cuts to this window is read that way too.
    let mut window = (until + taken).min(identity_until + identity_taken);
    if let Some(source) = &session.matrix_source {
        let after = source.access_token().await.map_err(|error| {
            let code = super::native::sdk_source_failure(error);
            eprintln!("hagency owner renewal failure: stage=sdk_source code={code}");
            if code == "matrix_authorization_unavailable" {
                Error::Unavailable
            } else {
                Error::Unauthorized
            }
        })?;
        if after.client_id != session.binding.client_id {
            return Err(Error::Unauthorized);
        }
        session.oauth_expires = until.min(identity_until);
    } else {
        window = window.min(session.oauth_expires);
    }
    session.authorized_until = until.min(identity_until).min(session.oauth_expires);
    session.authorized_until_ms = stated.min(stated_end(&[&identity]));
    session.clock_allowance = window.saturating_duration_since(session.authorized_until);
    session.full_window = session.full_window.max(proof_window);
    session.checked = Instant::now();
    Ok(())
}
/// How long callers use the proof in hand after a renewal went unanswered
/// before one of them asks again: the spacing of two checks of a proof.
const UNANSWERED_RENEWAL_SPACING: Duration = Duration::from_secs(5);
/// A proof that ends this soon is taken as the token itself running out.
const EXPIRING_PROOF: Duration = Duration::from_secs(20);
/// How much of its full window a renewed proof has to have lost before the
/// host has the SDK refresh its token for it. With the default window of
/// thirty seconds this is the moment the proof counts as expiring anyway, so
/// a server that grants no longer window sees the same refreshes as before.
const SHORTENED_PROOF: Duration = Duration::from_secs(10);
/// The least time between two refreshes made for a shortened proof. A token
/// that lives shorter than the window cannot be refreshed into a full proof,
/// and is refreshed when it runs out, as before.
const SHORTENED_PROOF_REFRESH_SPACING: Duration = Duration::from_secs(60);
/// Asks for the window of `AUTHORIZATION_WINDOW_HEADER`, with the same value,
/// and says that this host keeps one local session for each server session.
/// Under a longer window a host that made one local session per call would
/// reach its limit and refuse its own calls, so a server that knows this
/// header grants more than thirty seconds only to a request that carries it.
/// A server from before it reads the other header, which is sent as well.
const AUTHORIZATION_WINDOW_V2_HEADER: &str = "x-hagency-authorization-window-v2-ms";
/// What is left of a proof's window as the local clock reads it: the time to
/// its deadline plus what the proof clock left out of the window.
fn window_left(until: Instant, taken: Duration) -> Duration {
    (until + taken).saturating_duration_since(Instant::now())
}
/// Whether a proof with `window` left has lost enough of `full`, the window a
/// fresh token got, for a refresh, given how long ago the last one was asked.
fn proof_shortened(window: Duration, full: Duration, refreshed: Option<Duration>) -> bool {
    window + SHORTENED_PROOF <= full
        && refreshed.is_none_or(|ago| ago >= SHORTENED_PROOF_REFRESH_SPACING)
}
/// The window a renewal of `session` asks for. While the server's answers come
/// without its date, a longer window could not be held, and is not asked for.
fn requested_window_ms(session: &RemoteSession) -> u64 {
    proof_clock::requested_window_ms(session.server_clock != ServerClock::Undated)
}
/// What a renewal reports for an answer whose proof this host cannot use: a
/// clock ahead of the server's by the whole window is decided, and named;
/// any other such answer, with no time left for instance, is not.
fn unusable_proof(refusal: &'static str) -> Error {
    if refusal == proof_clock::CLOCK_AHEAD {
        Error::ClockAhead
    } else {
        Error::Unavailable
    }
}
/// Has the SDK refresh its token and renews the proof with the new one, asking
/// for `window_ms`: its bounds, and the end the server stated for it.
async fn renew_with_refreshed_token(
    client: &Client,
    server: &Url,
    session_token: &str,
    client_id: &str,
    source: &dyn super::native::MatrixTokenSource,
    window_ms: u64,
) -> Result<(Instant, Duration, u64), Error> {
    let refreshed = source.refresh_access_token().await.map_err(|error| {
        let code = super::native::sdk_source_failure(error);
        eprintln!("hagency owner renewal failure: stage=sdk_refresh code={code}");
        if code == "matrix_authorization_unavailable" {
            Error::Unavailable
        } else {
            Error::Unauthorized
        }
    })?;
    if refreshed.client_id != client_id || refreshed.access_token.is_empty() {
        return Err(Error::Unauthorized);
    }
    let request_started = Instant::now();
    let response = client
        .post(
            server
                .join("/hagency/api/v1/sessions/current/renew")
                .unwrap(),
        )
        .bearer_auth(session_token)
        .header(AUTHORIZATION_WINDOW_V2_HEADER, window_ms)
        .header(AUTHORIZATION_WINDOW_HEADER, window_ms)
        .header(ACTIVITY_WINDOW_HEADER, ACTIVITY_WINDOW_FOLLOWS_SESSION)
        .json(&json!({"accessToken":refreshed.access_token}))
        .send()
        .await
        .map_err(|_| Error::Unavailable)?;
    if !response.status().is_success() {
        eprintln!(
            "hagency owner renewal failure: stage=server_after_refresh status={}",
            response.status().as_u16()
        );
    }
    let (refreshed, clock) =
        proof_answer(request_started, response).await.map_err(renewal_refused)?;
    let (until, taken) = clock.bounds(&refreshed).map_err(unusable_proof)?;
    Ok((until, taken, stated_end(&[&refreshed])))
}
fn random() -> Result<String, &'static str> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| "random_unavailable")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
pub(super) fn origin(value: &str) -> Result<Url, &'static str> {
    crate::server_admission::normalize_origin(value).map_err(|e| e.code())
}
pub(super) fn http() -> Result<Client, &'static str> {
    static CLIENT: std::sync::LazyLock<Result<Client, &'static str>> =
        std::sync::LazyLock::new(|| {
            Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|_| "server_unavailable")
        });
    CLIENT.clone()
}
async fn read_response(mut response: reqwest::Response) -> Result<Value, &'static str> {
    let mut raw = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "server_unavailable")? {
        if raw.len() + chunk.len() > 128 * 1024 {
            return Err("invalid_server_response");
        }
        raw.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&raw).map_err(|_| "invalid_server_response")
}
async fn oauth_refresh_response(response: reqwest::Response) -> Result<Value, Error> {
    let status = response.status();
    let value = read_response(response)
        .await
        .map_err(|_| Error::Unavailable)?;
    if !status.is_success() {
        return Err(
            if matches!(status.as_u16(), 400 | 401 | 403)
                && matches!(
                    value["error"].as_str(),
                    Some("invalid_grant" | "invalid_client" | "unauthorized_client")
                )
            {
                Error::Unauthorized
            } else {
                Error::Unavailable
            },
        );
    }
    Ok(value)
}
async fn response(response: reqwest::Response) -> Result<Value, &'static str> {
    let status = response.status();
    let value = read_response(response).await?;
    if !status.is_success() {
        if matches!(status.as_u16(), 401 | 403) {
            return Err("sign_in_required");
        }
        return Err(match value["code"].as_str() {
            Some("sign_in_required") => "sign_in_required",
            Some("identity_mapping_mismatch") => "identity_mapping_mismatch",
            _ => "server_request_failed",
        });
    }
    Ok(value)
}
async fn proof_response(started: Instant, reply: reqwest::Response) -> Result<(Value, ProofClock), &'static str> {
    let clock = ProofClock::new(started, reply.headers());
    Ok((response(reply).await?, clock))
}
async fn get_identity(client: &Client, server: &Url, token: &str) -> Result<(Value, ProofClock), &'static str> {
    let started = Instant::now();
    let reply = client.get(server.join("/hagency/api/v1/identity").unwrap())
        .bearer_auth(token).send().await.map_err(|_| "server_unavailable")?;
    proof_response(started, reply).await
}
/// The server's answer to a step of the session renewal, or its refusal with
/// the status and code it gave, which `renewal_refused` reads by `said()`. The
/// sign-in steps read `response` instead, which keeps only the codes they
/// show. An answer with no code of the server's in it is
/// `server_request_failed`, which decides nothing.
async fn answer(reply: reqwest::Response) -> Result<Value, NativeError> {
    let status = reply.status().as_u16();
    let value = read_response(reply).await.map_err(|code| {
        if code == "server_unavailable" {
            renewal_failure(503, "server_unavailable")
        } else {
            renewal_failure(502, "invalid_server_response")
        }
    })?;
    if (200..300).contains(&status) {
        return Ok(value);
    }
    let code = value["code"]
        .as_str()
        .filter(|code| {
            !code.is_empty()
                && code.len() <= 80
                && code.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
        .unwrap_or("server_request_failed");
    Err(renewal_failure(status, code))
}
async fn proof_answer(
    started: Instant,
    reply: reqwest::Response,
) -> Result<(Value, ProofClock), NativeError> {
    let clock = ProofClock::new(started, reply.headers());
    Ok((answer(reply).await?, clock))
}
async fn identity_answer(
    client: &Client,
    server: &Url,
    token: &str,
) -> Result<(Value, ProofClock), NativeError> {
    let started = Instant::now();
    let reply = client
        .get(server.join("/hagency/api/v1/identity").unwrap())
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| renewal_failure(503, "server_unavailable"))?;
    proof_answer(started, reply).await
}
fn cookie(req: &Request, name: &str) -> Option<String> {
    req.headers()
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|v| {
            v.trim()
                .strip_prefix(&format!("{name}="))
                .map(str::to_owned)
        })
}
fn local_redirect(req: &Request) -> Result<String, &'static str> {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .ok_or("invalid_local_origin")?;
    let url =
        Url::parse(&format!("http://{host}{CALLBACK}")).map_err(|_| "invalid_local_origin")?;
    if !url.host_str().is_some_and(|h| {
        h.trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    }) {
        return Err("invalid_local_origin");
    }
    Ok(url.to_string())
}
pub(super) fn router() -> Router {
    Router::with_path("server-login")
        .get(login_status)
        .push(Router::with_path("switch").post(profiles::switch))
        .push(Router::with_path("start").post(start))
        .push(Router::with_path("callback").get(callback))
        .push(Router::with_path("sign-out").post(sign_out))
}
#[handler]
async fn login_status(req: &mut Request, depot: &Depot, res: &mut Response) {
    if !same_origin(req, depot, false) {
        super::failed(res, Error::Invalid);
        return;
    }
    let Ok(console) = console(depot) else {
        super::failed(res, Error::Unavailable);
        return;
    };
    ServerLogin::start_worker(console);
    let local_access_ready = current(req, depot).is_ok()
        && match cookie(req, COOKIE) {
            Some(value) => console.0.server_login.validate(&value).await.is_ok(),
            None => false,
        };
    let profiles = console.0.server_login.profiles_status().await;
    let binding = console.0.server_login.binding.lock().await;
    res.render(Json(json!({"localAccessReady":local_access_ready,"profiles":profiles["profiles"],"activeProfileId":profiles["activeProfileId"],"configured":binding.as_ref().is_some_and(|b|b.owner.is_some()),"server":binding.as_ref().map(|b|&b.origin),"name":binding.as_ref().map(|b|&b.name),"status":*console.0.server_login.status.lock().await})));
}
#[handler]
async fn sign_out(req: &mut Request, depot: &Depot, res: &mut Response) {
    if !same_origin(req, depot, true)
        || req.uri().query().is_some()
        || !super::body(req, 1).await.is_ok_and(|b| b.is_empty())
    {
        super::failed(res, Error::Unauthorized);
        return;
    }
    let Ok(console) = console(depot) else {
        super::failed(res, Error::Unavailable);
        return;
    };
    let _transition = console.0.server_login.transition.lock().await;
    let Some(value) = cookie(req, COOKIE) else {
        super::failed(res, Error::Unauthorized);
        return;
    };
    let active = console.0.server_login.binding.lock().await.clone();
    let permitted = console
        .0
        .server_login
        .sessions
        .lock()
        .await
        .get(&session_key(&value))
        .is_some_and(|session| may_stop_session(active.as_ref(), session));
    if !permitted {
        super::failed(res, Error::Unauthorized);
        return;
    }
    let bridge_allowed = console.0.authority.authenticate(&value).is_ok()
        && console.0.server_login.validate(&value).await.is_ok();
    if let Err(error) = console.0.authority.revoke_all() {
        super::failed(res, error);
        return;
    }
    *console.0.server_login.pending.lock().await = None;
    {
        let mut sessions = console.0.server_login.sessions.lock().await;
        for session in sessions.values_mut() {
            console.0.server_login.queue_revocation(session).await;
        }
    }
    *console.0.server_login.status.lock().await = json!({"state":"signed_out","deviceAuthorized":false,"transportOnline":false,"remoteRevocationPending":true});
    console.stop_owned_runtimes().await;
    console.stop_owner_provider().await;
    {
        let records = console.0.server_login.revocations.lock().await;
        if let Err(error) = console.0.server_login.save_revocations(&records) {
            super::failed(res, error);
            return;
        }
    }
    console.0.server_login.flush_revocations().await;
    let new_cookie = if bridge_allowed {
        console
            .0
            .authority
            .issue_owner_ticket()
            .and_then(|ticket| console.0.authority.exchange(&ticket))
    } else {
        Ok(String::new())
    };
    let Ok(new_cookie) = new_cookie else {
        super::failed(res, Error::Unavailable);
        return;
    };
    res.add_header(
        "set-cookie",
        if new_cookie.is_empty() {
            format!("{COOKIE}=; HttpOnly; SameSite=Strict; Path=/console; Max-Age=0")
        } else {
            format!("{COOKIE}={new_cookie}; HttpOnly; SameSite=Strict; Path=/console")
        },
        true,
    )
    .unwrap();
    res.render(Json(json!({"ok":true})));
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    server: String,
    name: String,
}
#[handler]
async fn start(req: &mut Request, depot: &Depot, res: &mut Response) {
    let result = begin(req, depot).await;
    match result {
        Ok((url, nonce)) => {
            res.add_header("set-cookie",format!("{NONCE_COOKIE}={nonce}; HttpOnly; SameSite=Lax; Path=/console/server-login; Max-Age=300"),true).unwrap();
            res.render(Json(json!({"url":url})));
        }
        Err(code) => {
            res.status_code(if code.ends_with("unavailable") {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::BAD_REQUEST
            });
            res.render(Json(json!({"code":code})));
        }
    }
}
async fn begin(req: &mut Request, depot: &Depot) -> Result<(String, String), &'static str> {
    if !same_origin(req, depot, true) {
        return Err("invalid_local_origin");
    }
    let console = console(depot).map_err(|_| "local_state_unavailable")?;
    let login = &console.0.server_login;
    let _transition = login.transition.lock().await;
    let binding = login.binding.lock().await.clone();
    let local_access = current(req, depot).is_ok()
        && match cookie(req, COOKIE) {
            Some(value) => login.validate(&value).await.is_ok(),
            None => false,
        };
    let raw = super::body(req, 8192).await.map_err(|_| "invalid_login")?;
    let input: Start = serde_json::from_slice(&raw).map_err(|_| "invalid_login")?;
    let server = origin(&input.server)?;
    let binding = if local_access {
        binding
    } else {
        // Expired browser authority may authenticate an already admitted identity,
        // but may neither select a new owner nor modify the active binding yet.
        let mut known = login
            .known_server(server.as_str())
            .await
            .ok_or("local_access_required")?;
        known.owner = None;
        known.subject = None;
        Some(known)
    };

    if input.name.trim().is_empty() || input.name.chars().count() > 128 {
        return Err("invalid_name");
    }
    if binding
        .as_ref()
        .is_some_and(|b| b.owner.is_some() && b.origin != server.as_str())
    {
        return Err("owner_mismatch");
    }
    if login.revocations.lock().await.len() >= 63 {
        return Err("local_session_limit");
    }
    let client = http()?;
    crate::server_admission::discover(server.as_str())
        .await
        .map_err(|e| e.code())?;
    let mut binding = match binding.filter(|b| b.origin == server.as_str()) {
        Some(binding) => binding,
        None => {
            let redirect = local_redirect(req)?;
            let mut registered = Url::parse(&redirect).unwrap();
            registered
                .set_port(None)
                .map_err(|_| "invalid_local_origin")?;
            let registration=response(client.post(server.join("/auth/oauth2/registration").unwrap()).json(&json!({"client_name":"Hagency Client","client_uri":"https://github.com/chrislearn/hagency-client","application_type":"native","token_endpoint_auth_method":"none","grant_types":["authorization_code","refresh_token"],"response_types":["code"],"redirect_uris":[registered]})).send().await.map_err(|_|"server_unavailable")?).await?;
            let client_id = registration["client_id"]
                .as_str()
                .filter(|v| v.len() <= 128)
                .ok_or("invalid_server_response")?
                .to_owned();
            Binding {
                origin: server.to_string(),
                client_id,
                installation_id: random()?,
                name: input.name.trim().to_owned(),
                owner: None,
                subject: None,
            }
        }
    };
    if binding.owner.is_none() {
        binding.name = input.name.trim().to_owned();
    }
    if local_access {
        login.save(binding.clone()).await?;
    }
    let state = random()?;
    let verifier = random()?;
    let nonce = random()?;
    let redirect = local_redirect(req)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut authorize = server.join("/auth/authorize").unwrap();
    let device = format!("Hagency{}", &binding.installation_id[..16]);
    authorize.query_pairs_mut().extend_pairs([
        ("prompt", "login"),
        ("response_type", "code"),
        ("client_id", binding.client_id.as_str()),
        ("redirect_uri", redirect.as_str()),
        ("state", state.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        (
            "scope",
            format!("urn:matrix:client:api:* urn:matrix:client:device:{device}").as_str(),
        ),
    ]);
    *login.pending.lock().await = Some(Pending {
        state,
        verifier,
        nonce: nonce.clone(),
        redirect,
        binding,
        known_identity_only: !local_access,
        expires: Instant::now() + Duration::from_secs(300),
    });
    *login.status.lock().await = json!({"state":"signing_in"});
    Ok((authorize.to_string(), nonce))
}
#[handler]
async fn callback(req: &mut Request, depot: &Depot, res: &mut Response) {
    let result = finish(req, depot).await;
    let Ok(console) = console(depot) else {
        super::failed(res, Error::Unavailable);
        return;
    };
    if result
        .as_ref()
        .is_err_and(|code| *code == "invalid_oauth_state")
    {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(json!({"code":"invalid_oauth_state"})));
        return;
    }
    match result {
        Ok((session, status)) => {
            *console.0.server_login.status.lock().await = status;
            res.add_header(
                "set-cookie",
                format!(
                    "{COOKIE}={session}; HttpOnly; SameSite=Strict; Path=/console; Max-Age=900"
                ),
                true,
            )
            .unwrap();
        }
        Err(code) => {
            *console.0.server_login.status.lock().await = json!({"state":"failed","code":code});
        }
    }
    res.add_header(
        "set-cookie",
        format!("{NONCE_COOKIE}=; HttpOnly; SameSite=Lax; Path=/console/server-login; Max-Age=0"),
        false,
    )
    .unwrap();
    res.status_code(StatusCode::SEE_OTHER);
    res.add_header("location", "/console/", true).unwrap();
}
async fn finish(req: &mut Request, depot: &Depot) -> Result<(String, Value), &'static str> {
    let console = console(depot).map_err(|_| "local_state_unavailable")?;
    let login = &console.0.server_login;
    let _transition = login.transition.lock().await;
    let state = req.query::<String>("state").ok_or("invalid_oauth_state")?;
    let mut lock = login.pending.lock().await;
    let pending = lock.as_ref().ok_or("invalid_oauth_state")?;
    if pending.expires <= Instant::now()
        || pending.state != state
        || cookie(req, NONCE_COOKIE).as_deref() != Some(pending.nonce.as_str())
    {
        return Err("invalid_oauth_state");
    }
    let mut pending = lock.take().unwrap();
    drop(lock);
    if req
        .query::<String>("iss")
        .is_some_and(|issuer| issuer != format!("{}auth/", pending.binding.origin))
    {
        return Err("owner_mismatch");
    }
    let code = req
        .query::<String>("code")
        .filter(|c| c.len() <= 4096)
        .ok_or("oauth_login_cancelled")?;
    let client = http()?;
    let server = origin(&pending.binding.origin)?;
    let tokens = response(
        client
            .post(server.join("/auth/oauth2/token").unwrap())
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", pending.binding.client_id.as_str()),
                ("redirect_uri", pending.redirect.as_str()),
                ("code", code.as_str()),
                ("code_verifier", pending.verifier.as_str()),
            ])
            .send()
            .await
            .map_err(|_| "server_unavailable")?,
    )
    .await?;
    let (token, refresh_token, oauth_lifetime) = oauth_credentials(&tokens)?;
    let mut issued_session = String::new();
    let completed = async {
        let (grant, grant_clock) = proof_response(
            Instant::now(),
            client
                .post(server.join("/hagency/api/v1/sessions/pasion").unwrap())
                .json(&json!({"accessToken":token}))
                .send()
                .await
                .map_err(|_| "server_unavailable")?,
        )
        .await?;
        let session_token = grant["token"]
            .as_str()
            .filter(|t| t.len() == 64)
            .ok_or("invalid_server_response")?;
        issued_session = session_token.to_owned();
        let verified = async {
            let (identity, identity_clock) = get_identity(&client, &server, session_token).await?;
            let authorized_until = grant_clock.deadline(&grant)?.min(identity_clock.deadline(&identity)?);
            identity["userId"]
                .as_str()
                .filter(|v| !v.is_empty() && v.len() <= 128)
                .ok_or("invalid_server_response")?;
            let user = identity["mxid"]
                .as_str()
                .filter(|v| {
                    v.len() <= 255
                        && v.starts_with('@')
                        && v[1..]
                            .split_once(':')
                            .is_some_and(|(local, host)| !local.is_empty() && !host.is_empty())
                        && !v.chars().any(|c| c.is_whitespace() || c.is_control())
                })
                .ok_or("invalid_server_response")?;
            let subject = identity["subject"]
                .as_str()
                .filter(|v| !v.is_empty() && v.len() <= 255)
                .ok_or("invalid_server_response")?;
            if identity["issuer"] != server.join("/auth/").unwrap().as_str()
                || grant["mxid"] != identity["mxid"]
                || grant["userId"] != identity["userId"]
                || identity["clientId"] != pending.binding.client_id
                || pending
                    .binding
                    .owner
                    .as_ref()
                    .is_some_and(|owner| owner != user)
                || pending
                    .binding
                    .subject
                    .as_ref()
                    .is_some_and(|owner| owner != subject)
            {
                return Err("owner_mismatch");
            }
            if pending.known_identity_only
                && !login.known_identity(server.as_str(), subject, user).await
            {
                return Err("owner_mismatch");
            }
            if let Some((expected_origin, expected_owner)) = login
                .native_expected
                .lock()
                .map_err(|_| "local_state_unavailable")?
                .as_ref()
                && (pending.binding.origin != *expected_origin || user != expected_owner.as_str())
            {
                return Err("matrix_account_mismatch");
            }
            Ok((
                identity.clone(),
                authorized_until,
                user.to_owned(),
                subject.to_owned(),
            ))
        }
        .await;
        let (identity, authorized_until, user, subject) = verified?;
        if pending.known_identity_only {
            console
                .0
                .authority
                .revoke_all()
                .map_err(|_| "local_state_unavailable")?;
            console.stop_owned_runtimes().await;
            console.stop_owner_provider().await;
        }
        pending.binding.owner = Some(user.to_owned());
        pending.binding.subject = Some(subject.to_owned());
        login.save(pending.binding.clone()).await?;
        let expiry = if refresh_token.is_some() {
            900
        } else {
            oauth_lifetime.as_secs().min(900)
        };
        if expiry == 0 {
            return Err("sign_in_required");
        }
        let session = console
            .0
            .authority
            .server_session(Duration::from_secs(expiry))
            .map_err(|_| "local_state_unavailable")?;
        let mut sessions = login.sessions.lock().await;
        // Replacing this installation's login stops prior capabilities even when
        // their UI cookie has already expired and can be removed from memory.
        for previous in sessions.values_mut() {
            login.queue_revocation(previous).await;
        }
        sessions.retain(|_, s| s.expires > Instant::now());
        if sessions.len() >= 64 {
            return Err("local_session_limit");
        }
        sessions.insert(
            URL_SAFE_NO_PAD.encode(Sha256::digest(session.as_bytes())),
            RemoteSession {
                matrix_source: None,
                token: session_token.to_owned(),
                oauth_token: token.clone(),
                user_id: identity["userId"].as_str().unwrap().into(),
                device: None,
                refresh_token: refresh_token.clone(),
                oauth_expires: Instant::now() + oauth_lifetime,
                authorized_until: authorized_until.min(Instant::now() + oauth_lifetime),
                authorized_until_ms: stated_end(&[&grant, &identity]),
                clock_allowance: std::time::Duration::ZERO,
                full_window: std::time::Duration::ZERO,
                sdk_refreshed: None,
                server_clock: ServerClock::Dated,
                invalidated: false,
                binding: pending.binding.clone(),
                checked: Instant::now(),
                expires: Instant::now() + Duration::from_secs(expiry),
            },
        );
        drop(sessions);
        let result = register_device(
            &client,
            &server,
            session_token,
            &pending.binding,
            identity["userId"].as_str().unwrap(),
        )
        .await;
        let (status, device_token) = match result {
            Ok((status, token)) => (status, Some(token)),
            Err(code) => (
                json!({"state":"signed_in","deviceAuthorized":false,"code":code}),
                None,
            ),
        };
        if let Some(remote) = login
            .sessions
            .lock()
            .await
            .get_mut(&URL_SAFE_NO_PAD.encode(Sha256::digest(session.as_bytes())))
        {
            remote.device = device_token;
        }
        ServerLogin::start_worker(console);
        Ok((session, status))
    }
    .await;
    if completed.is_err() {
        login
            .revoke_failed_login(
                &pending.binding,
                &issued_session,
                &token,
                refresh_token.as_deref(),
            )
            .await?;
    }
    completed
}
async fn register_device(
    client: &Client,
    server: &Url,
    token: &str,
    binding: &Binding,
    user_id: &str,
) -> Result<(Value, AuthorizedDevice), &'static str> {
    let (value, clock) = proof_response(
        Instant::now(),
        client
            .post(server.join("/hagency/api/v1/devices").unwrap())
            .bearer_auth(token)
            .json(&json!({"installationId":binding.installation_id,"name":binding.name}))
            .send()
            .await
            .map_err(|_| "server_unavailable")?,
    )
    .await?;
    let device = value["deviceId"]
        .as_str()
        .ok_or("invalid_server_response")?;
    let token = value["token"]
        .as_str()
        .filter(|t| t.len() == 64)
        .ok_or("invalid_server_response")?
        .to_owned();
    Ok((
        json!({"state":"device_authorized","deviceId":device,"generation":value["generation"],"deviceAuthorized":true,"transportOnline":false}),
        AuthorizedDevice {
            origin: binding.origin.clone(),
            device_id: device.to_owned(),
            generation: value["generation"]
                .as_u64()
                .filter(|v| *v > 0)
                .ok_or("invalid_server_response")?,
            token,
            owner_mxid: binding.owner.clone().ok_or("invalid_server_response")?,
            issuer: format!("{}auth/", binding.origin),
            subject: binding.subject.clone().ok_or("invalid_server_response")?,
            user_id: user_id.into(),
            valid_until: clock.deadline(&value)?,
            valid_until_ms: stated_end(&[&value]),
            clock_allowance: Duration::ZERO,
            revoked: std::sync::Arc::new(AtomicBool::new(false)),
            stopped: std::sync::Arc::new(AtomicBool::new(false)),
        },
    ))
}

/// A closed set of owner operations. No caller supplies a URL or bearer token.
pub(super) enum OwnerOperation {
    Management(super::native::management_server::ServerOperation),
    Allocation(super::native::allocation_transport::ServerOperation),
    ProjectSetup(super::native::project_setup::ServerOperation),
    AgentCommandStatus {
        operation: String,
        command: String,
    },
    Agents,
    RoomAgents,
    Devices,
    Agent {
        agent: String,
    },
    AssignExecutionDevice {
        agent: String,
        expected: i64,
    },
    AgentOwnerDirect {
        agent: String,
    },
    OwnerDirect {
        agent: String,
        room: String,
    },
    Projects,
    ScopeState {
        project: String,
        room: Option<String>,
    },
    ScopePause {
        project: String,
        room: Option<String>,
        paused: bool,
    },
    ProjectPolicy {
        project: String,
        expected: i64,
        policy: super::native::ProjectCreationPolicy,
    },
    RoomPolicy {
        project: String,
        room: String,
        expected: i64,
        policy: super::native::RoomCreationPolicy,
    },
    AdoptProject {
        space: String,
        project_folder: Option<String>,
    },
    ProjectRooms {
        project: String,
    },
    AdoptRoom {
        project: String,
        room: String,
    },
    RoomRoster {
        project: String,
        room: String,
    },
    Bindings {
        agent: String,
    },
    /// Lets the owner's Agent serve project Rooms.
    MakePublic {
        agent: String,
    },
    Binding {
        binding: String,
    },
    SetThreadAutoReply {
        binding: String,
        enabled: bool,
    },
    PauseBinding {
        binding: String,
    },
    ResumeBinding {
        binding: String,
    },
    LeaveBinding {
        binding: String,
    },
    Create {
        name: String,
        command: String,
    },
    Bind {
        agent: String,
        project: String,
        room: String,
        command: String,
    },
    Pause {
        agent: String,
    },
    Resume {
        agent: String,
    },
    Retire {
        agent: String,
    },
}
pub(super) struct OwnerReply {
    pub owner: String,
    pub origin: String,
    pub issuer: String,
    pub subject: String,
    pub value: Value,
}
#[derive(Debug)]
pub(super) struct OwnerError {
    pub status: u16,
    pub code: String,
}
impl OwnerError {
    fn new(status: u16, code: &str) -> Self {
        Self {
            status,
            code: code.into(),
        }
    }
}
fn operation_id(value: &str) -> Result<(), OwnerError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Err(OwnerError::new(400, "invalid_arguments"))
    } else {
        Ok(())
    }
}
fn policy_scope(project: &str, room: Option<&str>) -> Result<String, OwnerError> {
    operation_id(project)?;
    match room {
        Some(room) => {
            matrix_creations::room_id(room)?;
            if room.contains('%') || !room.contains(':') {
                return Err(OwnerError::new(400, "invalid_room_id"));
            }
            Ok(format!(
                "projects/{project}/rooms/{}",
                percent_encoding::utf8_percent_encode(room, percent_encoding::NON_ALPHANUMERIC)
            ))
        }
        None => Ok(format!("projects/{project}")),
    }
}
impl OwnerOperation {
    fn request(self) -> Result<(reqwest::Method, String, Option<Value>), OwnerError> {
        use reqwest::Method;
        if let Self::Management(operation) = self {
            return operation.request();
        }
        if let Self::Allocation(operation) = self {
            return operation.request();
        }
        if let Self::ProjectSetup(operation) = self {
            return operation.request();
        }
        let binding_request = match &self {
            Self::Binding { binding } => Some((Method::GET, binding, "")),
            Self::PauseBinding { binding } => Some((Method::POST, binding, "/pause")),
            Self::ResumeBinding { binding } => Some((Method::POST, binding, "/resume")),
            Self::LeaveBinding { binding } => Some((Method::DELETE, binding, "")),
            _ => None,
        };
        if let Some((method, binding, suffix)) = binding_request {
            operation_id(binding)?;
            return Ok((
                method,
                format!("/hagency/api/v1/bindings/{binding}{suffix}"),
                None,
            ));
        }
        let (method, suffix, body) = match self {
            Self::Management(_) => unreachable!("management handled above"),
            Self::Allocation(_) => unreachable!("allocation handled above"),
            Self::ProjectSetup(_) => unreachable!("project setup handled above"),
            Self::SetThreadAutoReply { binding, enabled } => {
                operation_id(&binding)?;
                (
                    Method::PUT,
                    format!("bindings/{binding}/reply-policy"),
                    Some(json!({"threadAutoReply":enabled})),
                )
            }
            Self::AgentCommandStatus { operation, command } => {
                if !matches!(operation.as_str(), "agent.create" | "agent.bind") {
                    return Err(OwnerError::new(400, "invalid_arguments"));
                }
                operation_id(&command)?;
                (Method::GET, format!("commands/{operation}/{command}"), None)
            }
            Self::Agents => (Method::GET, "agents".into(), None),
            Self::RoomAgents => (Method::GET, "room-agents".into(), None),
            Self::Projects => (Method::GET, "projects".into(), None),
            Self::ScopeState { project, room } => (
                Method::GET,
                format!("{}/service-state", policy_scope(&project, room.as_deref())?),
                None,
            ),
            Self::ScopePause {
                project,
                room,
                paused,
            } => (
                Method::POST,
                format!(
                    "{}/{}",
                    policy_scope(&project, room.as_deref())?,
                    if paused {
                        "pause-service"
                    } else {
                        "clear-service-pause"
                    }
                ),
                None,
            ),
            Self::ProjectPolicy {
                project,
                expected,
                policy,
            } => (
                Method::PUT,
                format!("{}/creation-policy", policy_scope(&project, None)?),
                Some(json!({"expectedRevision":expected,"policy":policy})),
            ),
            Self::RoomPolicy {
                project,
                room,
                expected,
                policy,
            } => (
                Method::PUT,
                format!("{}/creation-policy", policy_scope(&project, Some(&room))?),
                Some(json!({"expectedRevision":expected,"policy":policy})),
            ),
            Self::AdoptProject {
                space,
                project_folder,
            } => {
                matrix_creations::room_id(&space)?;
                let mut body = json!({"spaceId":space});
                // Sent only when chosen, so a server without shared project
                // folders adopts every other space as before.
                if let Some(folder) = project_folder {
                    if !super::native::valid_project_folder_name(&folder) {
                        return Err(OwnerError::new(400, "invalid_project_folder"));
                    }
                    body["projectFolder"] = json!(folder);
                }
                (Method::POST, "projects/adopt".into(), Some(body))
            }
            Self::ProjectRooms { project } => {
                operation_id(&project)?;
                (Method::GET, format!("projects/{project}/rooms"), None)
            }
            Self::AdoptRoom { project, room } => {
                operation_id(&project)?;
                matrix_creations::room_id(&room)?;
                (
                    Method::POST,
                    format!("projects/{project}/rooms/adopt"),
                    Some(json!({"roomId":room})),
                )
            }
            Self::RoomRoster { project, room } => {
                operation_id(&project)?;
                let room = matrix_creations::segment(&room)?;
                (
                    Method::GET,
                    format!("projects/{project}/rooms/{room}/agents"),
                    None,
                )
            }
            Self::Devices => (Method::GET, "devices".into(), None),
            Self::Agent { agent } => {
                operation_id(&agent)?;
                (Method::GET, format!("agents/{agent}"), None)
            }
            Self::AssignExecutionDevice { agent, expected } => {
                operation_id(&agent)?;
                if expected < 0 {
                    return Err(OwnerError::new(400, "invalid_arguments"));
                }
                (
                    Method::PUT,
                    format!("agents/{agent}/execution-device"),
                    Some(json!({"expectedGeneration":expected})),
                )
            }
            Self::AgentOwnerDirect { agent } => {
                operation_id(&agent)?;
                (Method::GET, format!("agents/{agent}/owner-direct"), None)
            }
            Self::OwnerDirect { agent, room } => {
                operation_id(&agent)?;
                matrix_creations::room_id(&room)?;
                (
                    Method::POST,
                    format!("agents/{agent}/owner-direct"),
                    Some(json!({"roomId":room})),
                )
            }
            Self::Bindings { agent } => {
                operation_id(&agent)?;
                (Method::GET, format!("agents/{agent}/bindings"), None)
            }
            Self::MakePublic { agent } => {
                operation_id(&agent)?;
                (
                    Method::PUT,
                    format!("agents/{agent}/visibility"),
                    Some(json!({"visibility":"public"})),
                )
            }
            Self::Binding { .. }
            | Self::PauseBinding { .. }
            | Self::ResumeBinding { .. }
            | Self::LeaveBinding { .. } => unreachable!("binding operation handled before match"),
            Self::Create { name, command } => {
                operation_id(&command)?;
                (
                    Method::POST,
                    "agents".into(),
                    Some(json!({"displayName":name,"idempotencyKey":command})),
                )
            }
            Self::Bind {
                agent,
                project,
                room,
                command,
            } => {
                operation_id(&agent)?;
                operation_id(&project)?;
                operation_id(&command)?;
                (
                    Method::POST,
                    format!("agents/{agent}/bindings"),
                    Some(json!({"projectId":project,"roomId":room,"idempotencyKey":command})),
                )
            }
            Self::Pause { agent } => {
                operation_id(&agent)?;
                (Method::POST, format!("agents/{agent}/pause"), None)
            }
            Self::Resume { agent } => {
                operation_id(&agent)?;
                (Method::POST, format!("agents/{agent}/resume"), None)
            }
            Self::Retire { agent } => {
                operation_id(&agent)?;
                (Method::DELETE, format!("agents/{agent}"), None)
            }
        };
        Ok((method, format!("/hagency/api/v1/{suffix}"), body))
    }
}
impl ServerLogin {
    pub(super) fn state_directory(&self) -> Option<PathBuf> {
        self.path.as_ref()?.parent().map(Path::to_path_buf)
    }
    pub(super) async fn owner_api(
        &self,
        cookie: &str,
        operation: OwnerOperation,
    ) -> Result<OwnerReply, OwnerError> {
        self.owner_api_inner(Some(cookie), None, operation, None)
            .await
    }
    async fn owner_api_inner(
        &self,
        cookie: Option<&str>,
        pin: Option<&super::device_execution::DeviceIdentity>,
        operation: OwnerOperation,
        creation_scope: Option<&matrix_creations::Scope<'_>>,
    ) -> Result<OwnerReply, OwnerError> {
        let needs_device = matches!(
            &operation,
            OwnerOperation::Create { .. }
                | OwnerOperation::AssignExecutionDevice { .. }
                | OwnerOperation::Management(..)
                | OwnerOperation::Allocation(..)
                | OwnerOperation::ProjectSetup(..)
        );
        let native_allocation = matches!(&operation, OwnerOperation::Allocation(_));
        let (method, path, body) = operation.request()?;
        let shared_protocol = native_allocation
            && path.starts_with("/hagency/api/v1/management/shared-execution/");
        if self.stopped.load(Ordering::Acquire) {
            return Err(OwnerError::new(401, "sign_in_required"));
        }
        let renewal = self.renewal.lock().await;
        let key = {
            let mut sessions = self.sessions.lock().await;
            let key = if let Some(cookie) = cookie {
                session_key(cookie)
            } else {
                sessions
                    .iter()
                    .find(|(_, s)| {
                        !s.invalidated
                            && s.device.as_ref().is_some_and(|d| {
                                pin == Some(&super::device_execution::DeviceIdentity::from(d))
                            })
                    })
                    .map(|(key, _)| key.clone())
                    .ok_or_else(|| OwnerError::new(401, "sign_in_required"))?
            };
            // Held to its end by the wall clock first, a proof a sleep ran
            // out is renewed below, not kept and then refused here.
            if let Some(session) = sessions.get_mut(&key) {
                session.follow_wall_clock();
            }
            key
        };
        self.renew_or_keep(&key).await.map_err(|e| {
            OwnerError::new(
                if matches!(e, Error::Unavailable) {
                    503
                } else {
                    401
                },
                if matches!(e, Error::Unavailable) {
                    "server_unavailable"
                } else {
                    "sign_in_required"
                },
            )
        })?;
        let session = self
            .sessions
            .lock()
            .await
            .get(&key)
            .cloned()
            .filter(|s| !s.invalidated && s.expires > Instant::now())
            .ok_or_else(|| OwnerError::new(401, "sign_in_required"))?;
        drop(renewal);
        if let Some(scope) = creation_scope {
            scope.matches(&session)?;
            scope.check().await?;
        }
        if pin.is_some_and(|pin| {
            session
                .device
                .as_ref()
                .is_none_or(|device| pin != &super::device_execution::DeviceIdentity::from(device))
        }) {
            return Err(OwnerError::new(401, "device_authorization_required"));
        }
        let server = origin(&session.binding.origin)
            .map_err(|_| OwnerError::new(401, "sign_in_required"))?;
        let client = http().map_err(|_| OwnerError::new(503, "server_unavailable"))?;
        let mut device = session.device.clone();
        let bearer = if needs_device {
            let device = device
                .as_mut()
                .ok_or_else(|| OwnerError::new(401, "device_authorization_required"))?;
            device.valid_until = session.authorized_until;
            device.valid_until_ms = session.authorized_until_ms;
            device.clock_allowance = session.clock_allowance;
            device
                .bearer()
                .map_err(|_| OwnerError::new(401, "device_authorization_required"))?
        } else {
            &session.token
        };
        // What the log names a failed request by: its route without the query.
        let route = format!("{method} {}", path.split('?').next().unwrap_or_default());
        let mut request = client
            .request(method, server.join(&path).unwrap())
            .bearer_auth(bearer);
        if shared_protocol {
            request = request.header("x-hagency-shared-protocol", "native-context-v3;claude-v4");
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        // A failure is logged where it is made, with who made it: this host,
        // when the request could not be sent or its answer not be read, or
        // the server, when it answered with a refusal.
        let mut reply = request.send().await.map_err(|_| {
            tracing::info!(origin = "host", stage = "send", %route, code = "server_unavailable", "Owner request got no answer");
            OwnerError::new(503, "server_unavailable")
        })?;
        let status = reply.status().as_u16();
        let mut raw = Vec::new();
        while let Some(chunk) = reply.chunk().await.map_err(|_| {
            tracing::info!(origin = "host", stage = "read", %route, status, code = "server_unavailable", "Owner request got no answer");
            OwnerError::new(503, "server_unavailable")
        })? {
            if raw.len() + chunk.len() > 128 * 1024 {
                return Err(OwnerError::new(502, "invalid_server_response"));
            }
            raw.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&raw)
            .map_err(|_| OwnerError::new(502, "invalid_server_response"))?;
        if !(200..300).contains(&status) {
            let code = value["code"]
                .as_str()
                .filter(|c| {
                    !c.is_empty()
                        && c.len() <= 80
                        && c.bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                })
                .unwrap_or("server_request_failed");
            if status >= 500 {
                tracing::info!(origin = "server", %route, status, code, "Owner request was not served");
            } else {
                tracing::debug!(origin = "server", %route, status, code, "Owner request was refused");
            }
            return Err(OwnerError::new(status, code));
        }
        if let Some(scope) = creation_scope {
            scope.check().await?;
        }
        let sessions = self.sessions.lock().await;
        // Cookie renewal can move this same grant while management HTTP is in
        // flight. Validate the grant itself; logout still invalidates/removes it.
        let current = sessions.values().find(|s| s.token == session.token);
        if self.stopped.load(Ordering::Acquire)
            || current.is_none_or(|s| s.invalidated || s.expires <= Instant::now())
        {
            return Err(OwnerError::new(401, "sign_in_required"));
        }
        if current.is_none_or(|s| s.authorized_until <= Instant::now()) {
            return Err(OwnerError::new(503, "server_unavailable"));
        }
        Ok(OwnerReply {
            owner: session
                .binding
                .owner
                .clone()
                .ok_or_else(|| OwnerError::new(401, "sign_in_required"))?,
            origin: session.binding.origin.clone(),
            issuer: format!("{}auth/", session.binding.origin),
            subject: session
                .binding
                .subject
                .clone()
                .ok_or_else(|| OwnerError::new(401, "sign_in_required"))?,
            value,
        })
    }
}

impl super::Console {
    /// Background service discovery uses the current authenticated session pinned
    /// to the exact owner/device generation; no browser cookie is retained.
    pub(super) async fn owner_api_pinned(
        &self,
        pin: &super::device_execution::DeviceIdentity,
        operation: OwnerOperation,
    ) -> Result<OwnerReply, OwnerError> {
        let current = self.authorized_device().await.map_err(|e| {
            OwnerError::new(
                if matches!(e, Error::Unavailable) {
                    503
                } else {
                    401
                },
                if matches!(e, Error::Unavailable) {
                    "server_unavailable"
                } else {
                    "device_authorization_required"
                },
            )
        })?;
        current
            .bearer()
            .map_err(|_| OwnerError::new(401, "device_authorization_required"))?;
        if pin != &super::device_execution::DeviceIdentity::from(&current) {
            return Err(OwnerError::new(401, "device_authorization_required"));
        }
        self.0
            .server_login
            .owner_api_inner(None, Some(pin), operation, None)
            .await
    }
    pub(super) async fn owner_api(
        &self,
        cookie: &str,
        operation: OwnerOperation,
    ) -> Result<OwnerReply, OwnerError> {
        self.0.server_login.owner_api(cookie, operation).await
    }
}

#[cfg(test)]
mod private_json_tests {
    use super::*;
    #[test]
    fn restart_reads_bounded_json_larger_than_a_single_token_without_restoring_authority() {
        let root = hagency_platform::legacy_writer::test_support::initialized_tempdir().unwrap();
        let binding = Binding {
            origin: "http://127.0.0.1:13377/".into(),
            client_id: "c".repeat(128),
            installation_id: "i".repeat(64),
            name: "n".repeat(128),
            owner: Some(format!("@{}:example.test", "o".repeat(180))),
            subject: Some("s".repeat(200)),
        };
        let body = serde_json::to_vec(&binding).unwrap();
        assert!(body.len() > 512);
        hagency_store::private::replace(&root.path().join("server-login.json"), &body).unwrap();
        let mut records = vec![Revocation {
            origin: binding.origin.clone(),
            client_id: binding.client_id.clone(),
            session_token: "d".repeat(64),
            oauth_token: "t".repeat(1000),
            hint: "refresh_token".into(),
            session_done: false,
            oauth_done: false,
        }];
        records.push(Revocation {
            origin: binding.origin.clone(),
            client_id: binding.client_id.clone(),
            session_token: String::new(),
            oauth_token: "minted-before-grant".into(),
            hint: "access_token".into(),
            session_done: true,
            oauth_done: false,
        });
        hagency_store::private::replace(
            &root.path().join("server-login-revocations.json"),
            &serde_json::to_vec(&records).unwrap(),
        )
        .unwrap();
        let login = ServerLogin::new(Some(root.path())).unwrap();
        assert!(login.sessions.try_lock().unwrap().is_empty());
        assert_eq!(login.revocations.try_lock().unwrap().len(), 2);
        assert!(read_private_json(&root.path().join("server-login.json"), 512).is_err());
    }
}

#[cfg(test)]
mod scope_policy_request_tests {
    use super::*;
    #[test]
    fn reply_policy_uses_owner_server_binding_path() {
        for enabled in [false, true] {
            let (method, path, body) = OwnerOperation::SetThreadAutoReply {
                binding: "bnd_1".into(),
                enabled,
            }
            .request()
            .unwrap();
            assert_eq!(method, reqwest::Method::PUT);
            assert_eq!(path, "/hagency/api/v1/bindings/bnd_1/reply-policy");
            assert_eq!(body, Some(json!({"threadAutoReply":enabled})));
        }
    }
    #[test]
    fn scope_requests_are_fixed_origin_paths_without_claimed_administrator() {
        let (method, path, body) = OwnerOperation::ScopePause {
            project: "prj_1".into(),
            room: Some("!r:test".into()),
            paused: true,
        }
        .request()
        .unwrap();
        assert_eq!(method, reqwest::Method::POST);
        assert_eq!(
            path,
            "/hagency/api/v1/projects/prj_1/rooms/%21r%3Atest/pause-service"
        );
        assert!(body.is_none());
        assert!(
            OwnerOperation::ScopeState {
                project: "../other".into(),
                room: None
            }
            .request()
            .is_err()
        );
        assert!(
            OwnerOperation::ScopeState {
                project: "prj_1".into(),
                room: Some("!r:test/admin".into())
            }
            .request()
            .is_err()
        );
        let (_, _, body) = OwnerOperation::RoomPolicy {
            project: "prj_1".into(),
            room: "!r:test".into(),
            expected: 9,
            policy: super::super::native::RoomCreationPolicy::Disabled,
        }
        .request()
        .unwrap();
        assert_eq!(
            body.unwrap(),
            json!({"expectedRevision":9,"policy":{"mode":"disabled"}})
        );
    }
}

#[cfg(test)]
mod shared_protocol_tests;

#[cfg(test)]
mod authorization_window_tests;

#[cfg(test)]
mod shortened_proof_tests {
    use super::*;
    #[test]
    fn a_proof_counts_as_shortened_once_it_lost_ten_seconds_of_the_full_window() {
        let secs = Duration::from_secs;
        // A two-minute window, and a token closer than that to its expiry.
        assert!(!proof_shortened(secs(120), secs(120), None));
        assert!(!proof_shortened(secs(111), secs(120), None));
        assert!(proof_shortened(secs(110), secs(120), None));
        assert!(proof_shortened(secs(60), secs(120), Some(secs(60))));
        // Not within a minute of the last refresh.
        assert!(!proof_shortened(secs(60), secs(120), Some(secs(59))));
        // Nothing is known before the first renewal.
        assert!(!proof_shortened(secs(25), Duration::ZERO, None));
        // The default window: only where the proof counts as expiring anyway.
        assert!(!proof_shortened(secs(21), secs(30), None));
        assert!(proof_shortened(EXPIRING_PROOF, secs(30), None));
    }
}

#[cfg(test)]
mod oauth_refresh_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn oauth_http_refresh_failures_preserve_credentials_and_allow_a_valid_retry() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}/", listener.local_addr().unwrap());
        let now = Instant::now();
        let mut session = RemoteSession {
            matrix_source: None,
            token: "owner-session".into(),
            oauth_token: "old-access".into(),
            user_id: "owner".into(),
            device: None,
            refresh_token: Some("retained-refresh".into()),
            oauth_expires: now,
            authorized_until: now,
            authorized_until_ms: u64::MAX,
            clock_allowance: std::time::Duration::ZERO,
            full_window: std::time::Duration::ZERO,
            sdk_refreshed: None,
            server_clock: ServerClock::Dated,
            invalidated: false,
            binding: Binding {
                origin: origin.clone(),
                client_id: "client".into(),
                installation_id: "installation".into(),
                name: "fixture".into(),
                owner: Some("@owner:test".into()),
                subject: Some("subject".into()),
            },
            checked: now,
            expires: now + Duration::from_secs(900),
        };
        let cases = [
            (503, json!({"error":"server_error"}), false),
            (429, json!({"error":"slow_down"}), false),
            (400, json!({"error":"invalid_grant"}), true),
            (401, json!({"error":"invalid_client"}), true),
            (200, json!({}), false),
        ];
        let mut replies: Vec<_> = cases
            .iter()
            .map(|(status, body, _)| (*status, body.clone()))
            .collect();
        let until = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 30_000) as u64;
        replies.extend([(200, json!({"access_token":"new-access","refresh_token":"new-refresh","expires_in":900})),
            (200, json!({"validUntilMs":until})),
            (200, json!({"mxid":"@owner:test","subject":"subject","clientId":"client","issuer":format!("{origin}auth/"),"userId":"owner","validUntilMs":until}))]);
        let server = tokio::spawn(async move {
            for (index, (status, body)) in replies.into_iter().enumerate() {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut raw = Vec::new();
                loop {
                    let mut chunk = [0u8; 4096];
                    let size = socket.read(&mut chunk).await.unwrap();
                    assert!(size > 0);
                    raw.extend_from_slice(&chunk[..size]);
                    let request = String::from_utf8_lossy(&raw);
                    if let Some((headers, body)) = request.split_once("\r\n\r\n") {
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if body.len() >= length {
                            break;
                        }
                    }
                }
                let request = String::from_utf8(raw).unwrap();
                if index <= 5 {
                    assert!(request.starts_with("POST /auth/oauth2/token "));
                    assert!(request.contains("refresh_token=retained-refresh"));
                }
                let body = body.to_string();
                socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        for (index, (_, _, terminal)) in cases.into_iter().enumerate() {
            let result = if index == 0 {
                // Standalone OAuth must reach renewal after the old proof
                // expires, rather than turning a temporary outage into logout.
                let root =
                    hagency_platform::legacy_writer::test_support::initialized_tempdir().unwrap();
                let login = ServerLogin::new(Some(root.path())).unwrap();
                login
                    .sessions
                    .lock()
                    .await
                    .insert("expired-proof".into(), session.clone());
                let result = login.authorized_device().await.map(|_| ());
                session = login
                    .sessions
                    .lock()
                    .await
                    .get("expired-proof")
                    .unwrap()
                    .clone();
                result
            } else {
                renew(&mut session).await
            };
            assert!(if terminal {
                matches!(result, Err(Error::Unauthorized))
            } else {
                matches!(result, Err(Error::Unavailable))
            });
            assert_eq!(session.oauth_token, "old-access");
            assert_eq!(session.refresh_token.as_deref(), Some("retained-refresh"));
            assert_eq!(session.authorized_until, now);
            assert!(!session.invalidated);
        }
        renew(&mut session).await.unwrap();
        assert_eq!(session.oauth_token, "new-access");
        assert_eq!(session.refresh_token.as_deref(), Some("new-refresh"));
        assert!(session.authorized_until > now);
        server.await.unwrap();
    }
}

#[cfg(test)]
mod renewal_reading_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn session_at(origin: &str, until: Instant, until_ms: u64) -> RemoteSession {
        let now = Instant::now();
        RemoteSession {
            matrix_source: None,
            token: "owner-session".into(),
            oauth_token: "access".into(),
            user_id: "owner".into(),
            device: None,
            refresh_token: None,
            oauth_expires: now + Duration::from_secs(900),
            authorized_until: until,
            authorized_until_ms: until_ms,
            clock_allowance: Duration::ZERO,
            full_window: Duration::ZERO,
            sdk_refreshed: None,
            server_clock: ServerClock::Dated,
            invalidated: false,
            binding: Binding {
                origin: origin.into(),
                client_id: "client".into(),
                installation_id: "installation".into(),
                name: "fixture".into(),
                owner: Some("@owner:test".into()),
                subject: Some("subject".into()),
            },
            checked: now,
            expires: now + Duration::from_secs(900),
        }
    }
    async fn bind() -> (tokio::net::TcpListener, String) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}/", listener.local_addr().unwrap());
        (listener, origin)
    }
    /// Answers each request with the next of `replies`, and gives back the
    /// request lines.
    fn serve(
        listener: tokio::net::TcpListener,
        replies: Vec<(u16, Value)>,
    ) -> tokio::task::JoinHandle<Vec<String>> {
        let served = serve_dated(
            listener,
            replies.into_iter().map(|(status, body)| (status, body, None)).collect(),
        );
        tokio::spawn(async move { served.await.unwrap().into_iter().map(|(line, _)| line).collect() })
    }
    /// As `serve`, with the server's date on each reply that gives one, as a
    /// fresh answer carries it; gives back each request line with the window
    /// the request asked for.
    fn serve_dated(
        listener: tokio::net::TcpListener,
        replies: Vec<(u16, Value, Option<u64>)>,
    ) -> tokio::task::JoinHandle<Vec<(String, Option<String>)>> {
        tokio::spawn(async move {
            let mut lines = Vec::new();
            for (status, body, date_ms) in replies {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut raw = Vec::new();
                loop {
                    let mut chunk = [0u8; 4096];
                    let size = socket.read(&mut chunk).await.unwrap();
                    assert!(size > 0);
                    raw.extend_from_slice(&chunk[..size]);
                    let request = String::from_utf8_lossy(&raw);
                    if let Some((headers, body)) = request.split_once("\r\n\r\n") {
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if body.len() >= length {
                            break;
                        }
                    }
                }
                let request = String::from_utf8(raw).unwrap();
                let window = request.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case(AUTHORIZATION_WINDOW_HEADER)
                        .then(|| value.trim().to_owned())
                });
                lines.push((request.lines().next().unwrap().to_owned(), window));
                let date = date_ms.map_or(String::new(), |ms| {
                    let date = httpdate::fmt_http_date(UNIX_EPOCH + Duration::from_millis(ms));
                    format!("Date: {date}\r\nCache-Control: no-store\r\n")
                });
                let body = body.to_string();
                socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\n{date}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
            lines
        })
    }
    const RENEW: &str = "POST /hagency/api/v1/sessions/current/renew HTTP/1.1";
    const IDENTITY: &str = "GET /hagency/api/v1/identity HTTP/1.1";

    #[test]
    fn a_renewal_failure_is_read_by_said_and_only_a_revocation_ends_the_session() {
        let read = |status: u16, code: &str| renewal_refused(renewal_failure(status, code));
        // A refusal of the credential, and the codes that say the session is
        // gone, whatever their status.
        for (status, code) in [
            (401, "authorization_expired"),
            (401, "authentication_required"),
            (403, "a_code_of_the_server"),
            (409, "identity_mapping_mismatch"),
            (400, "sign_in_required"),
        ] {
            assert!(matches!(read(status, code), Error::Unauthorized), "{status} {code}");
        }
        // Decided, and about the request: the proof in hand is kept to its end.
        for (status, code) in [
            (400, "invalid_arguments"),
            (400, "invalid_identity_configuration"),
            (404, "not_found"),
            (409, "a_code_of_the_server"),
        ] {
            assert!(matches!(read(status, code), Error::Unavailable), "{status} {code}");
        }
        // Not decided: no answer, the server not serving it, or an answer
        // with no code of the server's in it, whatever its status.
        for (status, code) in [
            (401, "server_request_failed"),
            (403, "server_request_failed"),
            (502, "invalid_server_response"),
            (503, "server_unavailable"),
            (503, "pasion_unavailable"),
            (503, "management_capacity"),
        ] {
            assert!(matches!(read(status, code), Error::Unavailable), "{status} {code}");
        }
    }

    #[tokio::test]
    async fn a_renewal_refused_with_no_code_of_the_server_signs_no_one_out() {
        let (listener, origin) = bind().await;
        let server = serve(listener, vec![(401, json!({}))]);
        let end = Instant::now() + Duration::from_secs(20);
        let mut session = session_at(&origin, end, wall_now_ms() + 20_000);
        session.checked = Instant::now() - Duration::from_secs(30);
        assert!(matches!(renew(&mut session).await, Err(Error::Unavailable)));
        assert_eq!(session.authorized_until, end);
        assert!(!session.invalidated);
        assert_eq!(server.await.unwrap(), [RENEW]);
        // The server's own refusal of the credential still ends it.
        let (listener, origin) = bind().await;
        let server = serve(listener, vec![(401, json!({"code":"authorization_expired"}))]);
        let mut session = session_at(&origin, end, wall_now_ms() + 20_000);
        session.checked = Instant::now() - Duration::from_secs(30);
        assert!(matches!(renew(&mut session).await, Err(Error::Unauthorized)));
        assert_eq!(server.await.unwrap(), [RENEW]);
    }

    #[test]
    fn a_proof_ends_at_the_earlier_of_its_monotonic_end_and_its_end_by_the_wall_clock() {
        let now_ms = wall_now_ms();
        let (device, _) = fixture_device("https://example.test/", "subject", "@owner:example.test");
        assert!(device.bearer().is_ok());
        // After a sleep past the end the server stated, the monotonic end
        // still lies ahead: the wall clock ends the proof.
        let mut slept = device.clone();
        slept.valid_until_ms = now_ms - 1;
        assert!(slept.valid_until() > Instant::now());
        assert!(slept.bearer().is_err());
        let mut awake = device;
        awake.valid_until_ms = now_ms + 60_000;
        assert!(awake.bearer().is_ok());
        // The session is held to it the same way.
        let mut session = session_at("https://example.test/", Instant::now() + Duration::from_secs(60), now_ms - 1);
        session.follow_wall_clock();
        assert!(session.authorized_until <= Instant::now());
        let mut session = session_at("https://example.test/", Instant::now() + Duration::from_secs(60), now_ms + 10_000);
        session.follow_wall_clock();
        assert!(session.authorized_until <= Instant::now() + Duration::from_secs(10));
        assert!(session.authorized_until > Instant::now() + Duration::from_secs(8));
        // Within a second of each other, the two ends are one reading rounded
        // two ways: the proof is left as it is.
        let end = Instant::now() + Duration::from_secs(10);
        let mut session = session_at("https://example.test/", end, wall_now_ms() + 9_500);
        session.follow_wall_clock();
        assert_eq!(session.authorized_until, end);
        // It only ever shortens a proof; one with no stated end keeps its own.
        for wall in [now_ms + 600_000, u64::MAX] {
            let end = Instant::now() + Duration::from_secs(60);
            let mut session = session_at("https://example.test/", end, wall);
            session.follow_wall_clock();
            assert_eq!(session.authorized_until, end);
        }
    }

    #[tokio::test]
    async fn a_proof_a_sleep_ran_out_is_renewed_at_once_and_held_to_the_new_end() {
        let (listener, origin) = bind().await;
        let until = wall_now_ms() + 30_000;
        let server = serve(listener, vec![
            (200, json!({"validUntilMs": until})),
            (200, json!({"mxid":"@owner:test","subject":"subject","clientId":"client","issuer":format!("{origin}auth/"),"userId":"owner","validUntilMs": until - 5_000})),
        ]);
        // Checked just now and a minute ahead on the monotonic clock, as
        // after a sleep: read on that clock alone it is used as it is.
        let mut session = session_at(&origin, Instant::now() + Duration::from_secs(60), wall_now_ms() - 1);
        renew(&mut session).await.unwrap();
        assert_eq!(server.await.unwrap(), [RENEW, IDENTITY]);
        assert_eq!(session.authorized_until_ms, until - 5_000);
        assert!(session.authorized_until > Instant::now() + Duration::from_secs(20));
        assert!(session.authorized_until <= Instant::now() + Duration::from_secs(25));
    }

    #[tokio::test]
    async fn a_renewal_answered_with_a_shorter_proof_holds_the_wall_end_to_it() {
        let (listener, origin) = bind().await;
        let shorter = wall_now_ms() + 10_000;
        let server = serve(listener, vec![
            (200, json!({"validUntilMs": shorter})),
            (503, json!({"code":"storage_unavailable"})),
        ]);
        let mut session = session_at(&origin, Instant::now() + Duration::from_secs(20), wall_now_ms() + 20_000);
        session.checked = Instant::now() - Duration::from_secs(30);
        assert!(matches!(renew(&mut session).await, Err(Error::Unavailable)));
        assert_eq!(server.await.unwrap(), [RENEW, IDENTITY]);
        // The server's session now ends with the shorter proof, on both clocks.
        assert_eq!(session.authorized_until_ms, shorter);
        assert!(session.authorized_until <= Instant::now() + Duration::from_secs(10));
    }

    #[tokio::test]
    async fn an_undated_proof_is_held_to_the_default_window_and_no_more_is_asked_for_until_a_dated_answer() {
        let (listener, origin) = bind().await;
        let identity = |until: u64| json!({"mxid":"@owner:test","subject":"subject","clientId":"client","issuer":format!("{origin}auth/"),"userId":"owner","validUntilMs": until});
        // An answer that lost the server's date on the way grants the 120 s
        // asked for; the next ones carry the date, with the default window.
        let long = wall_now_ms() + 120_000;
        let date = wall_now_ms();
        let server = serve_dated(listener, vec![
            (200, json!({"validUntilMs": long}), None),
            (200, identity(long), None),
            (200, json!({"validUntilMs": date + 30_000}), Some(date)),
            (200, identity(date + 30_000), Some(date)),
            (503, json!({"code":"server_unavailable"}), None),
        ]);
        let mut session = session_at(&origin, Instant::now() + Duration::from_secs(20), wall_now_ms() + 20_000);
        session.checked = Instant::now() - Duration::from_secs(30);
        let sent = Instant::now();
        renew(&mut session).await.unwrap();
        // Used, and held to the default window from the moment its request
        // went out, rather than refused and asked for again every second.
        assert!(session.authorized_until >= sent + Duration::from_secs(30));
        assert!(session.authorized_until <= Instant::now() + Duration::from_millis(30_001));
        assert_eq!(session.server_clock, ServerClock::Undated);
        // The next renewal asks for the default window only; after a dated
        // answer the one after it asks for the longer window again.
        session.checked = Instant::now() - Duration::from_secs(30);
        renew(&mut session).await.unwrap();
        assert_eq!(session.server_clock, ServerClock::Dated);
        session.checked = Instant::now() - Duration::from_secs(30);
        assert!(matches!(renew(&mut session).await, Err(Error::Unavailable)));
        let asked = |window: &str| Some(window.to_owned());
        assert_eq!(server.await.unwrap(), [
            (RENEW.to_owned(), asked("120000")),
            (IDENTITY.to_owned(), None),
            (RENEW.to_owned(), asked("30000")),
            (IDENTITY.to_owned(), None),
            (RENEW.to_owned(), asked("120000")),
        ]);
    }

    #[tokio::test]
    async fn a_clock_ahead_of_the_server_by_the_whole_window_is_decided_named_and_not_asked_again_at_once() {
        let (listener, origin) = bind().await;
        // By the server's date this computer's clock is forty seconds ahead,
        // and the server gives its default window of thirty seconds.
        let server_ms = wall_now_ms() - 40_000;
        let server = serve_dated(listener, vec![
            (200, json!({"validUntilMs": server_ms + 30_000}), Some(server_ms)),
        ]);
        let root = hagency_platform::legacy_writer::test_support::initialized_tempdir().unwrap();
        let login = ServerLogin::new(Some(root.path())).unwrap();
        let mut session = session_at(&origin, Instant::now() + Duration::from_secs(20), wall_now_ms() + 20_000);
        session.checked = Instant::now() - Duration::from_secs(30);
        login.sessions.lock().await.insert("ahead".into(), session);
        assert!(matches!(login.renew_session("ahead").await, Err(Error::ClockAhead)));
        assert_eq!(server.await.unwrap(), [(RENEW.to_owned(), Some("120000".to_owned()))]);
        // Named, and decided: the session is not signed out, and the proof
        // that answer ended is over here as well.
        assert_eq!(login.status.lock().await["code"], proof_clock::CLOCK_AHEAD);
        {
            let sessions = login.sessions.lock().await;
            let session = &sessions["ahead"];
            assert!(!session.invalidated);
            assert!(session.authorized_until <= Instant::now());
            assert!(session.clock_ahead_now());
        }
        // Not asked again at once, by a caller or by the worker: the server
        // is gone, and asking it would read no answer instead.
        assert!(matches!(login.authorized_device().await, Err(Error::ClockAhead)));
        assert!(matches!(login.renew_session("ahead").await, Err(Error::ClockAhead)));
    }

    #[test]
    fn a_clock_found_ahead_is_asked_about_again_once_it_was_set_or_after_a_while() {
        let mut session = session_at("https://example.test/", Instant::now(), wall_now_ms());
        session.server_clock = ServerClock::Ahead { at: Instant::now(), wall_ms: wall_now_ms() };
        assert!(session.clock_ahead_now());
        // The wall clock was set since, either way: asked again at once.
        for set_by in [-60_000i64, 60_000] {
            session.server_clock = ServerClock::Ahead {
                at: Instant::now(),
                wall_ms: wall_now_ms().saturating_add_signed(set_by),
            };
            assert!(!session.clock_ahead_now(), "{set_by}");
        }
        // Or once the spacing has passed.
        let at = Instant::now()
            .checked_sub(CLOCK_AHEAD_SPACING)
            .expect("monotonic clock older than the spacing");
        session.server_clock = ServerClock::Ahead {
            at,
            wall_ms: wall_now_ms() - CLOCK_AHEAD_SPACING.as_millis() as u64,
        };
        assert!(!session.clock_ahead_now());
        for clock in [ServerClock::Dated, ServerClock::Undated] {
            session.server_clock = clock;
            assert!(!session.clock_ahead_now());
        }
    }
}
