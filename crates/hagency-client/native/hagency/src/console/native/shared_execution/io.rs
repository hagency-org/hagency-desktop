use super::super::allocation_transport::{AllocationTransport, Operation, Scope};
use super::*;
use crate::console::server_login::OwnerError;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Weak;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PollVersion { Legacy, Claude, Octos }
pub(in crate::console::native) struct ServerOperation(Box<Action>, PollVersion);
pub(super) enum Action {
    SessionRoute { id: String, input: SharedSessionRouteInput },
    /// The shared project folder of the space a room belongs to, if any.
    RoomProjectFolder { room: String },
    Crypto(super::e2ee::CryptoAction),
    ApprovalRegister {
        authority: SharedActivityRef,
        work: SharedWorkItem,
        id: String,
        request: Value,
        remaining_ms: u64,
    },
    ApprovalTake {
        authority: SharedActivityRef,
        id: String,
        digest: String,
    },
    #[cfg(feature = "shared-budget-execution")]
    BudgetDemand(Box<super::budget_demand_io::BudgetDemandAction>),
    #[cfg(feature = "shared-budget-execution")]
    Budget(Box<super::budget_io::AttemptAction>),
    #[cfg(feature = "shared-budget-execution")]
    BudgetWork(Box<super::budget_work_io::WorkAction>),
    Context(Box<super::context_io::ContextAction>),
    Demand(Box<super::demand_io::DemandAction>),
    Attempt(Box<super::attempt_io::AttemptAction>),
    Poll {
        after: Option<String>,
    },
    Recover {
        id: String,
        digest: String,
        observation: Option<String>,
    },
    Process {
        id: String,
        input: SharedProcessInput,
    },
    Renew(SharedActivityRenewInput),
    WorkPoll(SharedActivityRef),
    WorkAck(SharedWorkInput),
    WorkStart(SharedWorkInput),
    WorkRecover {
        id: String,
        input: SharedWorkRecoverInput,
    },
    WorkResult {
        id: String,
        input: Box<SharedWorkResultUpload>,
    },
}
impl ServerOperation {
    #[cfg(test)]
    pub(in crate::console::native) fn protocol_test_poll() -> Self {
        Self::new(Action::Poll { after: None }).expect("valid sealed worker poll")
    }

    fn new(action: Action) -> Result<Self, NativeError> {
        match &action {
            Action::SessionRoute { id, input } => {
                super::super::key(id)?;
                input.validate().map_err(store)?;
            }
            Action::RoomProjectFolder { room } => {
                if !crate::console::space_folder::room_id(room) {
                    return Err(invalid());
                }
            }
            Action::Crypto(_) => {}
            Action::ApprovalRegister {
                authority,
                work,
                id,
                request,
                remaining_ms,
            } => {
                authority.validate().map_err(store)?;
                work.validate().map_err(store)?;
                super::super::key(id)?;
                if *remaining_ms == 0
                    || *remaining_ms > 120_000
                    || serde_json::to_vec(request).map_or(true, |v| v.len() > 64 * 1024)
                {
                    return Err(invalid());
                }
            }
            Action::ApprovalTake {
                authority,
                id,
                digest,
            } => {
                authority.validate().map_err(store)?;
                super::super::key(id)?;
                hash(digest)?;
            }
            #[cfg(feature = "shared-budget-execution")]
            Action::BudgetDemand(a) => a.validate()?,
            #[cfg(feature = "shared-budget-execution")]
            Action::Budget(a) => a.validate()?,
            #[cfg(feature = "shared-budget-execution")]
            Action::BudgetWork(a) => a.validate()?,
            Action::Context(a) => a.validate()?,
            Action::Attempt(a) => a.validate()?,
            Action::Demand(a) => a.validate()?,
            Action::Poll { after: Some(v) } => super::super::key(v)?,
            Action::Poll { after: None } => {}
            Action::Recover {
                id,
                digest,
                observation,
            } => {
                super::super::key(id)?;
                hash(digest)?;
                if let Some(v) = observation {
                    super::super::key(v)?;
                }
            }
            Action::Process { id, input } => {
                super::super::key(id)?;
                hash(&input.command_digest)?;
                hash(&input.token)?;
            }
            Action::Renew(v) => {
                v.authority.validate().map_err(store)?;
                super::super::key(&v.renewal_id)?;
            }
            Action::WorkPoll(v) => v.validate().map_err(store)?,
            Action::WorkAck(v) | Action::WorkStart(v) => {
                v.authority.validate().map_err(store)?;
                super::super::key(&v.work_id)?;
                hash(&v.work_digest)?;
            }
            Action::WorkRecover { id, input } => {
                super::super::key(id)?;
                hash(&input.work_digest)?;
                hash(&input.runtime_command_digest)?;
                if let Some(v) = &input.observation_id {
                    super::super::key(v)?;
                }
            }
            Action::WorkResult { id, input } => {
                super::super::key(id)?;
                hash(&input.work_digest)?;
                hash(&input.token)?;
            }
        }
        Ok(Self(Box::new(action), PollVersion::Legacy))
    }
    pub(in crate::console::native) fn request(
        self,
    ) -> Result<(reqwest::Method, String, Option<Value>), OwnerError> {
        if matches!(&*self.0,Action::Demand(a) if matches!(&**a,super::demand_io::DemandAction::Discovery))
        {
            return Ok((
                reqwest::Method::GET,
                "/hagency/api/v1/discovery".into(),
                None,
            ));
        }
        if let Action::RoomProjectFolder { room } = &*self.0 {
            let path = crate::console::space_folder::lookup_path(room).ok_or(OwnerError {
                status: 400,
                code: "invalid_shared_execution_payload".into(),
            })?;
            return Ok((reqwest::Method::GET, path, None));
        }
        let read = matches!(&*self.0,Action::Demand(a) if matches!(&**a,super::demand_io::DemandAction::Control{..}));
        let (mut path, body) = match *self.0 {
            Action::SessionRoute { id, input } => (format!("runtime/{id}/session-route"), json!(input)),
            // Answered by the read above; never a shared-execution upload.
            Action::RoomProjectFolder { .. } => {
                return Err(OwnerError { status: 400, code: "invalid_shared_execution_payload".into() });
            }
            Action::Crypto(v) => v.body(),
            Action::ApprovalRegister {
                authority,
                work,
                id,
                request,
                remaining_ms,
            } => (
                "approvals/register".into(),
                // The person who sent the message answers. The owner is not
                // asked and the server does not show the request to them.
                json!({"authority":authority,"workId":work.id,"workDigest":digest(&work).map_err(|_|OwnerError{status:400,code:"invalid_shared_work".into()})?,"id":id,"request":request,"remainingMs":remaining_ms,"requesterDecides":true}),
            ),
            Action::ApprovalTake {
                authority,
                id,
                digest,
            } => (
                "approvals/take".into(),
                json!({"authority":authority,"id":id,"digest":digest}),
            ),
            #[cfg(feature = "shared-budget-execution")]
            Action::BudgetDemand(a) => a.body(),
            #[cfg(feature = "shared-budget-execution")]
            Action::Budget(a) => a.body(),
            #[cfg(feature = "shared-budget-execution")]
            Action::BudgetWork(a) => a.body(),
            Action::Context(a) => a.body(),
            Action::Attempt(a) => a.body(),
            Action::Demand(a) => a.body(),
            Action::Poll { after } => ("runtime/poll".into(), json!({"after":after})),
            Action::Recover {
                id,
                digest,
                observation,
            } => (
                format!("runtime/{id}/recover"),
                json!({"commandDigest":digest,"observationId":observation}),
            ),
            Action::Process { id, input } => (format!("runtime/{id}/result"), json!(input)),
            Action::Renew(input) => ("activity/renew".into(), json!(input)),
            Action::WorkPoll(authority) => ("work/poll".into(), json!({"authority":authority})),
            Action::WorkAck(input) => ("work/ack".into(), json!(input)),
            Action::WorkStart(input) => ("work/start".into(), json!(input)),
            Action::WorkRecover { id, input } => (format!("work/{id}/recover"), json!(input)),
            Action::WorkResult { id, input } => (format!("work/{id}/result"), json!(input)),
        };
        if serde_json::to_vec(&body).map_or(true, |v| {
            v.len()
                > if path.starts_with("approvals/") {
                    96 * 1024
                } else if path.contains("work/") && path.ends_with("/result") {
                    64 * 1024
                } else {
                    16 * 1024
                }
        }) {
            return Err(OwnerError {
                status: 400,
                code: "invalid_shared_execution_payload".into(),
            });
        }
        if matches!(
                path.as_str(),
                "runtime/poll"
                    | "attempts/poll"
                    | "budget-attempts/poll"
                    | "demands/poll"
                    | "budget-demands/poll"
            )
        {
            match self.1 {
                PollVersion::Legacy => {},
                PollVersion::Claude => path.push_str("-v4"),
                PollVersion::Octos => path.push_str("-v5"),
            }
        }
        Ok((
            if read {
                reqwest::Method::GET
            } else {
                reqwest::Method::POST
            },
            format!("/hagency/api/v1/management/shared-execution/{path}"),
            if read { None } else { Some(body) },
        ))
    }
}
/// Current SDK-owned scopes are reacquired per operation; a retained runtime
/// does not retain a strong NativeOwner cycle or stale authentication lease.
#[derive(Clone)]
pub(super) struct Io {
    permission: Permission,
    manager: Weak<NativeManagement>,
    host_gate: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    namespace: Option<Arc<hagency_store::owner_resources::ResourceNamespace>>,
    provider_gate: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    claude_discovery: Arc<tokio::sync::Mutex<Option<(Instant, PollVersion)>>>,
    pub(super) project_folders: crate::console::space_folder::KnownFolders,
    #[cfg(test)]
    pub(super) crypto_fixture: Option<Arc<super::e2ee::CryptoFixture>>,
    /// Where tests keep space project folders; tests never use the real home.
    #[cfg(test)]
    pub(super) project_root: Option<std::path::PathBuf>,
}
pub(super) struct Bound {
    permission: Permission,
    pub transport: AllocationTransport,
    pub scope: Scope,
    namespace: Option<Arc<hagency_store::owner_resources::ResourceNamespace>>,
    provider_gate: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    claude_discovery: Arc<tokio::sync::Mutex<Option<(Instant, PollVersion)>>>,
}
impl Io {
    pub(super) fn hosting_admission(
        &self,
    ) -> Result<Arc<crate::console::hosting::Admission>, NativeError> {
        let manager = self
            .manager
            .upgrade()
            .ok_or_else(|| failure(409, "shared_execution_closed"))?;
        Ok(manager.owner.0.host.console.hosting_admission())
    }
    pub(super) fn crypto_directory(&self) -> Result<std::path::PathBuf, NativeError> {
        let manager = self
            .manager
            .upgrade()
            .ok_or_else(|| failure(409, "shared_execution_closed"))?;
        let root = manager
            .owner
            .0
            .host
            .console
            .0
            .server_login
            .state_directory()
            .ok_or_else(|| failure(409, "shared_crypto_storage_unavailable"))?
            .join("shared-matrix-devices");
        hagency_store::private::directory(&root).map_err(store)?;
        Ok(root)
    }

