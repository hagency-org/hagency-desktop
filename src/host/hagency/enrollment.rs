//! Native review fixes the exact account, model and declared monthly limit.
use super::*;
pub(crate) const ENROLL: &str = "hagency.accounts.enroll";
pub(crate) struct ResourceConsent {
    host: Host,
    lease: Lease,
    input: ::hagency::native_owner::NamespaceEnrollment,
    expires: Instant,
}
impl Host {
    pub fn resource_consent(&self,lease:Lease,args:Value)->Result<ResourceConsent,String>{
        let input: ::hagency::native_owner::NamespaceEnrollment=parse(args)?;
        input.validate().map_err(|_|"invalid_management_arguments".to_owned())?;
        let account=self.check(&lease)?;lease.authorize(&account,ENROLL,None)?;
        Ok(ResourceConsent{host:self.clone(),lease,input,expires:Instant::now()+Duration::from_secs(300)})
    }
}
impl ResourceConsent {
    pub fn account(&self)->&str {&self.lease.identity().account}
    pub fn details(&self)->String {
        enrollment_details(&self.input)
    }
    pub fn expired(&self)->bool {Instant::now()>=self.expires}
    pub async fn accept(self)->Result<Value,String>{
        if self.expired(){return Err("provider_confirmation_expired".into());}
        let account=self.host.check(&self.lease)?;self.lease.authorize(&account,ENROLL,None)?;
        let manager=self.host.manager(&self.lease).await?;
        self.host.check(&self.lease)?;self.lease.authorize(&account,ENROLL,None)?;
        let result=manager.enroll_account_resource(self.input,self.expires).await.map_err(|e|e.code).and_then(value);
        self.host.check(&self.lease).map_err(|_|"management_outcome_unknown".to_owned())?;
        self.lease.authorize(&account,ENROLL,None)?;
        rinx_miniapp_core::bounded_reply(result?)
    }
}
fn enrollment_details(input: &::hagency::native_owner::NamespaceEnrollment) -> String {
    format!("Codex · {}\n{} · {}\n{}: {}", input.account.account_id, input.model,
        input.reasoning.as_deref().unwrap_or("—"), crate::i18n::tr("Declared monthly token limit"),
        crate::utils::format_token_count(input.monthly_tokens))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enrollment_review_uses_millions_without_changing_the_request() {
        let input: ::hagency::native_owner::NamespaceEnrollment = parse(json!({
            "commandId":"original", "account":{"accountId":"account_selected", "expectedRevision":"a".repeat(64)},
            "model":"gpt-5.6-sol", "reasoning":"medium", "monthlyTokens":123456
        })).unwrap();
        input.validate().unwrap();
        let details = enrollment_details(&input);
        assert!(details.contains("0.123456 M"), "{details}");
        assert!(!details.contains(": 123456"));
        assert_eq!(input.monthly_tokens, 123456);
    }
    #[test]
    fn enrollment_input_excludes_identity_paths_and_approval_claims(){
        let valid=json!({"commandId":"original","account":{"accountId":"account_selected","expectedRevision":"a".repeat(64)},"model":"gpt-5.6-sol","reasoning":"medium","monthlyTokens":1000});
        let input: ::hagency::native_owner::NamespaceEnrollment=parse(valid.clone()).unwrap();input.validate().unwrap();
        for key in ["path","provider","quotaAccount","seatId","approved","token","url","executable","profile","owner"]{
            let mut bad=valid.clone();bad[key]=json!("forged");assert!(parse::<::hagency::native_owner::NamespaceEnrollment>(bad).is_err());
        }
        let authority=rinx_miniapp_core::SessionAuthority::default();
        let lease=authority.issue(InstanceId{app:octosense_app_contract::hagency::APP_ID.into(),account:"@owner:test".into(),room:None,generation:1},[ENROLL.to_owned()].into_iter().collect(),Default::default(),Instant::now()+Duration::from_secs(60));
        assert_eq!(permissions(&lease),vec![ManagementPermission::EnrollResources]);
        assert!(octosense_app_contract::hagency::SERVICES.contains(&ENROLL));
        assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&ENROLL));
        assert!(octosense_app_contract::hagency::words(ENROLL).is_some());
    }
}
