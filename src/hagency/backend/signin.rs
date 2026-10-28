//! Fresh OAuth consent is bound at SDK publication, never inferred from a
//! restored session or from whichever access token exists at lazy owner startup.
use super::*;

#[derive(PartialEq)]
struct Proof {
    origin: String,
    owner: String,
    device: String,
    client_id: String,
    epoch: u64,
    revision: u64,
}

impl Proof {
    fn from_client(client: &matrix_sdk::Client, epoch: u64, revision: u64) -> Option<Self> {
        Some(Self {
            origin: client.homeserver().to_string(),
            owner: client.user_id()?.to_string(),
            device: client.device_id()?.to_string(),
            client_id: client.oauth().client_id()?.as_str().into(),
            epoch,
            revision,
        })
    }
}

struct Candidate {
    client: matrix_sdk::Client,
    proof: Proof,
}

#[derive(Default)]
struct Signins {
    pending: Option<Candidate>,
    published: Option<Proof>,
}

impl Signins {
    fn record(&mut self, client: &matrix_sdk::Client, epoch: u64, revision: u64) {
        self.pending = Proof::from_client(client, epoch, revision).map(|proof| Candidate {
            client: client.clone(),
            proof,
        });
    }

    fn publish(&mut self, client: Option<&matrix_sdk::Client>, epoch: u64, revision: u64) {
        // Every replacement, including removal or a same-account restore,
        // invalidates an earlier publication. A candidate is used only once.
        self.published = None;
        let Some(candidate) = self.pending.take() else {
            return;
        };
        let Some(client) = client else { return };
        let Some(proof) = Proof::from_client(client, epoch, revision) else {
            return;
        };
        // In the pinned SDK, http_client() borrows a field of ClientInner.
        // Clones share that Arc; the retained original keeps the address alive.
        // A separate restored client has another field, even with identical
        // credentials. No token copy or comparison races SDK token rotation.
        if candidate.proof == proof
            && std::ptr::eq(candidate.client.http_client(), client.http_client())
        {
            self.published = Some(proof);
        }
    }

    fn consume(&mut self, client: &matrix_sdk::Client, epoch: u64, revision: u64) -> bool {
        let published = self.published.take();
        published.is_some() && published == Proof::from_client(client, epoch, revision)
    }
}

static SIGNINS: Mutex<Signins> = Mutex::new(Signins {
    pending: None,
    published: None,
});

pub(super) fn consume(client: &matrix_sdk::Client, epoch: u64, revision: u64) -> bool {
    SIGNINS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .consume(client, epoch, revision)
}

impl Service {
    /// Called only after fresh OAuth authorization and SDK persistence succeed,
    /// before that exact SDK client is installed in the next global revision.
    pub(crate) fn record_explicit_oauth_signin(client: &matrix_sdk::Client, epoch: u64) {
        let Some(revision) = crate::sliding_sync::client_revision().checked_add(1) else {
            return;
        };
        if epoch != crate::account_session::epoch() {
            return;
        }
        SIGNINS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .record(client, epoch, revision);
    }

    /// ClientSlot calls this while its publication mutex is held, after advancing
    /// the revision and retiring the old owner, before exposing the new client.
    pub(crate) fn publish_explicit_oauth_signin(
        client: Option<&matrix_sdk::Client>,
        revision: u64,
    ) {
        SIGNINS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .publish(client, crate::account_session::epoch(), revision);
    }
}

#[cfg(test)]
mod tests;