    pub fn new(manager: &Arc<NativeManagement>) -> Self {
        Self {
            manager: Arc::downgrade(manager),
            permission: Permission::ExecuteManagedAgents,
            host_gate: None,
            namespace: None,
            provider_gate: None,
            claude_discovery: Arc::default(),
            project_folders: Default::default(),
            #[cfg(test)]
            crypto_fixture: None,
            #[cfg(test)]
            project_root: None,
        }
    }
    #[cfg(feature = "shared-intake")]
    pub fn for_owner_stop(manager: &Arc<NativeManagement>) -> Self {
        Self {
            permission: Permission::ChangeAgentRunControl,
            ..Self::new(manager)
        }
    }
    pub fn with_provider_gate(&self, gate: Arc<dyn Fn() -> bool + Send + Sync>) -> Self {
        Self {
            provider_gate: Some(gate),
            ..self.clone()
        }
    }
    pub fn with_namespace(
        &self,
        namespace: Option<Arc<hagency_store::owner_resources::ResourceNamespace>>,
    ) -> Self {
        Self {
            namespace,
            ..self.clone()
        }
    }
    pub fn restrict_to_host(&mut self, gate: Arc<dyn Fn() -> bool + Send + Sync>) {
        let prior = self.host_gate.take();
        self.host_gate = Some(Arc::new(move || {
            gate() && prior.as_ref().is_none_or(|p| p())
        }));
    }
    pub async fn bound(&self) -> Result<Bound, NativeError> {
        if self.host_gate.as_ref().is_some_and(|gate| !gate()) {
            return Err(failure(401, "management_scope_changed"));
        }
        let manager = self
            .manager
            .upgrade()
            .ok_or_else(|| failure(409, "shared_execution_closed"))?;
        let mut transport = AllocationTransport::new(manager);
        if let Some(gate) = &self.host_gate {
            transport = transport.restrict_to_host(gate.clone());
        }
        let scope = transport.scope(self.permission).await?;
        Ok(Bound {
            transport,
            scope,
            permission: self.permission,
            namespace: self.namespace.clone(),
            provider_gate: self.provider_gate.clone(),
            claude_discovery: self.claude_discovery.clone(),
        })
    }
    pub async fn check(&self, c: &SharedRuntimeCommand) -> Result<(), NativeError> {
        let b = self.bound().await?;
        binding(&b.scope, c, true)?;
        b.check_namespace(c).await?;
        Ok(())
    }
    /// The same check from inside a running process. Around each of the
    /// daemon's SDK credential refreshes (every few minutes) the owner's
    /// management boundary briefly reports a changed scope, a lost
    /// authorization or an unknown outcome. Worker steps retry on their next
    /// pass, but a run ended on the first one and its message went unanswered.
    /// Such codes are retried for a short while; a real revocation still ends
    /// the run once they persist. Callers keep the run's cancellation and
    /// activity deadline around this.
    pub async fn check_running(&self, c: &SharedRuntimeCommand) -> Result<(), NativeError> {
        self.check_settled(c, || Ok(())).await
    }
    /// `check_running` for a step of a turn under its activity lease, such as
    /// a room call: each attempt first reads the lease on the wall clock
    /// (`wall_held`), as a proof's does. The turn waits on such a step without
    /// renewing its lease meanwhile. Without the wall-clock check, after a
    /// sleep of the computer a server that did not answer was asked again for
    /// `RUNNING_CHECK_SETTLE`, past the lease's next renewal moment.
    pub(super) async fn check_held(
        &self,
        c: &SharedRuntimeCommand,
        until: Instant,
        wall_end_ms: u64,
    ) -> Result<(), NativeError> {
        self.check_settled(c, || wall_held("check", until, wall_end_ms)).await
    }
    /// `check_held` for a step beside a Codex turn, under the lease in force
    /// at each attempt (`Held`). The lease is renewed beside such a step, so
    /// it ends by the next renewal moment without this check; with it, the
    /// step is not taken under a lease the wall clock has ended.
    pub(super) async fn check_under(
        &self,
        c: &SharedRuntimeCommand,
        held: &Held,
    ) -> Result<(), NativeError> {
        self.check_settled(c, || held.check("check")).await
    }
    async fn check_settled(
        &self,
        c: &SharedRuntimeCommand,
        held: impl Fn() -> Result<(), NativeError>,
    ) -> Result<(), NativeError> {
        let settle = std::time::Instant::now() + RUNNING_CHECK_SETTLE;
        loop {
            held()?;
            match self.check(c).await {
                Err(error) if self.transient(&error) && std::time::Instant::now() < settle => {
                    tracing::info!(code = %error.code, "Shared run authority check retrying");
                    tokio::time::sleep(RETRY_GAP).await;
                }
                result => return result,
            }
        }
    }
    /// `bound` from inside a running process, with the settling of
    /// `check_running`: the idle pass of a turn asks for a scope twice a
    /// second, and one refusal during a credential refresh ended the turn.
    pub async fn bound_running(&self) -> Result<Bound, NativeError> {
        let settle = Instant::now() + RUNNING_CHECK_SETTLE;
        self.settled(settle, || self.bound()).await
    }
    /// `bound_running` for a step of a turn under its activity lease, as
    /// `check_held`: the idle pass of a turn with owner approvals.
    pub(super) async fn bound_held(
        &self,
        until: Instant,
        wall_end_ms: u64,
    ) -> Result<Bound, NativeError> {
        let settle = Instant::now() + RUNNING_CHECK_SETTLE;
        self.settled(settle, || async {
            wall_held("scope", until, wall_end_ms)?;
            self.bound().await
        })
        .await
    }
    /// Runs one authority step of a running process and asks it again, until
    /// `until` (the activity lease, at the latest), while its failure is not a
    /// refusal. The step is run again from its start and may already have
    /// reached the server, so it has to be one whose exact replay is harmless:
    /// it reads, records the same local row under the same id, or uploads
    /// something the server answers again with what it already did.
    pub(super) async fn settled<T, F>(
        &self,
        until: Instant,
        step: impl FnMut() -> F,
    ) -> Result<T, NativeError>
    where
        F: std::future::Future<Output = Result<T, NativeError>>,
    {
        settle(until, |error| self.transient(error), step).await
    }
    /// The proof for a step under an activity lease: a scope, the enrollment
    /// under it, and the store's comparison of both with the lease
    /// (`authorize`, given the end the proof may keep). It is read-only until
    /// it answers. Around a credential refresh the boundary refuses for a
    /// moment; that is waited out while the lease lasts (`settled`) instead of
    /// ending the turn. A real revocation is not retried.
    ///
    /// Each attempt first reads the lease on the wall clock (`wall_held`),
    /// before it asks the server anything. After a sleep of the computer the
    /// lease's counted end `until` still lies ahead, by what was left of the
    /// lease before the sleep. The store reads the wall clock as well, but only
    /// once the server has answered for the scope and the enrollment: a server
    /// that did not answer was asked again until that counted end, and the
    /// turn worked on meanwhile.
    pub(super) async fn lease_proof(
        &self,
        r: &OwnerResourceRegistry,
        command: &SharedRuntimeCommand,
        lease: &SharedActivityLease,
        until: Instant,
        authorize: impl Fn(
            VerifiedManagementEnrollment,
            Instant,
        ) -> Result<VerifiedSharedActivity, hagency_store::Error>,
    ) -> Result<(Bound, VerifiedSharedActivity), NativeError> {
        self.settled(until, || async {
            wall_held("proof", until, lease.expires_at_ms)?;
            let b = self.bound().await?;
            let e = b.enrollment(r, command, true).await?;
            let verified = authorize(e, until.min(b.scope.deadline)).map_err(store)?;
            Ok((b, verified))
        })
        .await
    }
    /// `settled` for a step that no lease bounds: it is asked again for as
    /// long as a running process waits out a check.
    #[cfg(feature = "shared-budget-execution")]
    pub(super) async fn settled_shortly<T, F>(&self, step: impl FnMut() -> F) -> Result<T, NativeError>
    where
        F: std::future::Future<Output = Result<T, NativeError>>,
    {
        self.settled(Instant::now() + RUNNING_CHECK_SETTLE, step).await
    }
    /// An exchange with the server between the poll for work and the start
    /// of its turn: the poll itself, the binding, the acknowledgement and the
    /// context. It runs under the proof in hand. When it gets no answer
    /// (`NativeError::said`) it is asked again under a fresh proof while the
    /// lease lasts, as a check and a renewal are. The server answers each of
    /// these a second time with what it answered or did the first time: a
    /// poll gives the same work until it has started, an acknowledged work is
    /// acknowledged, and a context frozen before is given back as it is.
    ///
    /// Only the exchange goes in `step`. What this host records from its
    /// answer is written once, after it: a refusal of its own records is a
    /// decision, and the store's refusal reads like a replaced scope
    /// (`management_access_required`), which is asked again.
    ///
    /// Before, one such exchange without an answer ended the process, and
    /// with it a message the server had already bound to it. The start of
    /// the turn is not asked again here: an unanswered start is unknown, and
    /// stays so.
    ///
    /// `step` takes the proof and keeps it only while it runs, so nothing is
    /// held during the wait.
    pub(super) async fn asked_again<P, T, N, NF, S, SF>(
        &self,
        until: Instant,
        first: P,
        next: N,
        step: S,
    ) -> Result<T, NativeError>
    where
        N: Fn() -> NF,
        NF: std::future::Future<Output = Result<P, NativeError>>,
        S: Fn(P) -> SF,
        SF: std::future::Future<Output = Result<T, NativeError>>,
    {
        let mut proof = first;
        loop {
            match step(proof).await {
                Err(error) if self.transient(&error) && Instant::now() + RETRY_GAP < until => {
                    tracing::info!(code = %error.code, said = ?error.said(), "Pooled work step before its turn got no answer; asking again");
                    tokio::time::sleep(RETRY_GAP).await;
                    proof = next().await?;
                }
                result => return result,
            }
        }
    }
    /// Hosting stopping, or this management package retired or expired, is a
    /// real revocation: no retry, the run stops now.
    fn revoked(&self) -> bool {
        self.host_gate.as_ref().is_some_and(|gate| !gate())
            || self
                .manager
                .upgrade()
                .is_none_or(|manager| manager.is_retired())
    }
    /// A boundary failure worth waiting out: one of the shared codes, and not a
    /// real revocation.
    pub(super) fn transient(&self, error: &NativeError) -> bool {
        transient_boundary(error) && !self.revoked()
    }
    /// Uploads one activity renewal. If its reply is lost or a boundary check
    /// fails, the SAME renewal (id and authority) is resubmitted under a fresh
    /// scope until the lease ends: the server returns an already applied
    /// renewal again instead of renewing twice, so the run keeps its lease.
    ///
    /// The lease ends on either clock, as in `lease_end`: at `until`, counted
    /// from its request, or at `wall_end_ms`, its end on the wall clock. The
    /// wall clock is read before each send and before each fresh scope
    /// (`wall_held`). Only it counts a sleep of the computer that falls
    /// between two of these; read on the other clock alone, the renewal was
    /// sent again, and the turn worked on, for as long as the lease had been
    /// left before the sleep.
    pub(super) async fn upload_renewal<T: DeserializeOwned>(
        &self,
        mut b: Bound,
        action: impl Fn() -> Action,
        until: Instant,
        wall_end_ms: u64,
    ) -> Result<(Bound, Timed<T>), NativeError> {
        let fresh_scope = || async {
            wall_held("renewal", until, wall_end_ms)?;
            self.bound().await
        };
        loop {
            wall_held("renewal", until, wall_end_ms)?;
            let error = match b.upload(action(), until).await {
                Ok(reply) => return Ok((b, reply)),
                Err(error) => error,
            };
            if !self.transient(&error) || Instant::now() + RETRY_GAP >= until {
                return Err(error);
            }
            tracing::info!(code = %error.code, said = ?error.said(), "Shared activity renewal resubmitting the same renewal");
            // The scope that failed is given back before the wait: a waiter
            // holds no management session.
            drop(b);
            tokio::time::sleep(RETRY_GAP).await;
            b = match fresh_scope().await {
                Ok(fresh) => fresh,
                Err(error) if self.transient(&error) && Instant::now() + RETRY_GAP < until => {
                    tokio::time::sleep(RETRY_GAP).await;
                    fresh_scope().await?
                }
                Err(error) => return Err(error),
            };
        }
    }
}

