//! Native review retains the original account retirement request.
use super::*;
pub(crate) const RETIRE: &str = "hagency.accounts.retire";
pub(crate) struct RetirementConsent {
    host: Host,
    lease: Lease,
    input: ::hagency::native_owner::RetireAccount,
    expires: Instant,
}
impl Host {
    pub fn retirement_consent(&self,lease:Lease,args:Value)->Result<RetirementConsent,String>{
        let input: ::hagency::native_owner::RetireAccount=parse(args)?;
        input.validate().map_err(|_|"invalid_management_arguments".to_owned())?;
        let account=self.check(&lease)?;lease.authorize(&account,RETIRE,None)?;
        Ok(RetirementConsent{host:self.clone(),lease,input,expires:Instant::now()+Duration::from_secs(300)})
    }
}
impl RetirementConsent {
    pub fn account(&self)->&str {&self.lease.identity().account}
    pub fn details(&self)->String {
        format!("Codex · {}",self.input.account_id)
    }
    pub fn expired(&self)->bool {Instant::now()>=self.expires}
    pub async fn accept(self)->Result<Value,String>{
        if self.expired(){return Err("provider_confirmation_expired".into());}
        let account=self.host.check(&self.lease)?;self.lease.authorize(&account,RETIRE,None)?;
        let manager=self.host.manager(&self.lease).await?;
        self.host.check(&self.lease)?;self.lease.authorize(&account,RETIRE,None)?;
        let result=manager.retire_account(self.input,self.expires).await.map_err(|e|e.code).and_then(value);
        self.host.check(&self.lease).map_err(|_|"management_outcome_unknown".to_owned())?;
        self.lease.authorize(&account,RETIRE,None)?;
        rinx_miniapp_core::bounded_reply(result?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retirement_is_closed_and_requires_its_own_grant() {
        let valid=json!({"commandId":"original","accountId":"account_selected","expectedRevision":"a".repeat(64)});
        let input: ::hagency::native_owner::RetireAccount=parse(valid.clone()).unwrap();input.validate().unwrap();
        for key in ["path","provider","approved","token","url","executable","profile","owner","logout","deleteCredentials"] {
            let mut bad=valid.clone();bad[key]=json!("forged");assert!(parse::<::hagency::native_owner::RetireAccount>(bad).is_err());
        }
        let authority=rinx_miniapp_core::SessionAuthority::default();
        let lease=authority.issue(InstanceId{app:octosense_app_contract::hagency::APP_ID.into(),account:"@owner:test".into(),room:None,generation:1},[RETIRE.to_owned()].into_iter().collect(),Default::default(),Instant::now()+Duration::from_secs(60));
        assert_eq!(permissions(&lease),vec![ManagementPermission::RetireAccounts]);
        assert!(octosense_app_contract::hagency::SERVICES.contains(&RETIRE));
        assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&RETIRE));
        assert!(octosense_app_contract::hagency::words(RETIRE).is_some());
    }
}
