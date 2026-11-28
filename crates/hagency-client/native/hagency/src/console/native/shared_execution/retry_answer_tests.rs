//! How the answer to a stop sent on the retry route is read: recorded there,
//! or reported on the plain route.
use super::*;
use super::super::demand_io::RetryReply;
use serde_json::json;

fn offer() -> SharedDemandOffer {
    serde_json::from_value(json!({
        "version": 1, "demandId": "demand_stop_route", "assignmentId": "assignment_stop_route",
        "eventId": "$stop_route:test", "eventDigest": "d".repeat(64), "controlRevision": 1,
        "receivedAtMs": 1_000, "predecessorCommandId": "attempt_stopped",
        "predecessorCommandDigest": "c".repeat(64),
    }))
    .unwrap()
}
fn stop(reason: Option<&str>) -> SharedAttemptProcessReceipt {
    SharedAttemptProcessReceipt {
        version: 2,
        observation_id: "obs_attempt_stopped".into(),
        command_id: "attempt_stopped".into(),
        command_digest: "c".repeat(64),
        assignment_id: "assignment_stop_route".into(),
        agent_reservation_id: "reservation_stop_route".into(),
        process_attempt: 2,
        original_runtime_digest: "b".repeat(64),
        ledger_revision: 7,
        process_digest: Some("e".repeat(64)),
        state: "stopped".into(),
        policy_digest: None,
        reason: reason.map(Into::into),
    }
}
/// The server's answer to the retry request: the receipt it recorded,
/// and the event as it stands after it.
fn answer(receipt: &SharedAttemptProcessReceipt, ack: &str, state: &str, retry: Option<u64>) -> Result<RetryReply, NativeError> {
    let o = offer();
    let d = o.digest().unwrap();
    let bound = (state == "bound").then(|| json!({
        "version": 1, "demandId": o.demand_id, "demandDigest": d, "controlRevision": 1,
        "assignmentId": o.assignment_id, "eventId": o.event_id, "receivedAtMs": o.received_at_ms,
        "runtimeCommandId": "attempt_stopped", "runtimeCommandDigest": "c".repeat(64),
        "workId": "work_stop_route", "workDigest": "f".repeat(64),
    }));
    Ok(serde_json::from_value(json!({
        "acknowledgement": {
            "observationId": receipt.observation_id, "receiptDigest": digest(receipt).unwrap(),
            "activity": null, "state": ack,
        },
        "demand": {
            "offer": o, "demandDigest": d, "state": state, "runtimeCommandId": "attempt_stopped",
            "binding": bound, "retry": retry,
        },
    }))
    .unwrap())
}

#[test]
fn a_stop_on_the_retry_route_is_recorded_there_or_reported_plainly() {
    let o = offer();
    let s = stop(None);
    // The server records the stop whatever it then does with the event:
    // a retry opened (its work was given back), the event still bound
    // (its turn began; nothing more is sent), or the event ended after
    // its wait for a retry (`retry_lapsed`).
    for (state, retry) in [("retry_offered", Some(1)), ("bound", None), ("head_changed", None)] {
        assert!(retry_answer(answer(&s, "stopped", state, retry), &s, &o).unwrap(), "{state}");
    }
    // A refusal records nothing: the stop goes on the plain route. So
    // does the 400 of a switched-off retry route, and the answer of a
    // server whose policy does not serve retries.
    for (status, code) in [(400, "invalid_shared_demand_retry"), (503, "shared_demand_retry_unavailable"), (409, "shared_demand_changed"), (409, "shared_demand_retry_changed")] {
        assert!(!retry_answer(Err(failure(status, code)), &s, &o).unwrap(), "{code}");
    }
    // No answer is no refusal: the same request is sent again.
    for (status, code) in [(503, "storage_unavailable"), (503, "server_unavailable"), (503, "management_capacity")] {
        assert_eq!(retry_answer(Err(failure(status, code)), &s, &o).unwrap_err().code, code);
    }
    assert_eq!(retry_answer(Err(failure(401, "sign_in_required")), &s, &o).unwrap_err().code, "sign_in_required");
    // A time-bound stop that the server's bound does not yet allow waits.
    let bound = stop(Some("uncertain_time_bound"));
    let pending = || Err(failure(409, "shared_attempt_timeout_pending"));
    assert!(retry_answer(pending(), &bound, &o).is_err());
    assert!(!retry_answer(pending(), &s, &o).unwrap());
    // An answer for another receipt, or one that left the process running,
    // is not this stop's record.
    assert!(retry_answer(answer(&stop(Some("x")), "stopped", "bound", None), &s, &o).is_err());
    assert!(retry_answer(answer(&s, "cleanup_pending", "bound", None), &s, &o).is_err());
}