pub(super) const RETRY_GAP: std::time::Duration = std::time::Duration::from_secs(2);

/// Whether the wall clock says that a lease ending at `wall_end_ms` has
/// ended. It is the clock the store judges the lease by
/// (`authorize_activity`), so that both see a sleep of the computer alike.
fn wall_ended(wall_end_ms: u64) -> bool {
    hagency_store::owner_resources::wall_clock_ms().is_ok_and(|now| now >= wall_end_ms)
}
/// Refuses a step under an activity lease that the wall clock has ended,
/// before the step asks the server anything: as at the lease's counted end,
/// with `shared_execution_expired`, a decision that ends the run. What that
/// counted end still leaves (`counted_left_ms`) tells a sleep of the computer
/// from a lease that ran out awake: only the wall clock counts a sleep.
pub(super) fn wall_held(step: &'static str, until: Instant, wall_end_ms: u64) -> Result<(), NativeError> {
    if !wall_ended(wall_end_ms) {
        return Ok(());
    }
    tracing::warn!(
        step,
        counted_left_ms = until.saturating_duration_since(Instant::now()).as_millis() as u64,
        "An activity lease has ended on the wall clock; the step under it is not taken"
    );
    Err(failure(409, "shared_execution_expired"))
}
/// The lease a Codex turn holds now, by its ends on both clocks, for the
/// steps that run while the driver renews that lease beside them: the run
/// controls, the delivery of the turn's messages, and the opening of its
/// provider thread. The driver replaces its lease at each renewal and updates
/// this with it, as the Claude driver's watch channel does, so such a step
/// reads the lease in force, not the one it began under.
#[derive(Clone)]
pub(super) struct Held(Arc<std::sync::Mutex<(Instant, u64)>>);
impl Held {
    pub fn new(until: Instant, wall_end_ms: u64) -> Self {
        Self(Arc::new(std::sync::Mutex::new((until, wall_end_ms))))
    }
    /// The ends of the lease a renewal granted.
    pub fn renewed(&self, until: Instant, wall_end_ms: u64) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = (until, wall_end_ms);
    }
    /// `wall_held` under the lease in force now.
    pub fn check(&self, step: &'static str) -> Result<(), NativeError> {
        let (until, wall_end_ms) = *self.0.lock().unwrap_or_else(|e| e.into_inner());
        wall_held(step, until, wall_end_ms)
    }
}
/// What the wall clock leaves of a lease that ends at `wall_end_ms`, for the
/// log; nothing when that clock cannot be read.
pub(super) fn wall_left_ms(wall_end_ms: u64) -> Option<u64> {
    hagency_store::owner_resources::wall_clock_ms()
        .ok()
        .map(|now| wall_end_ms.saturating_sub(now))
}

