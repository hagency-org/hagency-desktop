//! Narrow resource alerts; neither triage nor suppression changes admission.
use super::{parse,Value};
use ::hagency::native_owner::{ResourceAlertQuery,ResourceAlertDetailQuery,TriageResourceAlert};
pub(super) const LIST:&str="hagency.alerts.list";
pub(super) const GET:&str="hagency.alerts.get";
pub(super) const TRIAGE:&str="hagency.alerts.triage";
pub(super) enum Call {List(ResourceAlertQuery),Get(ResourceAlertDetailQuery),Triage(TriageResourceAlert)}
pub(super) fn call(service:&str,args:Value)->Result<Option<Call>,String>{
    let invalid=||"invalid_management_arguments".to_owned();
    Ok(Some(match service {
        LIST=>{let q:ResourceAlertQuery=parse(args)?;q.validate().map_err(|_|invalid())?;Call::List(q)},
        GET=>{let q:ResourceAlertDetailQuery=parse(args)?;q.validate().map_err(|_|invalid())?;Call::Get(q)},
        TRIAGE=>{let q:TriageResourceAlert=parse(args)?;q.validate().map_err(|_|invalid())?;Call::Triage(q)},
        _=>return Ok(None),
    }))
}
fn date(row:&mut Value,key:&str,target:&str){
    row[target]=row.get(key).and_then(Value::as_u64).and_then(|v|i64::try_from(v).ok()).and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis).map(|v|Value::String(v.format("%Y-%m-%d %H:%M:%S UTC").to_string())).unwrap_or(Value::Null);
}
fn alert(row:&mut Value){date(row,"firstSeenMs","firstSeen");date(row,"lastSeenMs","lastSeen");date(row,"suppressUntilMs","suppressUntil");}
pub(super) fn display(mut value:Value)->Value{
    if let Some(rows)=value.get_mut("alerts").and_then(Value::as_array_mut){for row in rows{alert(row);}}
    if let Some(row)=value.get_mut("alert"){alert(row);}
    if let Some(rows)=value.get_mut("notes").and_then(Value::as_array_mut){for row in rows{date(row,"atMs","at");}}
    value
}
#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::json;
    #[test]
    fn resource_alert_host_contract_is_closed_and_cannot_supply_authority(){
        let input=json!({"commandId":"original","resourceId":"resource","id":"ceiling:original","expectedRevision":"a".repeat(64),"to":"assigned","note":"checked"});
        assert!(matches!(call(TRIAGE,input.clone()).unwrap(),Some(Call::Triage(_))));
        for field in ["actor","owner","profile","assignee","suppressUntilMs","path","token","allow","expectedRevisionNumber"]{let mut forged=input.clone();forged[field]=json!("forged");assert!(call(TRIAGE,forged).is_err(),"{field}");}
        assert!(call(LIST,json!({"resourceId":"resource","status":"unknown"})).is_err());
        assert!(call(GET,json!({"resourceId":"resource","id":"../private"})).is_err());
        assert!(call(LIST,json!({"resourceId":"resource"})).is_ok());
        for cap in [LIST,GET,TRIAGE]{assert!(octosense_app_contract::hagency::SERVICES.contains(&cap));assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&cap));assert!(octosense_app_contract::hagency::words(cap).is_some());}
        let view=display(json!({"alert":{"firstSeenMs":0,"lastSeenMs":1000,"suppressUntilMs":null},"notes":[{"atMs":2000}]}));assert_eq!(view["alert"]["lastSeen"],"1970-01-01 00:00:01 UTC");assert!(view["alert"]["suppressUntil"].is_null());assert_eq!(view["notes"][0]["at"],"1970-01-01 00:00:02 UTC");
    }
}
