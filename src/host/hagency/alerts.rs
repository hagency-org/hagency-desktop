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
    /// The UI localizes alerts from `kind`/`params` and shows local, relative
    /// times from the `*Ms` numbers; the UTC labels are only added beside them.
    #[test]
    fn alert_display_keeps_kind_params_and_machine_times(){
        let params=json!({"ceilingTokens":1000,"drawnTokens":1200,"overByTokens":200,"committedTokens":1200,"measuredTokens":null,
            "percent":120,"period":"month","periodStartMs":1_790_812_800_000u64,"periodEndMs":1_793_491_200_000u64});
        let row=json!({"id":"agent_ceiling_overrun:r","kind":"ceiling_overrun","params":params,"summary":"English",
            "firstSeenMs":1_793_261_520_000u64,"lastSeenMs":1_793_261_520_000u64,"suppressUntilMs":null});
        let list=display(json!({"alerts":[row.clone()],"observedAtMs":1_793_261_600_000u64}));
        let shown=&list["alerts"][0];
        assert_eq!((&shown["kind"],&shown["params"]),(&json!("ceiling_overrun"),&params));
        assert_eq!(shown["lastSeenMs"],1_793_261_520_000u64);assert_eq!(shown["lastSeen"],"2026-10-29 08:12:00 UTC");
        assert_eq!(list["observedAtMs"],1_793_261_600_000u64);
        let detail=display(json!({"alert":row,"notes":[{"atMs":1_793_261_520_000u64}]}));
        assert_eq!((&detail["alert"]["kind"],&detail["alert"]["params"]),(&json!("ceiling_overrun"),&params));
        assert_eq!(detail["notes"][0]["atMs"],1_793_261_520_000u64);
    }
}