/// The one line for work that ends before its turn began: the step it ended
/// at and what the failure said (`NativeError::said`). Step names and codes
/// only, never identifiers.
pub(super) fn ended_before_its_turn(stage: &'static str, error: &NativeError) {
    tracing::warn!(
        stage,
        status = error.status,
        code = %error.code,
        said = ?error.said(),
        "Pooled work ended before its turn began"
    );
}

/// The retry rule behind `Io::settled`, apart from `Io` so it can be tested:
/// a failure `transient` accepts is retried after `RETRY_GAP` while one more
/// gap still fits before `until`; any other result is returned as it is.
async fn settle<T, F>(
    until: Instant,
    transient: impl Fn(&NativeError) -> bool,
    mut step: impl FnMut() -> F,
) -> Result<T, NativeError>
where
    F: std::future::Future<Output = Result<T, NativeError>>,
{
    loop {
        match step().await {
            Err(error) if transient(&error) && Instant::now() + RETRY_GAP < until => {
                tracing::info!(code = %error.code, said = ?error.said(), "Shared run authority step retrying");
                tokio::time::sleep(RETRY_GAP).await;
            }
            result => return result,
        }
    }
}

/// How long a running process waits out a transient authority failure.
const RUNNING_CHECK_SETTLE: std::time::Duration = std::time::Duration::from_secs(20);

