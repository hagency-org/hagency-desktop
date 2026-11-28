//! Closed schema17 B routes. Offers and bindings describe history, not launch authority.
use super::*;
use serde_json::{Value, json};
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Status {
    pub offer: SharedDemandOffer,
    pub demand_digest: String,
    pub state: String,
    pub runtime_command_id: Option<String>,
    pub binding: Option<SharedDemandBinding>,
    /// The latest retry of this event (ADR-193 amendment), once this
    /// executor asked for one.
    #[serde(default)]
    pub retry: Option<u64>,
}
impl Status {
    pub fn validate(&self) -> Result<(), NativeError> {
        if self.offer.digest().map_err(store)? != self.demand_digest
            || self
                .retry
                .is_some_and(|n| !(1..=MAX_DEMAND_RETRIES).contains(&n))
            || self.state == "retry_offered" && self.retry.is_none()
            || !matches!(
                self.state.as_str(),
                "offered"
                    | "start_committed"
                    | "retry_offered"
                    | "bound"
                    | "cancelled"
                    | "expired"
                    | "head_changed"
                    | "budget_blocked"
                    | "usage_unrepresentable"
                    | "blocked"
            )
        {
            return Err(invalid());
        }
        if let Some(id) = &self.runtime_command_id {
            super::super::key(id)?;
        }
        if let Some(b) = &self.binding
            && (b.version != 1
                || b.demand_id != self.offer.demand_id
                || b.demand_digest != self.demand_digest
                || b.assignment_id != self.offer.assignment_id
                || b.event_id != self.offer.event_id
                || b.received_at_ms != self.offer.received_at_ms
                || b.control_revision != self.offer.control_revision
                || self.runtime_command_id.as_ref() != Some(&b.runtime_command_id))
        {
            return Err(invalid());
        }
        if self.state == "bound" {
            self.binding.as_ref().ok_or_else(invalid)?.validate_offer(&self.offer).map_err(store)?;
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PollReply {
    pub demand: Option<Status>,
    pub next_after: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RecoverReply {
    pub demand: Option<Status>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct WorkBinding {
    pub offer: SharedDemandOffer,
    pub demand_digest: String,
    pub binding: SharedDemandBinding,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BindingReply {
    pub demand: Option<WorkBinding>,
}
/// The executor's not_started evidence for a demand's latest attempt, or
/// (retry v2) the stop of its claimed attempt, with its request to run the
/// same event again in place (ADR-193 amendment). For a stop the server
/// decides, by its record of the turn's start, whether the event runs again.
#[derive(Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RetryInput {
    pub demand_digest: String,
    pub expected_control_revision: u64,
    pub retry: u64,
    pub attempt_id: String,
    pub result: super::attempt_io::ResultInput,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RetryReply {
    pub acknowledgement: super::attempt_io::Acknowledgement,
    pub demand: Status,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BlockInput {
    pub demand_digest: String,
    pub expected_control_revision: u64,
    pub reason: String,
    pub budget: Option<SharedBudgetObservation>,
    /// Sent only to a server that advertises retries (ADR-193 amendment).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub retries: bool,
    /// Sent only to a server that lists retry v2: this executor reads a
    /// message given back at its start's fence as waiting again.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reoffers: bool,
}
pub(super) enum DemandAction {
    Discovery,
    Control {
        assignment: String,
    },
    Stop {
        assignment: String,
        input: SharedDemandStopInput,
    },
    /// `retries`: this executor runs retries and the server advertises them;
    /// only then may a status carry them (ADR-193 amendment). `reoffers`:
    /// the server lists retry v2, and a message given back at its start's
    /// fence is shown waiting again rather than ended.
    Poll {
        after: Option<String>,
        retries: bool,
        reoffers: bool,
    },
    Recover {
        id: String,
        digest: String,
        retries: bool,
        reoffers: bool,
    },
    Binding {
        work: String,
        work_digest: String,
        runtime_digest: String,
    },
    Block {
        id: String,
        input: BlockInput,
    },
    Retry {
        id: String,
        input: RetryInput,
    },
}
impl DemandAction {
    pub(super) fn validate(&self) -> Result<(), NativeError> {
        // Native context and engine negotiation share this read-only endpoint.
        // Enabling execution must not also enable intake mutations.
        if !cfg!(feature = "shared-intake")
            && !(cfg!(feature = "shared-execution") && matches!(self, Self::Discovery))
        {
            return Err(failure(409, "shared_intake_disabled"));
        }
        match self {
            Self::Discovery => {}
            Self::Control { assignment } => super::super::key(assignment)?,
            Self::Stop { assignment, input } => {
                super::super::key(assignment)?;
                super::super::key(&input.command_id)?;
                if input.expected_revision == 0
                    || input.expected_revision >= 9_007_199_254_740_991
                    || input.mode != "paused"
                {
                    return Err(invalid());
                }
            }
            Self::Poll { after, retries, reoffers } => {
                if let Some(id) = after {
                    super::super::key(id)?;
                }
                if *reoffers && !*retries {
                    return Err(invalid());
                }
            }
            Self::Recover { id, digest, retries, reoffers } => {
                super::super::key(id)?;
                hash(digest)?;
                if *reoffers && !*retries {
                    return Err(invalid());
                }
            }
            Self::Binding {
                work,
                work_digest,
                runtime_digest,
            } => {
                super::super::key(work)?;
                hash(work_digest)?;
                hash(runtime_digest)?;
            }
            Self::Block { id, input } => {
                super::super::key(id)?;
                hash(&input.demand_digest)?;
                if input.expected_control_revision == 0
                    || input.expected_control_revision > 9_007_199_254_740_991
                    || input.reoffers && !input.retries
                    || !matches!(
                        input.reason.as_str(),
                        "budget_blocked" | "usage_unrepresentable"
                    )
                {
                    return Err(invalid());
                }
            }
            Self::Retry { id, input } => {
                super::super::key(id)?;
                super::super::key(&input.attempt_id)?;
                hash(&input.demand_digest)?;
                hash(&input.result.command_digest)?;
                hash(&input.result.token)?;
                // not_started evidence (retry v1), or a claimed attempt's stop
                // (retry v2), begun or not: the server decides.
                if input.expected_control_revision == 0
                    || input.expected_control_revision > 9_007_199_254_740_991
                    || !(1..=MAX_DEMAND_RETRIES + 1).contains(&input.retry)
                    || !matches!(
                        input.result.receipt.state.as_str(),
                        "not_started" | "stopped"
                    )
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
    pub(super) fn body(self) -> (String, Value) {
        match self {
            Self::Discovery => (String::new(), Value::Null),
            Self::Control { assignment } => {
                (format!("assignments/{assignment}/control"), Value::Null)
            }
            Self::Stop { assignment, input } => {
                (format!("assignments/{assignment}/control"), json!(input))
            }
            Self::Poll { after, retries, reoffers } => (
                "demands/poll".into(),
                opted(json!({"after":after}), retries, reoffers),
            ),
            Self::Recover { id, digest, retries, reoffers } => (
                format!("demands/{id}/recover"),
                opted(json!({"demandDigest":digest}), retries, reoffers),
            ),
            Self::Binding {
                work,
                work_digest,
                runtime_digest,
            } => (
                format!("work/{work}/demand"),
                json!({"workDigest":work_digest,"runtimeCommandDigest":runtime_digest}),
            ),
            Self::Block { id, input } => (format!("demands/{id}/block"), json!(input)),
            Self::Retry { id, input } => (format!("demands/{id}/retry"), json!(input)),
        }
    }
}
/// A poll or recover body with the retry opt-ins this executor sends; each is
/// omitted when false, so the bytes for an older server are unchanged.
fn opted(mut body: Value, retries: bool, reoffers: bool) -> Value {
    if retries {
        body["retries"] = Value::Bool(true);
    }
    if reoffers {
        body["reoffers"] = Value::Bool(true);
    }
    body
}
impl From<DemandAction> for Action {
    fn from(v: DemandAction) -> Self {
        Self::Demand(Box::new(v))
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RunControl {
    pub assignment_id: String,
    pub revision: u64,
    pub mode: String,
    pub running_since_ms: Option<u64>,
    pub updated_at_ms: u64,
    pub origin: String,
}
impl RunControl {
    pub fn validate(&self, id: &str) -> Result<(), NativeError> {
        if self.assignment_id != id
            || self.revision == 0
            || self.revision >= 9_007_199_254_740_991
            || !matches!(self.mode.as_str(), "manual" | "running" | "paused")
            || self.updated_at_ms == 0
            || self.updated_at_ms > 9_007_199_254_740_991
            || self
                .running_since_ms
                .is_some_and(|v| v == 0 || v > self.updated_at_ms)
            || self.origin.len() > 128
        {
            return Err(invalid());
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ControlReply {
    pub control: RunControl,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StopReply {
    pub command_id: String,
    pub applied_revision: u64,
    pub control: RunControl,
}

// Discovery is extensible; only this bounded protocol-capability projection is
// consumed. It never substitutes for authenticated B route policy checks.
#[derive(Deserialize)]
pub(super) struct Discovery {
    pub capabilities: Vec<String>,
}
pub(super) async fn paired(b: &Bound) -> Result<bool, NativeError> {
    let d = b.demand_discovery().await?;
    if d.capabilities.len() > 64 || d.capabilities.iter().any(|c| c.len() > 128) {
        return Err(invalid());
    }
    Ok(d.capabilities
        .iter()
        .any(|c| c == "shared-durable-demand-v1"))
}
/// Whether the server retries an event whose attempt never started.
pub(super) async fn retries(b: &Bound) -> Result<bool, NativeError> {
    let d = b.demand_discovery().await?;
    if d.capabilities.len() > 64 || d.capabilities.iter().any(|c| c.len() > 128) {
        return Err(invalid());
    }
    Ok(d.capabilities.iter().any(|c| c == "shared-demand-retry-v1"))
}
/// Retry v2 in a bounded discovery answer: the server also takes back the
/// work of a stopped demand attempt whose turn it never recorded as begun,
/// and retries its event in place.
pub(super) fn retry_v2_in(d: &Discovery) -> Option<bool> {
    if d.capabilities.len() > 64 || d.capabilities.iter().any(|c| c.len() > 128) {
        return None;
    }
    let has = |name: &str| d.capabilities.iter().any(|c| c == name);
    Some(has("shared-demand-retry-v1") && has("shared-demand-retry-v2"))
}
/// Whether the server advertises retry v2 now.
pub(super) async fn retries_v2(b: &Bound) -> Result<bool, NativeError> {
    Ok(retry_protocol(b).await?.1)
}
/// Retry v1 and retry v2 as one bounded discovery answer lists them.
pub(super) async fn retry_protocol(b: &Bound) -> Result<(bool, bool), NativeError> {
    let d = b.demand_discovery().await?;
    let v2 = retry_v2_in(&d).ok_or_else(invalid)?;
    Ok((d.capabilities.iter().any(|c| c == "shared-demand-retry-v1"), v2))
}
pub(super) async fn require_pair(b: &Bound) -> Result<(), NativeError> {
    if !paired(b).await? {
        return Err(failure(409, "shared_intake_protocol_unavailable"));
    }
    Ok(())
}

#[cfg(all(test, feature = "shared-intake"))]
mod reoffer_tests {
    use super::*;

    #[test]
    fn retry_v2_opt_ins_go_out_only_when_listed_and_keep_older_bytes() {
        let body = |a: DemandAction| {
            a.validate().unwrap();
            a.body().1
        };
        let poll = |retries, reoffers| DemandAction::Poll { after: None, retries, reoffers };
        assert_eq!(body(poll(false, false)), json!({"after": null}));
        assert_eq!(body(poll(true, false)), json!({"after": null, "retries": true}));
        assert_eq!(body(poll(true, true)), json!({"after": null, "retries": true, "reoffers": true}));
        assert!(poll(false, true).validate().is_err(), "retry v2 is listed only beside v1");
        let digest = "a".repeat(64);
        let recover = |retries, reoffers| DemandAction::Recover { id: "demand".into(), digest: digest.clone(), retries, reoffers };
        assert_eq!(body(recover(true, false)), json!({"demandDigest": digest, "retries": true}));
        assert_eq!(body(recover(true, true)), json!({"demandDigest": digest, "retries": true, "reoffers": true}));
        let block = |retries, reoffers| BlockInput {
            demand_digest: digest.clone(),
            expected_control_revision: 1,
            reason: "budget_blocked".into(),
            budget: None,
            retries,
            reoffers,
        };
        assert_eq!(serde_json::to_value(block(true, false)).unwrap().get("reoffers"), None);
        assert_eq!(serde_json::to_value(block(true, true)).unwrap()["reoffers"], true);
        assert!(DemandAction::Block { id: "demand".into(), input: block(false, true) }.validate().is_err());
        let listed = |caps: &[&str]| retry_v2_in(&Discovery { capabilities: caps.iter().map(|c| (*c).to_owned()).collect() });
        assert_eq!(listed(&["shared-durable-demand-v1", "shared-demand-retry-v1"]), Some(false));
        assert_eq!(listed(&["shared-demand-retry-v2"]), Some(false));
        assert_eq!(listed(&["shared-demand-retry-v1", "shared-demand-retry-v2"]), Some(true));
        assert_eq!(listed(&[&"x".repeat(129)]), None);
    }
}
