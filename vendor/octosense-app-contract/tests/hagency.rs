use octosense_app_contract::{AppManifest, HostLimits, policy, hagency};
use serde_json::json;
#[test]
fn exact_hagency_services_require_explicit_version_and_never_imply_other_grants() {
    for service in hagency::SERVICES {
        let manifest=|requires| AppManifest::parse(&json!({"schema":1,"id":"test","name":"Test","version":"1","integrity":{"bundle_blake3":""},"capabilities":[service],"requires":requires}).to_string()).unwrap();
        let limits=HostLimits::default().with_require_signature(false);
        assert!(policy::resolve(&manifest(json!([])),&limits).is_err());
        let policy=policy::resolve(&manifest(json!([hagency::FEATURE])),&limits).unwrap();
        assert!(policy.allows(service));
        assert_eq!(policy.capabilities.len(),1);
        assert!(!policy.allows("net"));
        assert!(hagency::words(service).is_some());
    }
    for service in ["hagency","hagency.*","hagency.shell","hagency.http"] {
        let manifest=AppManifest::parse(&json!({"schema":1,"id":"test","name":"Test","version":"1","integrity":{"bundle_blake3":""},"capabilities":[service]}).to_string()).unwrap();
        assert!(policy::resolve(&manifest,&HostLimits::default().with_require_signature(false)).is_err());
    }
}