/// A failure that was not a refusal (`NativeError::said`): the step could not
/// be asked, its scope was replaced, or it was sent and got no answer. A
/// driver asks such a step again. Every step it does that with reads, or is
/// answered by the server a second time with what it already did.
fn transient_boundary(error: &NativeError) -> bool {
    error.passes()
}
impl Bound {
    /// Preparation only inspects provider configuration. Pin all its checks to
    /// the delivery scope, including its expiry, and the selected launch gates.
    /// A running process still obtains new scopes through its own Io.
    pub(super) fn preparation_check(
        mut self,
        launch: &Io,
        command: SharedRuntimeCommand,
    ) -> super::super::managed_runtime::LiveCheck {
        self.namespace = launch.namespace.clone();
        self.provider_gate = launch.provider_gate.clone();
        let bound = Arc::new(self);
        let command = Arc::new(command);
        Arc::new(move || {
            let b = bound.clone();
            let command = command.clone();
            Box::pin(async move {
                b.transport.check_scope(&b.scope, false).await?;
                binding(&b.scope, &command, true)?;
                b.check_namespace(&command).await
            })
        })
    }
    async fn check_namespace(&self, c: &SharedRuntimeCommand) -> Result<(), NativeError> {
        if self.provider_gate.as_ref().is_some_and(|gate| !gate()) {
            return Err(failure(409, "provider_account_changed"));
        }
        if let Some(namespace) = &self.namespace {
            if !namespace.matches_resource(
                &c.reserve_command.resource_id,
                &c.reserve_command.resource_revision,
            ) {
                return Err(invalid());
            }
            namespace
                .check_with(self.scope.profile.clone())
                .await
                .map_err(|_| failure(409, "shared_execution_namespace_unavailable"))?;
        }
        Ok(())
    }
    pub async fn demand_discovery(&self) -> Result<super::demand_io::Discovery, NativeError> {
        self.transport
            .request(
                &self.scope,
                Operation::SharedExecution(ServerOperation::new(
                    super::demand_io::DemandAction::Discovery.into(),
                )?),
                false,
            )
            .await
    }

    /// The shared project folder of a Room's space. A plain read, answered
    /// without a server timestamp, like discovery.
    pub async fn room_project_folder(
        &self,
        room: &str,
    ) -> Result<Option<crate::console::space_folder::RoomFolder>, NativeError> {
        let reply: crate::console::space_folder::RoomFolderReply = self
            .transport
            .request(
                &self.scope,
                Operation::SharedExecution(ServerOperation::new(Action::RoomProjectFolder {
                    room: room.into(),
                })?),
                false,
            )
            .await?;
        Ok(reply.project_folder)
    }

