//! Closed native stopped-task API. Execution identity is always host-derived.
use super::{parse, Value};
use ::hagency::native_owner::{ResolveStopped, ReviewStopped, StoppedListQuery, StoppedQuery};
pub(super) const LIST: &str = "hagency.stopped.list";
pub(super) const GET: &str = "hagency.stopped.get";
pub(super) const REVIEW: &str = "hagency.stopped.review";
pub(super) const RESOLVE: &str = "hagency.stopped.resolve";
pub(super) enum Call {
    List(StoppedListQuery), Get(StoppedQuery), Review(ReviewStopped), Resolve(ResolveStopped),
}
pub(super) fn call(service: &str, args: Value) -> Result<Option<Call>, String> {
    let invalid = |_| "invalid_management_arguments".to_owned();
    Ok(Some(match service {
        LIST => {let q:StoppedListQuery=parse(args)?;q.validate().map_err(invalid)?;Call::List(q)}
        GET => {let q:StoppedQuery=parse(args)?;q.validate().map_err(invalid)?;Call::Get(q)}
        REVIEW => {let q:ReviewStopped=parse(args)?;q.validate().map_err(invalid)?;Call::Review(q)}
        RESOLVE => {let q:ResolveStopped=parse(args)?;q.validate().map_err(invalid)?;Call::Resolve(q)}
        _ => return Ok(None),
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn stopped_services_reject_caller_execution_authority() {
        for (service,args) in [
            (LIST,json!({"resourceId":"r"})),
            (GET,json!({"resourceId":"r","engagementId":"e","dispatchId":"d"})),
            (REVIEW,json!({"resourceId":"r","engagementId":"e","dispatchId":"d","commandId":"review","expectedRevision":"a".repeat(64)})),
            (RESOLVE,json!({"resourceId":"r","engagementId":"e","dispatchId":"d","commandId":"decide","reviewCommandId":"review","reviewRevision":"a".repeat(64),"action":"keep_blocked","operatorNote":"Reviewed effects","continuationInstruction":null})),
        ] {
            assert!(call(service,args.clone()).unwrap().is_some());
            assert!(octosense_app_contract::hagency::SERVICES.contains(&service));
            assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&service));
            assert!(octosense_app_contract::hagency::words(service).is_some());
            for field in ["owner","profile","token","inspectionToken","inspectionId","replacement","sessionId","resources","force","path"] {
                let mut forged=args.clone();forged[field]=json!("forged");
                assert!(call(service,forged).is_err(),"{service}: {field}");
            }
        }
    }
    #[test]
    fn stopped_resolution_requires_review_and_action_specific_instruction() {
        let input=json!({"resourceId":"r","engagementId":"e","dispatchId":"d","commandId":"decide","reviewCommandId":"review","reviewRevision":"a".repeat(64),"action":"continue","operatorNote":"Reviewed effects","continuationInstruction":"Continue remaining work"});
        let Some(Call::Resolve(q))=call(RESOLVE,input.clone()).unwrap() else {panic!("expected resolution")};
        assert_eq!(q.review_command_id,"review");
        for (field,value) in [("continuationInstruction",Value::Null),("reviewRevision",json!("stale")),("dispatchId",json!("../other")),("action",json!("force")),("operatorNote",json!(""))] {
            let mut invalid=input.clone();invalid[field]=value;assert!(call(RESOLVE,invalid).is_err());
        }
        let mut invalid=input;invalid["action"]=json!("accept_completed");assert!(call(RESOLVE,invalid).is_err());
    }
}