    async fn operation(&self, action: Action) -> Result<ServerOperation, NativeError> {
        let poll = match &action {
            Action::Poll { .. } => true,
            Action::Attempt(a) => matches!(**a, super::attempt_io::AttemptAction::Poll { .. }),
            Action::Demand(a) => matches!(**a, super::demand_io::DemandAction::Poll { .. }),
            #[cfg(feature = "shared-budget-execution")]
            Action::Budget(a) => matches!(**a, super::budget_io::AttemptAction::Poll { .. }),
            #[cfg(feature = "shared-budget-execution")]
            Action::BudgetDemand(a) => matches!(
                **a,
                super::budget_demand_io::BudgetDemandAction::Poll { .. }
            ),
            _ => false,
        };
        let mut operation = ServerOperation::new(action)?;
        if poll {
            let mut cached = self.claude_discovery.lock().await;
            let supported = if let Some((at, value)) = *cached
                && at.elapsed() < Duration::from_secs(30)
            {
                value
            } else {
                let supported = match self.demand_discovery().await {
                    Ok(discovery) => {
                        if discovery.capabilities.len() > 64
                            || discovery.capabilities.iter().any(|c| c.len() > 128)
                        {
                            return Err(invalid());
                        }
                        if cfg!(target_os = "macos") && discovery.capabilities.iter().any(|c| c == "shared-octos-execution-v1") {
                            PollVersion::Octos
                        } else if discovery.capabilities.iter().any(|c| c == "shared-claude-execution-v1") {
                            PollVersion::Claude
                        } else {
                            PollVersion::Legacy
                        }
                    }
                    Err(error) if error.status == 404 => PollVersion::Legacy,
                    Err(error) => return Err(error),
                };
                *cached = Some((Instant::now(), supported));
                supported
            };
            operation.1 = supported;
        }
        Ok(operation)
    }
    fn check_action(&self, action: &Action) -> Result<(), NativeError> {
        if self.permission == Permission::ChangeAgentRunControl
            && !matches!(action,Action::Demand(a) if matches!(&**a,super::demand_io::DemandAction::Stop{..}|super::demand_io::DemandAction::Control{..}|super::demand_io::DemandAction::Discovery))
        {
            return Err(failure(403, "management_permission_required"));
        }
        Ok(())
    }
    pub async fn request<T: DeserializeOwned>(
        &self,
        a: Action,
        uncertain: bool,
    ) -> Result<Timed<T>, NativeError> {
        self.check_action(&a)?;
        self.transport
            .request(
                &self.scope,
                Operation::SharedExecution(self.operation(a).await?),
                uncertain,
            )
            .await
    }
    pub async fn upload<T: DeserializeOwned>(
        &self,
        a: Action,
        until: Instant,
    ) -> Result<Timed<T>, NativeError> {
        self.check_action(&a)?;
        self.transport
            .upload(
                &self.scope,
                Operation::SharedExecution(self.operation(a).await?),
                until,
            )
            .await
    }
    pub async fn enrollment(
        &self,
        r: &OwnerResourceRegistry,
        c: &SharedRuntimeCommand,
        fresh: bool,
    ) -> Result<VerifiedManagementEnrollment, NativeError> {
        binding(&self.scope, c, fresh)?;
        if fresh {
            self.check_namespace(c).await?;
        }
        let grant = if let Some(grant) = &c.reserve_command.pool_grant {
            grant.clone()
        } else {
            r.managed_agent_project_history(
                self.scope.profile.clone(),
                self.scope.device.device_id().into(),
                c.reserve_command.allocation_id.clone(),
                c.reserve_command.allocation_receipt_digest.clone(),
            )
            .await
            .map_err(store)?
            .grant_receipt
        };
        if digest(&grant)? != c.reserve_command.grant_receipt_digest {
            return Err(invalid());
        }
        let e = self
            .transport
            .enrollment(&self.scope, &grant, fresh)
            .await?;
        self.transport.check_scope(&self.scope, false).await?;
        if fresh {
            self.check_namespace(c).await?;
        }
        Ok(e)
    }
    pub async fn cleanup(
        &self,
        r: &OwnerResourceRegistry,
        c: &SharedRuntimeCommand,
    ) -> Result<ManagedCleanupAuthority, NativeError> {
        let e = self.enrollment(r, c, false).await?;
        let a = r
            .shared_cleanup_authority(e, c.clone())
            .await
            .map_err(store)?;
        self.transport.check_scope(&self.scope, false).await?;
        Ok(a)
    }
    /// Read-only lookup used only for non-executable successor deliveries.
    /// Current scope and immutable command checks still bracket missing history.
    pub async fn find_cleanup(
        &self,
        r: &OwnerResourceRegistry,
        c: &SharedRuntimeCommand,
    ) -> Result<Option<ManagedCleanupAuthority>, NativeError> {
        binding(&self.scope, c, false)?;
        self.transport.check_scope(&self.scope, false).await?;
        let grant = if let Some(grant) = &c.reserve_command.pool_grant {
            grant.clone()
        } else {
            let project = r.find_managed_agent_project_history(
                self.scope.profile.clone(),
                self.scope.device.device_id().into(),
                c.reserve_command.allocation_id.clone(),
                c.reserve_command.allocation_receipt_digest.clone(),
            ).await.map_err(store)?;
            let Some(project) = project else {
                self.transport.check_scope(&self.scope, false).await?;
                return Ok(None);
            };
            project.grant_receipt
        };
        if digest(&grant)? != c.reserve_command.grant_receipt_digest {
            return Err(invalid());
        }
        let enrollment = self.transport.enrollment(&self.scope, &grant, false).await?;
        let result = r.find_shared_cleanup_authority(enrollment, c.clone()).await.map_err(store)?;
        self.transport.check_scope(&self.scope, false).await?;
        Ok(result)
    }
}
pub(super) fn binding(s: &Scope, c: &SharedRuntimeCommand, fresh: bool) -> Result<(), NativeError> {
    c.validate(s.device.owner_mxid()).map_err(store)?;
    if c.reserve_command.executor_device_id != s.device.device_id()
        || c.reserve_command.executor_generation > s.device.generation()
        || (fresh && c.reserve_command.executor_generation != s.device.generation())
    {
        return Err(failure(409, "shared_execution_enrollment_changed"));
    }
    Ok(())
}
/// Extract only the mandatory authenticated server timestamp, then decode the
/// remainder into a closed DTO. No arbitrary server fields survive this step.
pub(super) struct Timed<T> {
    pub value: T,
    pub observed: u64,
}
impl<'de, T: DeserializeOwned> Deserialize<'de> for Timed<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut value = Value::deserialize(d)?;
        let observed = value
            .as_object_mut()
            .and_then(|o| o.remove("observedAtMs"))
            .and_then(|v| v.as_u64())
            .filter(|v| *v > 0 && *v <= hagency_core::JSON_SAFE_MAX)
            .ok_or_else(|| serde::de::Error::custom("invalid observation"))?;
        let value = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self { value, observed })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DeliveryReply {
    pub delivery: Option<SharedRuntimeDelivery>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ActivityReply {
    pub activity: SharedActivityLease,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct WorkReply {
    pub work: Option<SharedWorkItem>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AckReply {
    pub acknowledged: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TicketReply {
    pub ticket: SharedWorkResultTicket,
}
/// When an activity lease is renewed: ten seconds after it was granted, and
/// no later than twenty seconds before its end, but not sooner than five
/// seconds after the grant. A thirty-second lease is renewed when it always
/// was. One that follows a longer authorization window is renewed as often,
/// so a renewal that gets no answer has the rest of the lease to be asked
/// again. A lease that is shorter than twenty seconds, because its authority
/// ends or because this computer's clock is ahead (`lease_end`), is renewed
/// half-way through it and not at once: at once, each renewal would be due
/// the moment it is answered. The five seconds are for the same reason. A
/// clock that is ahead cuts every lease to the same length, and one a little
/// longer than twenty seconds was renewed each time the moment it was
/// granted.
pub(super) fn renewal_moment(granted: Instant, until: Instant) -> Instant {
    let length = until.saturating_duration_since(granted);
    match length.checked_sub(Duration::from_secs(20)) {
        Some(early) if !early.is_zero() => {
            granted + early.clamp(Duration::from_secs(5), Duration::from_secs(10))
        }
        _ => granted + length / 2,
    }
}
/// The local end of an activity lease: what the server issued, counted from
/// the moment the request for it was sent, and never past the end of the
/// authority it is held under (`Scope::authority_end`).
///
/// A lease is what a process holds between steps. It is not cut to the scope
/// of the request that brought it, as a ticket is (`window`): each step takes
/// a scope of its own, and that scope ends within a minute. Cut to it, a
/// lease that follows a two-minute window would end after one.
///
/// It also ends no later than the wall clock says, which may only end it
/// earlier. The store reads the same clock (`authorize_activity`). Taken
/// into the end, that clock brings the renewal before it
/// (`renewal_moment`): a computer whose clock is ahead by most of a lease
/// keeps the lease, renewed sooner. Counted from the request alone, the lease
/// was renewed after the wall clock had ended it, and the process ended at
/// its first renewal.
pub(super) fn lease_end(
    token: &str,
    until: u64,
    observed: u64,
    started: Instant,
    authority_end: Instant,
) -> Result<Instant, NativeError> {
    let wall_now_ms = hagency_store::owner_resources::wall_clock_ms().map_err(store)?;
    lease_end_at(token, until, observed, started, authority_end, wall_now_ms)
}
/// `lease_end`, with the wall clock read at `wall_now_ms`.
fn lease_end_at(
    token: &str,
    until: u64,
    observed: u64,
    started: Instant,
    authority_end: Instant,
    wall_now_ms: u64,
) -> Result<Instant, NativeError> {
    hash(token)?;
    let longest = hagency_runtime::owned::LONGEST_ACTIVITY_LEASE.as_millis() as u64;
    let remaining = until
        .checked_sub(observed)
        .filter(|v| *v > 0 && *v <= longest)
        .ok_or_else(invalid)?;
    let now = Instant::now();
    // No more than the lease: a clock that is slow gives it more.
    let wall_left = until.saturating_sub(wall_now_ms).min(remaining);
    let wall_end = now + Duration::from_millis(wall_left);
    let end = (started + Duration::from_millis(remaining))
        .min(authority_end)
        .min(wall_end);
    if now >= end {
        return Err(failure(409, "shared_execution_expired"));
    }
    Ok(end)
}
pub(super) fn window(
    token: &str,
    until: u64,
    observed: u64,
    started: Instant,
    scope: &Scope,
) -> Result<Instant, NativeError> {
    hash(token)?;
    let remaining = until
        .checked_sub(observed)
        .filter(|v| *v > 0 && *v <= 30_000)
        .ok_or_else(invalid)?;
    let deadline = (started + Duration::from_millis(remaining)).min(scope.deadline);
    if Instant::now() >= deadline {
        return Err(failure(409, "shared_execution_ticket_expired"));
    }
    Ok(deadline)
}

#[cfg(test)]
mod lease_tests {
    use super::*;

    fn end(lease_ms: u64, authority_in: u64) -> Result<Instant, NativeError> {
        let started = Instant::now();
        lease_end_at(
            &"c".repeat(64),
            1_000_000 + lease_ms,
            1_000_000,
            started,
            started + Duration::from_secs(authority_in),
            1_000_000,
        )
    }
    /// A lease issued with `lease_ms`, on a computer whose wall clock is
    /// `ahead_ms` ahead of the server's (behind, if negative).
    fn end_ahead(lease_ms: u64, ahead_ms: i64) -> Result<Instant, NativeError> {
        let started = Instant::now();
        lease_end_at(
            &"c".repeat(64),
            1_000_000 + lease_ms,
            1_000_000,
            started,
            started + Duration::from_secs(300),
            1_000_000u64.saturating_add_signed(ahead_ms),
        )
    }
    fn secs(until: Instant) -> u64 {
        until.saturating_duration_since(Instant::now()).as_secs()
    }

    /// A lease that follows a two-minute window lasts as long as it was
    /// issued for. The scope of one request ends within a minute; a lease
    /// that ended with it would be half as long.
    #[test]
    fn a_lease_lasts_as_long_as_it_was_issued_for_under_its_authority() {
        assert!((118..=120).contains(&secs(end(120_000, 300).unwrap())));
        assert!((88..=90).contains(&secs(end(90_000, 300).unwrap())));
        assert!((28..=30).contains(&secs(end(30_000, 300).unwrap())));
        // The authority it is held under ends first.
        assert!((98..=100).contains(&secs(end(120_000, 100).unwrap())));
        assert!((8..=10).contains(&secs(end(120_000, 10).unwrap())));
    }

    #[test]
    fn a_lease_longer_than_the_longest_window_is_refused() {
        assert!(end(120_001, 300).is_err());
        assert!(end(0, 300).is_err());
        let started = Instant::now();
        let passed = lease_end_at(&"c".repeat(64), 999_999, 1_000_000, started, started, 1_000_000);
        assert!(passed.is_err());
        // An authority that has ended leaves nothing of the lease.
        assert_eq!(end(30_000, 0).unwrap_err().code, "shared_execution_expired");
    }

    /// The wall clock may only end a lease earlier. A computer whose clock is
    /// behind holds the whole lease. One whose clock is ahead holds what its
    /// clock leaves of it, and renews before that runs out: counted from the
    /// request alone, its renewal came after the store had refused the lease
    /// on the wall clock, and the process ended at its first renewal.
    #[test]
    fn the_wall_clock_only_ends_a_lease_earlier() {
        assert!((28..=30).contains(&secs(end_ahead(30_000, -10_000).unwrap())));
        assert!((118..=120).contains(&secs(end_ahead(120_000, -60_000).unwrap())));
        let started = Instant::now();
        let until = end_ahead(30_000, 20_000).unwrap();
        assert!((8..=10).contains(&secs(until)));
        let renewal = renewal_moment(started, until);
        assert!(renewal < until);
        assert!(renewal <= started + Duration::from_millis(5_500));
        let until = end_ahead(120_000, 100_000).unwrap();
        assert!((18..=20).contains(&secs(until)));
        assert!(renewal_moment(started, until) <= started + Duration::from_secs(10));
        // Ahead by the whole lease: nothing is left of it.
        assert_eq!(end_ahead(30_000, 30_000).unwrap_err().code, "shared_execution_expired");
    }

    #[test]
    fn a_lease_is_renewed_ten_seconds_after_its_grant_whatever_its_length() {
        let granted = Instant::now();
        let after = |ms: u64| granted + Duration::from_millis(ms);
        // Thirty seconds: twenty seconds before the end, the moment before.
        assert_eq!(renewal_moment(granted, after(30_000)), after(10_000));
        // One that follows a longer window is not left until its last moments.
        assert_eq!(renewal_moment(granted, after(120_000)), after(10_000));
        assert_eq!(renewal_moment(granted, after(60_000)), after(10_000));
        // One a little shorter is renewed twenty seconds before its end, as before.
        assert_eq!(renewal_moment(granted, after(25_000)), after(5_000));
        // One shorter still is not renewed sooner than five seconds after
        // its grant: twenty seconds before its end, that was at once.
        assert_eq!(renewal_moment(granted, after(24_000)), after(5_000));
        assert_eq!(renewal_moment(granted, after(20_001)), after(5_000));
        // One that leaves less than that is renewed half-way through, never
        // at once.
        assert_eq!(renewal_moment(granted, after(20_000)), after(10_000));
        assert_eq!(renewal_moment(granted, after(8_000)), after(4_000));
        assert_eq!(renewal_moment(granted, granted), granted);
    }
}

#[cfg(test)]
mod settle_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn flaky(calls: &AtomicU32, failures: u32, code: &'static str) -> Result<u32, NativeError> {
        let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
        if call <= failures {
            Err(failure(401, code))
        } else {
            Ok(call)
        }
    }

    #[test]
    fn only_the_writer_contention_response_is_retryable_as_writer_contention() {
        assert!(transient_boundary(&failure(503, "domain_writer_busy")));
        assert!(!transient_boundary(&failure(403, "domain_writer_busy")));
        assert!(!transient_boundary(&failure(503, "shared_room_send_authority_changed")));
    }

    /// The store's word that the wall clock has ended a lease is a decision,
    /// and its run ends as at the lease's counted end. Read as a replaced
    /// scope, the code the store gave it before, it was waited out while the
    /// other clock still counted: after a sleep of the computer, a turn worked
    /// on for as long as its lease had been left before the sleep.
    #[test]
    fn a_lease_the_wall_clock_ended_is_not_waited_out() {
        let ended = store(hagency_store::Error::ActivityLeaseEnded);
        assert_eq!((ended.status, ended.code.as_str()), (409, "shared_execution_expired"));
        assert_eq!(ended.said(), crate::console::native::Said::Decided);
        assert!(!transient_boundary(&ended));
        // What a replaced scope refuses is still waited out.
        assert!(transient_boundary(&store(hagency_store::Error::LocalAuthority)));
    }

    /// A step under a lease that the wall clock has ended is refused before it
    /// asks the server anything, as a decision: it is not asked again, though
    /// the end the other clock counted is still far ahead. A lease the wall
    /// clock holds lets the step go on.
    #[test]
    fn a_step_under_a_lease_the_wall_clock_ended_is_not_taken() {
        let now = hagency_store::owner_resources::wall_clock_ms().unwrap();
        let until = Instant::now() + Duration::from_secs(100);
        let ended = wall_held("proof", until, now - 1).unwrap_err();
        assert_eq!((ended.status, ended.code.as_str()), (409, "shared_execution_expired"));
        assert_eq!(ended.said(), crate::console::native::Said::Decided);
        assert!(!transient_boundary(&ended));
        // An hour ahead: beyond any clock a test sets ahead.
        assert!(wall_held("proof", until, now + 3_600_000).is_ok());
    }

    /// An `Io` whose management has closed: a step that reaches it is
    /// refused with `shared_execution_closed` and asks no server.
    fn closed_io() -> Io {
        Io {
            permission: Permission::ExecuteManagedAgents,
            manager: std::sync::Weak::new(),
            host_gate: None,
            namespace: None,
            provider_gate: None,
            claude_discovery: Arc::default(),
            project_folders: Default::default(),
            crypto_fixture: None,
            project_root: None,
        }
    }

    /// A turn's own scope under a lease the wall clock has ended is refused
    /// before anything is asked: the refusal is the lease's end, not the
    /// closed management behind it.
    #[tokio::test]
    async fn a_turn_s_own_scope_under_a_lease_the_wall_clock_ended_is_not_asked_for() {
        let io = closed_io();
        let now = hagency_store::owner_resources::wall_clock_ms().unwrap();
        let until = Instant::now() + Duration::from_secs(100);
        let ended = io.bound_held(until, now - 1).await.err().unwrap();
        assert_eq!((ended.status, ended.code.as_str()), (409, "shared_execution_expired"));
        let asked = io.bound_held(until, now + 3_600_000).await.err().unwrap();
        assert_eq!((asked.status, asked.code.as_str()), (409, "shared_execution_closed"));
    }

    /// A step beside a Codex turn reads the lease in force when it is taken.
    /// The copy that the run controls keep was taken before the renewal, and
    /// reads the end that renewal granted, not the one it began under.
    #[test]
    fn a_step_beside_a_turn_reads_the_lease_a_renewal_granted() {
        let now = hagency_store::owner_resources::wall_clock_ms().unwrap();
        let until = Instant::now() + Duration::from_secs(100);
        let held = Held::new(until, now - 1);
        let beside = held.clone();
        let ended = beside.check("control").unwrap_err();
        assert_eq!((ended.status, ended.code.as_str()), (409, "shared_execution_expired"));
        held.renewed(until, now + 3_600_000);
        assert!(beside.check("control").is_ok());
    }

    /// A request that left and got no answer was not refused. The step is
    /// asked again, as it is after a check that could not be made: taken as a
    /// refusal, one broken exchange ended the run it belonged to.
    #[tokio::test(start_paused = true)]
    async fn an_exchange_without_an_answer_is_asked_again() {
        for code in ["server_unavailable", "storage_unavailable"] {
            let calls = AtomicU32::new(0);
            let until = Instant::now() + Duration::from_secs(20);
            let answered = settle(until, transient_boundary, || async { flaky(&calls, 2, code) })
                .await
                .unwrap();
            assert_eq!(answered, 3, "{code}");
        }
        // A refusal is returned at once.
        let calls = AtomicU32::new(0);
        let until = Instant::now() + Duration::from_secs(20);
        let error = settle(until, transient_boundary, || async {
            flaky(&calls, 2, "shared_activity_expired_or_changed")
        })
        .await
        .unwrap_err();
        assert_eq!(error.code, "shared_activity_expired_or_changed");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn transient_boundary_failure_is_retried_until_the_step_answers() {
        let calls = AtomicU32::new(0);
        let until = Instant::now() + Duration::from_secs(60);
        let value = settle(until, transient_boundary, || async {
            flaky(&calls, 2, "management_scope_changed")
        })
        .await
        .unwrap();
        assert_eq!(value, 3, "two refusals during a credential refresh, then the answer");
    }

    #[tokio::test(start_paused = true)]
    async fn other_failures_and_a_revoked_boundary_end_the_step_at_once() {
        let calls = AtomicU32::new(0);
        let until = Instant::now() + Duration::from_secs(60);
        let error = settle(until, transient_boundary, || async {
            flaky(&calls, 9, "shared_execution_closed")
        })
        .await
        .unwrap_err();
        assert_eq!(error.code, "shared_execution_closed");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // `Io::transient` answers false once hosting stopped or the package
        // was retired, whatever the code.
        let calls = AtomicU32::new(0);
        let error = settle(until, |_| false, || async {
            flaky(&calls, 9, "management_scope_changed")
        })
        .await
        .unwrap_err();
        assert_eq!(error.code, "management_scope_changed");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_failure_that_outlasts_the_lease_is_returned() {
        let calls = AtomicU32::new(0);
        // Less than one retry gap of lease left: no further attempt fits.
        let until = Instant::now() + Duration::from_secs(1);
        let error = settle(until, transient_boundary, || async {
            flaky(&calls, 9, "management_scope_changed")
        })
        .await
        .unwrap_err();
        assert_eq!(error.code, "management_scope_changed");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[cfg(test)]
mod claude_protocol_tests {
    use super::*;

    #[test]
    fn shared_claude_poll_opt_in_keeps_legacy_bodies_and_effect_routes() {
        fn poll(kind: usize) -> Action {
            match kind {
                0 => Action::Poll {
                    after: Some("original_cursor".into()),
                },
                1 => super::super::attempt_io::AttemptAction::Poll {
                    after: Some("original_cursor".into()),
                }
                .into(),
                2 => super::super::demand_io::DemandAction::Poll {
                    after: Some("original_cursor".into()),
                    retries: true,
                }
                .into(),
                #[cfg(feature = "shared-budget-execution")]
                3 => super::super::budget_io::AttemptAction::Poll {
                    after: Some("original_cursor".into()),
                }
                .into(),
                #[cfg(feature = "shared-budget-execution")]
                4 => super::super::budget_demand_io::BudgetDemandAction::Poll {
                    after: Some("original_cursor".into()),
                }
                .into(),
                _ => unreachable!(),
            }
        }
        let count = if cfg!(feature = "shared-budget-execution") {
            5
        } else {
            3
        };
        for kind in 0..count {
            if kind == 2 && !cfg!(feature = "shared-intake") {
                let error = ServerOperation::new(poll(kind))
                    .err()
                    .expect("demand polling requires shared intake");
                assert_eq!(error.code, "shared_intake_disabled");
                continue;
            }
            let legacy = ServerOperation::new(poll(kind)).unwrap().request().unwrap();
            let mut opted = ServerOperation::new(poll(kind)).unwrap();
            opted.1 = PollVersion::Claude;
            let opted = opted.request().unwrap();
            assert_eq!(legacy.0, opted.0);
            assert_eq!(legacy.2, opted.2);
            assert_eq!(format!("{}-v4", legacy.1), opted.1);
            let mut octos = ServerOperation::new(poll(kind)).unwrap();
            octos.1 = PollVersion::Octos;
            let octos = octos.request().unwrap();
            assert_eq!(legacy.0, octos.0);
            assert_eq!(legacy.2, octos.2);
            assert_eq!(format!("{}-v5", legacy.1), octos.1);
        }
        fn recovery() -> Action {
            Action::Recover {
                id: "immutable_original".into(),
                digest: "a".repeat(64),
                observation: Some("original_receipt".into()),
            }
        }
        let legacy = ServerOperation::new(recovery()).unwrap().request().unwrap();
        let mut opted = ServerOperation::new(recovery()).unwrap();
        opted.1 = PollVersion::Octos;
        assert_eq!(
            legacy,
            opted.request().unwrap(),
            "discovery cannot rewrite original recovery paths or IDs"
        );
    }
}
