//! Token-free native commands routed through the existing authenticated service.
pub(super) mod allocation_transport;
mod managed_agent_matrix_transport;
mod managed_agent_transport;
mod managed_agent_settlement_transport;
pub use managed_agent_settlement_transport::{AgentSettlementProgress, AgentSettlementTransport};
#[cfg(feature="agent-budget")]
mod managed_agent_budget_transport;
#[cfg(feature="agent-budget")]
pub use managed_agent_budget_transport::{AgentBudgetProgress, AgentBudgetTransport};
mod managed_runtime;
mod managed_runtime_transport;
pub(in crate::console) mod shared_execution;
mod shutdown;
pub mod telemetry;
pub use telemetry::{
    AttachReport, ExecutionPath, OwnerTelemetry, ResourceTelemetry, RuntimeCapability, Support,
    TelemetryError, TelemetryParts, TelemetryStatus,
};
pub use managed_agent_matrix_transport::{ManagedAgentMatrixTransport, ManagedMatrixProgress};
pub use managed_agent_transport::{ManagedAgentProgress, ManagedAgentTransport};
pub use managed_runtime_transport::{ManagedRuntimeProgress, ManagedRuntimeTransport};
pub use shared_execution::{
    SharedSuccessorAssessment, SharedSuccessorBudgetReview, SharedSuccessorBlockReason, SharedSuccessorDescription,
    PreparedSharedSuccessor, SharedDemandPoll, SharedDemandProgress, SharedDemandResume,
    SharedDemandStopReceipt, SharedDemandStopResume, SharedExecutionProgress,
    SharedExecutionTransport, SharedSuccessorPage, SharedSuccessorResume, SharedSuccessorReview,
    SharedSuccessorSummary, SharedWorkRecoveryProgress,
};
mod configuration;
mod owner_grant;
pub use allocation_transport::{AllocationProgress, AllocationTransport, ResourceGrantView};
pub use owner_grant::{
    OwnerGrantAccess, OwnerGrantChoices, OwnerGrantClosure, OwnerGrantIntents, OwnerGrantProgress,
    OwnerGrantCancellationReview, OwnerGrantCancellationResult,
    OwnerGrantPublicationProgress, OwnerGrantPublicationResult, OwnerGrantPublicationReview,
    OwnerGrantPublicationState, OwnerGrantReview, OwnerGrantReviewContext, OwnerGrantReviewResult,
    OwnerGrantState, OwnerGrantSummary, OwnerGrantForm, OwnerGrantFloor,
    OwnerGrantSuccessionForm, OwnerGrantSuccessionKind, OwnerGrantSuccessionState,
    OwnerGrantSuccessionSummary,
};
mod matrix_creations;
pub(super) mod project_setup;
pub use matrix_creations::{
    MatrixCreation, MatrixCreationEntry, MatrixCreationInput, MatrixCreationKind,
    MatrixCreationList, MatrixCreationPhase, MatrixCreationReply, MatrixCreationResume,
};
pub use project_setup::{
    AssignAdministrator, BeginSetup, EligibleSetupAllocation, FailedCommand, ProjectAdministrator,
    ProjectReadiness, ProjectSetup, ProjectSetupTransport, RebaseSetup, SetupBinding,
    SetupCandidates, SetupProgress, StageProgress,
};
mod personal_agent_settings;
pub use personal_agent_settings::PersonalAgentQuery;
mod personal_agent_observation;
pub use personal_agent_observation::ObservePersonalSettings;
mod personal_agent_policy;
pub use personal_agent_policy::SavePersonalPolicy;
mod personal_agent_model;
pub use personal_agent_model::{SavePersonalModel, PreparePersonalWorkspace, PersonalModelView, PersonalModelChoice, PersonalModelChoices, PersonalModelSummary, PersonalModelReview};
mod personal_agent_rules;
pub use personal_agent_rules::PersonalRulesReview;
mod management;
mod resource_definitions;
pub use resource_definitions::PreparedResourceTemplate;
pub use hagency_store::management::{TemplateCreateInput,TemplateUpdateInput,TemplateDeleteInput,ResourceTemplatePage,ResourceTemplateDetail,ResourceTemplateResult};
mod operator_task_commands;
#[cfg(feature = "resource-adoption")]
mod operator_graph_commands;
#[cfg(feature = "resource-adoption")]
mod operator_graph_reads;
#[cfg(feature = "resource-adoption")]
mod operator_graph_recipients;
#[cfg(feature = "resource-adoption")]
pub use operator_graph_recipients::{OperatorGraphRecipientOptionsQuery, OperatorGraphRecipientObserve, OperatorGraphBindRecipient};
#[cfg(feature = "resource-adoption")]
pub use operator_graph_commands::{OperatorGraphCreate,OperatorGraphUpdate,OperatorGraphCancel,OperatorGraphNodeDraft,OperatorGraphPatchDraft,PreparedOperatorGraph,OperatorGraphResult};
#[cfg(feature = "resource-adoption")]
pub use crate::operator_graphs::current::{OperatorGraphQuery,OperatorGraphDetailQuery,OperatorGraphNodeQuery,OperatorGraphPage,OperatorGraphSummary,OperatorGraphDetail,OperatorGraphNode,OperatorGraphNodeSummary};
pub use operator_task_commands::PreparedOperatorTask;
pub use hagency_store::management::{TaskField,CreateOperatorTask,EditOperatorTask,OperatorTaskVersion,TransitionOperatorTask,CommentOperatorTask,OperatorTaskChange,OperatorTaskResult};
mod management_intents;
pub(super) mod management_server;
#[cfg(feature = "resource-adoption")]
pub use crate::operator_graphs::canonical::{OperatorGraphJournal, OperatorGraphChange, OperatorGraphNodeInput, OperatorGraphNodePatch, OperatorGraphSnapshot};
mod operator_tasks;
pub use management_server::agent_status::{
    AgentAssignmentStatus, AgentExecutionStatus, AgentMembershipStatus, AgentRuntimeStatus,
    AgentStage, AgentStatus,
};
pub use management_server::agents::{
    AGENT_ADMISSION_FEATURE, AgentDefinition, AgentRequest, AgentRequestContext, AgentRequestInput,
    AgentRequestReadiness,
};
#[cfg(feature = "resource-adoption")]
mod current_profile;
mod provider;
mod provider_identity;
#[cfg(test)]
mod reconciliation_sources;
#[cfg(feature = "resource-adoption")]
mod device_setup;
#[cfg(feature = "resource-adoption")]
pub use current_profile::{CurrentAllocationExecutor,CurrentProfileSelection,CurrentSharedExecutor};

#[cfg(all(feature = "resource-adoption", any(test, feature = "test-support")))]
pub use device_setup::{initialize_resource_activation_fixture, resource_activation_fixture_root};
#[cfg(feature = "resource-adoption")]
pub use device_setup::{DeviceSetupContext,DeviceSetupReview};

#[cfg(test)]
mod initialization_tests;
mod resource_name;
mod resource_edit;
pub use resource_edit::{CeilingAmendmentReview, LocalResourceEditReview};
mod resource_create;
pub use resource_create::ResourceConfigurationReview;
pub use hagency_store::{ProfileChange, CeilingChange};
mod resource_alerts;
mod approval_bindings;
mod tool_approvals;
mod execution_policy;
pub use hagency_store::management::{ExecutionPolicyListQuery, AgentExecutionPolicyPage, ExecutionPolicyQuery, ExecutionPolicyValue, PolicyApplication, AgentExecutionPolicy, EditExecutionPolicy};
pub use hagency_store::management::{ToolApprovalQuery, ToolApprovalLookup, ToolApproval, ToolApprovalSubject, ToolApprovalSnapshot, ToolApprovalPage, ToolGrantQuery, ToolGrantMode, ToolRuleDetails, ToolGrant, ToolGrantPage, RevokeToolGrant, RevokedToolGrant};
pub use hagency_store::management::{ResourceApprovalBinding, ResourceApprovalBindingPage, ResourceApprovalBindingQuery, RetireResourceApprovalBinding, RetiredResourceApprovalBinding};
mod alert_observer;
pub use hagency_store::management::{ResourceAlert, ResourceAlertPage, ResourceAlertQuery, ResourceAlertDetail, ResourceAlertDetailQuery, ResourceAlertNote, TriageResourceAlert};
mod account_preparation;
mod account_retirement;
mod account_login;
mod account_models;
mod account_enrollment;
mod claude_account;
mod octos_account;
pub use hagency_store::owner_resources::NamespaceEnrollment;
pub use account_login::AccountLoginAuthorization;
mod resource_onboarding;
use super::*;
use crate::owner_host::OwnerHost;
pub use hagency_store::management::{
    Permission as ManagementPermission, PublishResource,
    AccountInventory, AccountInventoryQuery, AccountSummary, PrepareAccount, AccountPreparation, RetireAccount, AccountRetirement,
    AccountLoginRequest, AccountLoginStatus, AccountModelQuery,
    RenameResource, EditLocalResource, CreateResourceConfiguration, ResourceDetail, ResourceDetailQuery, ResourceNameResult, ResourcePage,
    CeilingAmendment, CeilingAmendmentBlocker,
    ResourceQuery,
};
pub use hagency_store::management::{
    OperatorBoardQuery, OperatorTaskDetailQuery, OperatorTaskQuery,
};
pub use hagency_store::{CeilingAmendmentResult, ResourceConfigurationResult, ResourcePublicationResult};
pub use management::{ManagementPackage, NativeManagement};
pub use management_server::{AgentRunMode, ManagementCommand};
pub use resource_onboarding::ManagementRuntime;
use salvo::test::{ResponseExt, TestClient};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, Mutex as StdMutex, Weak},
};
use tokio::sync::Mutex;

/// The SDK remains the sole owner of OAuth refresh and revocation.
pub struct MatrixAccessToken {
    pub access_token: String,
    pub client_id: String,
}
impl std::fmt::Debug for MatrixAccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatrixAccessToken")
            .field("access_token", &"[redacted]")
            .field("client_id", &self.client_id)
            .finish()
    }
}
pub(in crate::console) fn sdk_source_failure(error: NativeError) -> &'static str {
    match error.code.as_str() {
        "matrix_account_changed" => "matrix_account_changed",
        "matrix_authorization_unavailable" => "matrix_authorization_unavailable",
        "matrix_oauth_sign_in_required" => "matrix_oauth_sign_in_required",
        "unsupported_hagency_issuer" => "unsupported_hagency_issuer",
        "matrix_account_mismatch" => "matrix_account_mismatch",
        "unsupported_hagency_server" => "unsupported_hagency_server",
        "unsupported_hagency_protocol" => "unsupported_hagency_protocol",
        "unsupported_hagency_capabilities" => "unsupported_hagency_capabilities",
        "invalid_hagency_metadata" => "invalid_hagency_metadata",
        "hagency_server_unavailable" => "hagency_server_unavailable",
        "hagency_server_rejected" => "hagency_server_rejected",
        _ => "matrix_authorization_required",
    }
}
/// Codes by which the SDK token source, or this owner's own closed
/// authorization service, reports that the enrolled Matrix account is no
/// longer the one in use. Only these retire the owner. A server refusal, an
/// unreadable answer, an unfinished registration or a command this profile
/// does not allow fails that one command and leaves local work running.
pub(in crate::console) fn matrix_source_lost(code: &str) -> bool {
    matches!(
        code,
        "matrix_account_changed"
            | "matrix_account_mismatch"
            | "matrix_oauth_sign_in_required"
            | "matrix_authorization_required"
            | "unsupported_hagency_issuer"
    )
}
pub trait MatrixTokenSource: Send + Sync {
    /// Trusted host provenance from a new explicit SDK sign-in. Restored SDK
    /// sessions return false; this cannot resurrect issued runtime custody.
    fn permits_explicit_reenrollment(&self) -> bool {
        false
    }
    /// Atomically consume the exact new-sign-in permission. The trusted host
    /// must keep this separate from account validity and return true at most
    /// once for that OAuth success. A read-only permission flag is insufficient.
    fn consume_explicit_reenrollment(&self) -> bool {
        false
    }
    /// Return the current SDK token, checking its account epoch before and after
    /// awaiting. Implementations must not call this native service recursively.
    fn access_token(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>> + Send + '_>,
    >;
    /// Ask the SDK refresh owner for a fresh grant before a short authorization
    /// window expires. No refresh credential crosses this interface.
    fn refresh_access_token(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>> + Send + '_>,
    > {
        self.access_token()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentAction {
    Pause,
    Resume,
    Retire,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BindingAction {
    SetThreadAutoReply(bool),
    Status,
    Pause,
    Resume,
    Leave,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProviderAction {
    UseLocal,
    Disconnect,
    Models,
    Status,
    Login,
    Cancel,
    Logout,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LocalAction {
    AgentModel,
    SaveAgentModel,
    AgentPolicy,
    SaveAgentPolicy,
    ResetAgentPolicy,
    Policy,
    SavePolicy,
    ResetPolicy,
    Runtime,
    RuntimeStart,
    RuntimeStop,
    StartAgentService,
    StopAgentService,
    AgentServiceStatus,
    Approvals,
    Decision { approval_id: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectCreationPolicy {
    pub default_allow: bool,
    pub allow: std::collections::BTreeSet<String>,
    pub deny: std::collections::BTreeSet<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum RoomCreationPolicy {
    InheritProject {
        deny: std::collections::BTreeSet<String>,
    },
    AllowList {
        allow: std::collections::BTreeSet<String>,
        deny: std::collections::BTreeSet<String>,
    },
    Disabled,
}
impl<'de> Deserialize<'de> for RoomCreationPolicy {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
        enum Strict {
            InheritProject {
                deny: std::collections::BTreeSet<String>,
            },
            AllowList {
                allow: std::collections::BTreeSet<String>,
                deny: std::collections::BTreeSet<String>,
            },
            Disabled {},
        }
        Ok(match Strict::deserialize(d)? {
            Strict::InheritProject { deny } => Self::InheritProject { deny },
            Strict::AllowList { allow, deny } => Self::AllowList { allow, deny },
            Strict::Disabled {} => Self::Disabled,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    AgentConfigurationStatus {
        agent_id: String,
    },
    ExecutionEnvironment {
        agent_id: String,
    },
    AgentConfiguration {
        agent_id: String,
        expected_generation: i64,
        idempotency_key: String,
        command: Box<Command>,
    },
    AgentCommands,
    ResumeAgentCommand {
        id: String,
    },
    ScopeServiceState {
        project_id: String,
        room_id: Option<String>,
    },
    SetProjectCreationPolicy {
        project_id: String,
        expected_revision: i64,
        policy: ProjectCreationPolicy,
    },
    SetRoomCreationPolicy {
        project_id: String,
        room_id: String,
        expected_revision: i64,
        policy: RoomCreationPolicy,
    },
    SetScopeServicePause {
        project_id: String,
        room_id: Option<String>,
        paused: bool,
    },
    Devices,
    AgentDetails {
        agent_id: String,
    },
    AssignAgentToCurrentDevice {
        agent_id: String,
        expected_generation: i64,
    },
    AgentOwnerDirectStatus {
        agent_id: String,
    },
    EnsureAgentOwnerDirect {
        agent_id: String,
    },
    AgentOwnerDirect {
        agent_id: String,
        room_id: String,
    },

    LoginStatus,
    BeginLogin,
    Logout,
    Projects,
    ProjectRooms {
        project_id: String,
    },
    SpaceCandidates {
        cursor: Option<String>,
    },
    RoomRoster {
        project_id: String,
        room_id: String,
    },
    AdoptProject {
        space_id: String,
        /// Opt-in shared project folder of a new Agent space (its name).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        project_folder: Option<String>,
    },
    AdoptRoom {
        project_id: String,
        room_id: String,
    },
    CreateMatrix {
        input: Value,
    },
    Creations,
    Creation {
        id: String,
    },
    ResumeCreation {
        id: String,
        room_id: Option<String>,
    },
    Agents,
    RoomAgents,
    Bindings {
        agent_id: String,
    },
    CreateAgent {
        input: Value,
    },
    BindAgent {
        agent_id: String,
        input: Value,
    },
    Agent {
        agent_id: String,
        action: AgentAction,
    },
    Binding {
        agent_id: String,
        binding_id: String,
        action: BindingAction,
    },
    Local {
        agent_id: String,
        action: LocalAction,
        input: Option<Value>,
        binding_id: Option<String>,
        requester: Option<String>,
    },
    PrepareAgentWorkspace {
        agent_id: String,
    },
    /// The tool requests of every local Agent running here that wait for
    /// this owner. A read; deciding one is `Local` with `Decision`.
    LocalApprovals,
    Provider {
        action: ProviderAction,
    },
    RuntimeProvider {
        runtime: String,
        action: ProviderAction,
    },
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Identity {
    pub origin: String,
    pub issuer: String,
    pub subject: String,
    pub owner: String,
}
#[derive(Debug, thiserror::Error)]
#[error("{code}")]
pub struct NativeError {
    pub status: u16,
    pub code: String,
}
fn failure(status: u16, code: &str) -> NativeError {
    NativeError {
        status,
        code: code.into(),
    }
}
/// What a failure says about the step that returned it.
///
/// This is the one table of it. Whoever decides to wait, to send again, to
/// take another route or to give up reads the failure here, so that a
/// condition that passes is the same condition everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Said {
    /// The step could not be asked: nothing of it was sent or kept.
    NotAsked,
    /// The scope the step ran under no longer stands, as around a credential
    /// refresh. Under a fresh scope the step can be asked again; if the
    /// authority is really gone, taking that scope is what says so.
    ScopeReplaced,
    /// The step was sent and no answer came back, or the server answered
    /// that it could not serve the request. Whether it took effect is not
    /// known.
    NoAnswer,
    /// The step was decided: refused, or failed in a way that is its own. Its
    /// code says which, and whoever ran the step reads it.
    Decided,
}
impl NativeError {
    pub(crate) fn said(&self) -> Said {
        match self.code.as_str() {
            "management_capacity"
            | "management_unavailable"
            | "matrix_authorization_unavailable" => Said::NotAsked,
            "domain_writer_busy" if self.status == 503 => Said::NotAsked,
            "management_scope_changed"
            | "management_access_required"
            | "owner_authorization_required" => Said::ScopeReplaced,
            "server_unavailable"
            | "storage_unavailable"
            | "server_request_failed"
            | "invalid_server_response"
            | "management_outcome_unknown" => Said::NoAnswer,
            // What this host makes itself with a status of 500 or more. Each
            // is the failure of a step of its own and is handled where it is
            // made. They are named so that the rule below is about the
            // server only; `every_failure_made_on_the_pooled_path_is_read`
            // fails when one is made and not named.
            "invalid_allocation_response"
            | "management_drain_unavailable"
            | "management_state_unavailable"
            | "native_random_unavailable"
            | "shared_crypto_device_unavailable"
            | "shared_encrypted_send_pending"
            | "shared_history_unavailable"
            | "shared_message_delivery_pending"
            | "shared_message_storage_unavailable"
            | "shared_native_history_unavailable"
            | "shared_question_answers_unavailable"
            | "shared_question_device_pending"
            | "shared_question_event_unavailable"
            | "shared_question_send_pending"
            | "shared_room_keys_pending"
            | "shared_room_send_authority_changed" => Said::Decided,
            // What the server answers while its policy does not serve a
            // route. It gives these a status of 503, and they are decisions
            // all the same: asked again, it says the same. This is how an
            // operator ends pooled work (a renewal is refused and the run
            // closes with its usage), and how a host learns that a route is
            // not offered and takes the other one.
            //
            // The list is the server's, whole: every code it returns as
            // unavailable right after a predicate of its management policy,
            // and the one for a server with no such policy. It is kept by
            // hand, as the server is another repository. One that is missing
            // is read by the rule below: asked again for a bounded while,
            // never taken for a refusal that was not made.
            "agent_admission_unavailable"
            | "agent_budget_issuance_unavailable"
            | "agent_executor_unavailable"
            | "agent_matrix_provisioning_unavailable"
            | "agent_runtime_provisioning_unavailable"
            | "calendar_grants_unavailable"
            | "management_not_configured"
            | "parent_settlement_unavailable"
            | "pool_agents_unavailable"
            | "pool_grant_succession_unavailable"
            | "pool_publication_first_unavailable"
            | "project_reservations_unavailable"
            | "project_setup_unavailable"
            | "shared_budget_execution_issuance_unavailable"
            | "shared_demand_issuance_unavailable"
            | "shared_demand_retry_unavailable"
            | "shared_durable_intake_unavailable"
            | "shared_encryption_unavailable"
            | "shared_execution_issuance_unavailable"
            | "shared_execution_unavailable"
            | "shared_successor_issuance_unavailable"
            | "shared_thread_context_issuance_unavailable" => Said::Decided,
            // Any other failure with such a status is the server's: it did
            // not serve the request. Its codes are its own and change with
            // it (its database, a restart, a full queue), so they are read
            // by their status.
            _ if self.status >= 500 => Said::NoAnswer,
            _ => Said::Decided,
        }
    }
    /// Whether the step may be asked again: it was not decided. Asking again
    /// is right only for a step whose exact replay is harmless: one that reads,
    /// or one the server answers a second time with what it already did.
    pub(crate) fn passes(&self) -> bool {
        self.said() != Said::Decided
    }
}
#[cfg(test)]
mod said_tests {
    use super::{Said, failure};
    #[test]
    fn a_failure_is_read_in_one_place() {
        for code in ["management_capacity", "management_unavailable", "matrix_authorization_unavailable"] {
            assert_eq!(failure(503, code).said(), Said::NotAsked, "{code}");
        }
        assert_eq!(failure(503, "domain_writer_busy").said(), Said::NotAsked);
        assert_eq!(failure(403, "domain_writer_busy").said(), Said::Decided);
        for code in ["management_scope_changed", "management_access_required", "owner_authorization_required"] {
            assert_eq!(failure(401, code).said(), Said::ScopeReplaced, "{code}");
        }
        // A request that was sent and got no answer is not a refusal either.
        for code in ["server_unavailable", "storage_unavailable", "server_request_failed", "management_outcome_unknown"] {
            assert_eq!(failure(503, code).said(), Said::NoAnswer, "{code}");
            assert!(failure(503, code).passes(), "{code}");
        }
        // Nor is an answer of the server that it could not serve the request,
        // whatever it calls the reason.
        for (status, code) in [(503, "database_unavailable"), (503, "worker_stopping"), (500, "internal")] {
            assert_eq!(failure(status, code).said(), Said::NoAnswer, "{code}");
        }
        // What the server answers while its policy does not serve a route is
        // a decision, though it gives it that status: asked again, it says
        // the same.
        for code in [
            "agent_admission_unavailable",
            "agent_budget_issuance_unavailable",
            "agent_executor_unavailable",
            "agent_matrix_provisioning_unavailable",
            "agent_runtime_provisioning_unavailable",
            "calendar_grants_unavailable",
            "management_not_configured",
            "parent_settlement_unavailable",
            "pool_agents_unavailable",
            "pool_grant_succession_unavailable",
            "pool_publication_first_unavailable",
            "project_reservations_unavailable",
            "project_setup_unavailable",
            "shared_budget_execution_issuance_unavailable",
            "shared_demand_issuance_unavailable",
            "shared_demand_retry_unavailable",
            "shared_durable_intake_unavailable",
            "shared_encryption_unavailable",
            "shared_execution_issuance_unavailable",
            "shared_execution_unavailable",
            "shared_successor_issuance_unavailable",
            "shared_thread_context_issuance_unavailable",
        ] {
            assert_eq!(failure(503, code).said(), Said::Decided, "{code}");
            assert!(!failure(503, code).passes(), "{code}");
        }
        // An answer with no code in it was not made by the server's own
        // handlers, which give every refusal one. Whatever its status, it
        // decides nothing.
        assert_eq!(failure(409, "server_request_failed").said(), Said::NoAnswer);
        // The same status on a failure this host makes for a step of its own
        // says nothing: an authority that changed is an answer.
        assert_eq!(failure(503, "shared_room_send_authority_changed").said(), Said::Decided);
        assert_eq!(failure(401, "sign_in_required").said(), Said::Decided);
        assert_eq!(failure(409, "a_code_of_the_server").said(), Said::Decided);
        assert!(!failure(409, "shared_execution_expired").passes());
    }

    /// What a failure says when it is not decided, by code and status as this
    /// host makes it on the pooled path, with the three that are made
    /// elsewhere (by the server, or where a request is sent).
    const READ: &[(&str, u16, Said)] = &[
        ("domain_writer_busy", 403, Said::Decided),
        ("domain_writer_busy", 503, Said::NotAsked),
        ("invalid_server_response", 502, Said::NoAnswer),
        ("management_access_required", 401, Said::ScopeReplaced),
        ("management_capacity", 503, Said::NotAsked),
        ("management_outcome_unknown", 409, Said::NoAnswer),
        ("management_scope_changed", 401, Said::ScopeReplaced),
        ("management_scope_changed", 409, Said::ScopeReplaced),
        ("management_unavailable", 503, Said::NotAsked),
        ("matrix_authorization_unavailable", 503, Said::NotAsked),
        ("owner_authorization_required", 401, Said::ScopeReplaced),
        ("server_request_failed", 500, Said::NoAnswer),
        ("server_unavailable", 503, Said::NoAnswer),
        ("storage_unavailable", 503, Said::NoAnswer),
    ];
    /// Codes that decide their step, with whatever status. Each has its own
    /// handling where it is made.
    const DECIDED: &[&str] = &[
        "agent_executor_ledger_retired", "allocation_acknowledgement_conflict",
        "allocation_enrollment_changed", "allocation_receipt_conflict",
        "allocation_ticket_expired", "call_timed_out", "device_setup_required",
        "empty_device_already_active", "encrypted_approval_device_changed",
        "grant_publication_receipt_missing", "invalid_allocation_response",
        "invalid_management_arguments", "invalid_management_revision",
        "management_agent_control_local_stop_pending", "management_backing_changed",
        "management_change_cancelled", "management_drain_outcome_unknown",
        "management_drain_unavailable", "management_model_unqualified", "management_not_found",
        "management_permission_required", "management_profile_mismatch",
        "management_resource_edit_changed", "management_resource_edit_not_available",
        "management_resource_in_use", "management_resource_name_changed",
        "management_resource_source_changed", "management_revision_conflict",
        "management_saved_command_not_found", "management_state_unavailable",
        "management_stopped_not_resolvable", "management_stopped_review_expired",
        "management_task_revision_changed", "management_workspace_active",
        "management_workspace_inspection_required", "matrix_account_mismatch",
        "native_allocation_receipt_missing", "native_context_protocol_required",
        "native_grant_not_publishable", "native_grant_receipt_conflict",
        "native_grant_receipt_required", "native_random_unavailable", "not_found",
        "owner_grant_predecessor_required", "owner_grant_publication_history_unavailable",
        "owner_local_custody_required", "owner_stop_local_custody_required",
        "provider_account_changed", "provider_identity_unavailable",
        "question_device_unavailable", "shared_budget_protocol_unavailable",
        "shared_callback_capacity", "shared_claude_container_unavailable",
        "shared_claude_sandbox_unsupported", "shared_claude_turn_policy_invalid",
        "shared_claude_version_unsupported", "shared_claude_workspace_unavailable",
        "shared_crypto_device_unavailable", "shared_crypto_storage_unavailable",
        "shared_demand_changed", "shared_demand_original_missing",
        "shared_encrypted_send_pending", "shared_execution_activity_missing",
        "shared_execution_closed", "shared_execution_disabled",
        "shared_execution_enrollment_changed", "shared_execution_event_unknown",
        "shared_execution_expired", "shared_execution_identity_failed",
        "shared_execution_initialize_failed", "shared_execution_launch_unavailable",
        "shared_execution_matrix_unavailable", "shared_execution_namespace_unavailable",
        "shared_execution_outcome_unknown", "shared_execution_resource_changed",
        "shared_execution_thread_unknown", "shared_execution_ticket_expired",
        "shared_file_response_failed", "shared_file_tool_unavailable",
        "shared_history_response_failed", "shared_history_tool_unavailable",
        "shared_history_unavailable", "shared_intake_disabled",
        "shared_intake_protocol_unavailable", "shared_message_delivery_pending",
        "shared_message_storage_unavailable", "shared_native_history_unavailable",
        "shared_octos_duration_reached", "shared_octos_shell_cleanup_unknown",
        "shared_question_answer_failed", "shared_question_answers_unavailable",
        "shared_question_device_pending", "shared_question_device_unavailable",
        "shared_question_event_unavailable", "shared_question_send_pending",
        "shared_room_keys_pending", "shared_room_send_authority_changed",
        "shared_run_control_unknown", "shared_successor_capacity_unavailable",
        "shared_successor_original_missing", "shared_successor_preparation_changed",
        "shared_successors_disabled",
    ];

    /// Every `<status>, "<code>"` in the sources of the pooled path: what
    /// `failure(..)` is given there, directly or as a pair.
    fn made_on_the_pooled_path() -> std::collections::BTreeSet<(u16, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/console/native");
        let mut files = vec![root.join("allocation_transport.rs"), root.join("management.rs")];
        let mut directories = vec![root.join("shared_execution"), root.join("allocation_transport")];
        while let Some(directory) = directories.pop() {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    directories.push(path);
                } else {
                    files.push(path);
                }
            }
        }
        let mut made = std::collections::BTreeSet::new();
        for file in files {
            let name = file.file_name().unwrap().to_str().unwrap();
            if !name.ends_with(".rs")
                || name.ends_with("tests.rs")
                || file.components().any(|part| part.as_os_str() == "tests")
            {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap();
            let bytes = text.as_bytes();
            for at in 0..bytes.len().saturating_sub(3) {
                let digits = &bytes[at..at + 3];
                if !digits.iter().all(u8::is_ascii_digit)
                    || (at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_'))
                {
                    continue;
                }
                let rest = text[at + 3..].trim_start();
                let Some(rest) = rest.strip_prefix(',') else { continue };
                let Some(rest) = rest.trim_start().strip_prefix('"') else { continue };
                let Some((code, _)) = rest.split_once('"') else { continue };
                if !code.is_empty()
                    && code.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                {
                    made.insert((text[at..at + 3].parse().unwrap(), code.to_owned()));
                }
            }
        }
        made
    }

    /// Every failure this host makes on the pooled path is listed above with
    /// what it says. A code that is added without being read here fails this
    /// test, so that nothing is decided, or passes, by being left out.
    #[test]
    fn every_failure_made_on_the_pooled_path_is_read() {
        let made = made_on_the_pooled_path();
        assert!(made.len() > 100, "the scan found the pooled path: {}", made.len());
        let mut unread = Vec::new();
        for (status, code) in &made {
            let listed = READ
                .iter()
                .find(|(read, at, _)| read == code && at == status)
                .map(|(_, _, said)| *said)
                .or_else(|| DECIDED.contains(&code.as_str()).then_some(Said::Decided));
            match listed {
                Some(said) => assert_eq!(failure(*status, code).said(), said, "{status} {code}"),
                None => unread.push((status, code)),
            }
        }
        assert!(unread.is_empty(), "made on the pooled path and not read: {unread:?}");
        for (code, status, said) in READ {
            assert_eq!(failure(*status, code).said(), *said, "{status} {code}");
            assert!(!DECIDED.contains(code), "{code} is listed twice");
        }
        // A listed code that is no longer made is taken off the list.
        for code in DECIDED {
            assert!(made.iter().any(|(_, made)| made == code), "{code} is not made any more");
        }
    }
}
/// The one rule for an Agent space's shared project folder name, the same as
/// the server's: 1 to 64 characters, no control characters, none of `/`, `\` or
/// `:`, no leading `.`, and no leading or trailing whitespace.
pub fn valid_project_folder_name(name: &str) -> bool {
    (1..=64).contains(&name.chars().count())
        && !name.starts_with('.')
        && !name.starts_with(char::is_whitespace)
        && !name.ends_with(char::is_whitespace)
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '\0'))
}
fn key(id: &str) -> Result<(), NativeError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(failure(400, "invalid_arguments"));
    }
    Ok(())
}
fn segment(id: &str) -> Result<String, NativeError> {
    if id.len() > 255
        || !id.starts_with('!')
        || !id.contains(':')
        || id
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || ['/', '\\', '?', '#', '%'].contains(&c))
    {
        return Err(failure(400, "invalid_arguments"));
    }
    Ok(percent_encoding::utf8_percent_encode(id, percent_encoding::NON_ALPHANUMERIC).to_string())
}
fn project_scope(project: &str, room: Option<&str>) -> Result<String, NativeError> {
    key(project)?;
    match room {
        Some(room) => Ok(format!("{project}/rooms/{}", segment(room)?)),
        None => Ok(project.into()),
    }
}
fn request(command: Command) -> Result<(&'static str, String, Option<Value>), NativeError> {
    let root = "/console/api";
    let mut body = None;
    let (method, path) = match command {
        Command::AgentConfigurationStatus { .. }
        | Command::ExecutionEnvironment { .. }
        | Command::AgentConfiguration { .. } => {
            return Err(failure(400, "configuration_operation_unsupported"));
        }
        Command::LoginStatus => ("GET", "/console/server-login".into()),
        Command::Logout => ("POST", "/console/server-login/logout".into()),
        Command::BeginLogin => return Err(failure(400, "login_requires_native_context")),
        Command::Projects => ("GET", format!("{root}/owner-projects")),
        Command::AgentCommands => ("GET", format!("{root}/owned-agents/commands")),
        Command::ResumeAgentCommand { id } => {
            key(&id)?;
            ("POST", format!("{root}/owned-agents/commands/{id}/resume"))
        }
        Command::ScopeServiceState {
            project_id,
            room_id,
        } => {
            let scope = project_scope(&project_id, room_id.as_deref())?;
            (
                "GET",
                format!("{root}/owner-projects/{scope}/service-state"),
            )
        }
        Command::SetProjectCreationPolicy {
            project_id,
            expected_revision,
            policy,
        } => {
            key(&project_id)?;
            if expected_revision < 0 {
                return Err(failure(400, "invalid_arguments"));
            }
            body = Some(json!({"expectedRevision":expected_revision,"policy":policy}));
            (
                "PUT",
                format!("{root}/owner-projects/{project_id}/creation-policy"),
            )
        }
        Command::SetRoomCreationPolicy {
            project_id,
            room_id,
            expected_revision,
            policy,
        } => {
            let scope = project_scope(&project_id, Some(&room_id))?;
            if expected_revision < 0 {
                return Err(failure(400, "invalid_arguments"));
            }
            body = Some(json!({"expectedRevision":expected_revision,"policy":policy}));
            (
                "PUT",
                format!("{root}/owner-projects/{scope}/creation-policy"),
            )
        }
        Command::SetScopeServicePause {
            project_id,
            room_id,
            paused,
        } => {
            let scope = project_scope(&project_id, room_id.as_deref())?;
            let suffix = if paused {
                "pause-service"
            } else {
                "clear-service-pause"
            };
            ("POST", format!("{root}/owner-projects/{scope}/{suffix}"))
        }
        Command::Devices => ("GET", format!("{root}/owned-agents/devices")),
        Command::LocalApprovals => ("GET", format!("{root}/owned-agents/pending-approvals")),
        Command::AgentDetails { agent_id } => {
            key(&agent_id)?;
            ("GET", format!("{root}/owned-agents/{agent_id}"))
        }
        Command::AssignAgentToCurrentDevice {
            agent_id,
            expected_generation,
        } => {
            key(&agent_id)?;
            if expected_generation < 0 {
                return Err(NativeError {
                    status: 400,
                    code: "invalid_arguments".into(),
                });
            }
            body = Some(json!({"expectedGeneration":expected_generation}));
            (
                "PUT",
                format!("{root}/owned-agents/{agent_id}/execution-device"),
            )
        }
        Command::AgentOwnerDirectStatus { agent_id } => {
            key(&agent_id)?;
            (
                "GET",
                format!("{root}/owned-agents/{agent_id}/owner-direct"),
            )
        }
        Command::EnsureAgentOwnerDirect { agent_id } => {
            key(&agent_id)?;
            (
                "POST",
                format!("{root}/owned-agents/{agent_id}/owner-direct/ensure"),
            )
        }
        Command::AgentOwnerDirect { agent_id, room_id } => {
            key(&agent_id)?;
            segment(&room_id)?;
            body = Some(json!({"roomId":room_id}));
            (
                "POST",
                format!("{root}/owned-agents/{agent_id}/owner-direct"),
            )
        }
        Command::ProjectRooms { project_id } => {
            key(&project_id)?;
            ("GET", format!("{root}/owner-projects/{project_id}/rooms"))
        }
        Command::RoomRoster {
            project_id,
            room_id,
        } => {
            key(&project_id)?;
            (
                "GET",
                format!(
                    "{root}/owner-projects/{project_id}/rooms/{}/agents",
                    segment(&room_id)?
                ),
            )
        }
        Command::SpaceCandidates { cursor } => {
            let mut p = format!("{root}/owner-projects/space-candidates");
            if let Some(c) = cursor {
                p.push_str(&format!("?cursor={}", segment(&c)?));
            }
            ("GET", p)
        }
        Command::AdoptProject {
            space_id,
            project_folder,
        } => {
            segment(&space_id)?;
            let mut adopt = json!({"spaceId":space_id});
            if let Some(folder) = project_folder {
                if !valid_project_folder_name(&folder) {
                    return Err(failure(400, "invalid_project_folder"));
                }
                adopt["projectFolder"] = json!(folder);
            }
            body = Some(adopt);
            ("POST", format!("{root}/owner-projects"))
        }
        Command::AdoptRoom {
            project_id,
            room_id,
        } => {
            key(&project_id)?;
            segment(&room_id)?;
            body = Some(json!({"roomId":room_id}));
            ("POST", format!("{root}/owner-projects/{project_id}/rooms"))
        }
        Command::CreateMatrix { .. }
        | Command::Creations
        | Command::Creation { .. }
        | Command::ResumeCreation { .. } => {
            return Err(failure(500, "native_operation_requires_direct_dispatch"));
        }
        Command::Agents => ("GET", format!("{root}/owned-agents")),
        Command::RoomAgents => ("GET", format!("{root}/owned-agents/room-candidates")),
        Command::CreateAgent { input } => {
            body = Some(input);
            ("POST", format!("{root}/owned-agents"))
        }
        Command::Bindings { agent_id } => {
            key(&agent_id)?;
            ("GET", format!("{root}/owned-agents/{agent_id}/bindings"))
        }
        Command::BindAgent { agent_id, input } => {
            key(&agent_id)?;
            body = Some(input);
            ("POST", format!("{root}/owned-agents/{agent_id}/bindings"))
        }
        Command::Agent { agent_id, action } => {
            key(&agent_id)?;
            let (m, s) = match action {
                AgentAction::Pause => ("POST", "/pause"),
                AgentAction::Resume => ("POST", "/resume"),
                AgentAction::Retire => ("DELETE", ""),
            };
            (m, format!("{root}/owned-agents/{agent_id}{s}"))
        }
        Command::Binding {
            agent_id,
            binding_id,
            action,
        } => {
            key(&agent_id)?;
            key(&binding_id)?;
            let (m, s) = match action {
                BindingAction::SetThreadAutoReply(enabled) => {
                    body = Some(serde_json::json!({"threadAutoReply":enabled}));
                    ("PUT", "/reply-policy")
                }
                BindingAction::Status => ("GET", ""),
                BindingAction::Pause => ("POST", "/pause"),
                BindingAction::Resume => ("POST", "/resume"),
                BindingAction::Leave => ("DELETE", ""),
            };
            (
                m,
                format!("{root}/owned-agents/{agent_id}/bindings/{binding_id}{s}"),
            )
        }
        Command::PrepareAgentWorkspace { agent_id } => {
            key(&agent_id)?;
            (
                "POST",
                format!("{root}/owner-provider/workspaces/{agent_id}"),
            )
        }
        Command::RuntimeProvider { runtime, action } => {
            if !matches!(runtime.as_str(), "claude" | "octos") {
                return Err(failure(400, "invalid_arguments"));
            }
            let (method, suffix) = match action {
                ProviderAction::Status => ("GET", ""),
                ProviderAction::Models => ("GET", "/models"),
                ProviderAction::UseLocal => ("POST", "/use-local"),
                ProviderAction::Disconnect => ("POST", "/disconnect"),
                _ => return Err(failure(400, "local_provider_login_managed_locally")),
            };
            (
                method,
                format!("{root}/owner-provider/runtimes/{runtime}{suffix}"),
            )
        }
        Command::Provider { action } => {
            let (m, s) = match action {
                ProviderAction::UseLocal => ("POST", "/use-local"),
                ProviderAction::Disconnect => ("POST", "/disconnect"),
                ProviderAction::Models => ("GET", "/models"),
                ProviderAction::Status => ("GET", ""),
                ProviderAction::Login => ("POST", "/login"),
                ProviderAction::Cancel => ("POST", "/cancel"),
                ProviderAction::Logout => ("POST", "/logout"),
            };
            (m, format!("{root}/owner-provider{s}"))
        }
        Command::Local {
            agent_id,
            action,
            input,
            binding_id,
            requester,
        } => {
            key(&agent_id)?;
            if matches!(
                action,
                LocalAction::AgentModel
                    | LocalAction::SaveAgentModel
                    | LocalAction::AgentPolicy
                    | LocalAction::SaveAgentPolicy
                    | LocalAction::ResetAgentPolicy
                    | LocalAction::StartAgentService
                    | LocalAction::StopAgentService
                    | LocalAction::AgentServiceStatus
            ) && (binding_id.is_some() || requester.is_some())
            {
                return Err(failure(400, "invalid_arguments"));
            }

            body = input;
            let (m, s) = match action {
                LocalAction::AgentModel => ("GET", "agent-model-profile".into()),
                LocalAction::SaveAgentModel => ("PUT", "agent-model-profile".into()),
                LocalAction::AgentPolicy => ("GET", "agent-policy".into()),
                LocalAction::SaveAgentPolicy => ("PUT", "agent-policy".into()),
                LocalAction::ResetAgentPolicy => ("DELETE", "agent-policy".into()),
                LocalAction::Policy => ("GET", "local-policy".into()),
                LocalAction::SavePolicy => ("PUT", "local-policy".into()),
                LocalAction::ResetPolicy => ("DELETE", "local-policy".into()),
                LocalAction::Runtime => ("GET", "runtime".into()),
                LocalAction::RuntimeStart => ("POST", "runtime/start".into()),
                LocalAction::RuntimeStop => ("POST", "runtime/stop".into()),
                LocalAction::StartAgentService => ("POST", "runtime/service/start".into()),
                LocalAction::StopAgentService => ("POST", "runtime/service/stop".into()),
                LocalAction::AgentServiceStatus => ("GET", "runtime/service".into()),
                LocalAction::Approvals => ("GET", "runtime/approvals".into()),
                LocalAction::Decision { approval_id } => {
                    key(&approval_id)?;
                    ("POST", format!("runtime/approvals/{approval_id}/decision"))
                }
            };
            let mut p = format!("{root}/owned-agents/{agent_id}/{s}");
            if m == "GET" && s == "local-policy" {
                let b = binding_id.ok_or(failure(400, "invalid_arguments"))?;
                key(&b)?;
                let r = requester.ok_or(failure(400, "invalid_arguments"))?;
                let mut url = reqwest::Url::parse("http://local.invalid/").unwrap();
                url.query_pairs_mut()
                    .append_pair("bindingId", &b)
                    .append_pair("requester", &r);
                p.push('?');
                p.push_str(url.query().unwrap());
            }
            (m, p)
        }
    };
    Ok((method, path, body))
}
struct Inner {
    host: OwnerHost,
    #[cfg(feature = "resource-adoption")]
    current_profile: std::sync::OnceLock<String>,
    service: salvo::Service,
    address: String,
    expected_origin: String,
    expected_owner: String,
    cookie: Mutex<String>,
    nonce: Mutex<Option<String>>,
    calls: Mutex<()>,
    matrix_sync: Mutex<()>,
    configuration_stopped: std::sync::atomic::AtomicBool,
    configuration_tasks: StdMutex<Vec<super::lifecycle::RetainedTask>>,
    configuration_custody: super::lifecycle::TaskCustody,
    callback_task: StdMutex<Option<tokio::task::JoinHandle<()>>>,
    matrix_source: StdMutex<Option<Arc<dyn MatrixTokenSource>>>,
    management: management::Registry,
    account_logins: account_login::Manager,
    managed_runtime: managed_runtime::Manager,
    shared_execution: shared_execution::Manager,
}
impl Drop for Inner {
    fn drop(&mut self) {
        self.account_logins.close();
        self.shared_execution.close();
        self.managed_runtime.close();
        self.management.retire();
        self.host.console.retire();
        if let Ok(tasks) = self.configuration_tasks.get_mut() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
        if let Ok(task) = self.callback_task.get_mut()
            && let Some(task) = task.take()
        {
            task.abort();
        }
    }
}
/// A native owner service. Neither credentials nor its private local grants are
/// returned in command responses. SDK-authenticated hosts open no listener;
/// standalone browser authentication listens only for its OAuth callback.
#[derive(Clone)]
pub struct NativeOwner(Arc<Inner>);
#[cfg(feature = "resource-adoption")]
mod server_administration;
#[derive(Clone, Copy, PartialEq, Eq)]
enum LoginMode {
    Browser,
    MatrixSdk,
}
enum MatrixStartup {
    Configuration,
    #[cfg(feature = "resource-adoption")]
    Current {
        registry: hagency_store::owner_resources::OwnerResourceRegistry,
        initialization_id: String,
    },
}
impl NativeOwner {
    #[cfg(unix)]
    pub(crate) fn set_hosting_admission(&self, admission: Arc<super::hosting::Admission>) {
        *self.0.host.console.0.hosting.lock().unwrap_or_else(|p| p.into_inner()) = admission;
    }
    /// Only for state a current profile must never create: the regular
    /// management journal. Local execution does not use this check.
    fn require_regular_mode(&self) -> Result<(), NativeError> {
        #[cfg(feature = "resource-adoption")]
        if self.0.current_profile.get().is_some() {
            return Err(failure(403, "source_current_profile_only"));
        }
        Ok(())
    }
    /// Open a current profile under fresh SDK and native owner-root authority.
    /// Canonical resource ACLs and original current command receipts remain bound.
    #[cfg(feature = "resource-adoption")]
    #[allow(clippy::too_many_arguments)]
    pub async fn open_current_profile_with_matrix_guarded(
        state: &Path,
        origin: &str,
        owner: &str,
        source: Arc<dyn MatrixTokenSource>,
        registry: hagency_store::owner_resources::OwnerResourceRegistry,
        initialization_id: String,
        guard: impl FnOnce(&NativeOwner) -> Result<(), NativeError> + Send,
    ) -> Result<Self, NativeError> {
        key(&initialization_id)?;
        Self::open_matrix_authorized(
            state,
            origin,
            owner,
            source,
            guard,
            MatrixStartup::Current {
                registry,
                initialization_id,
            },
        )
        .await
    }
    pub async fn open(state: &Path, origin: &str, owner: &str) -> Result<Self, NativeError> {
        Self::open_inner(state, origin, owner, LoginMode::Browser).await
    }
    async fn open_inner(
        state: &Path,
        origin: &str,
        owner: &str,
        login_mode: LoginMode,
    ) -> Result<Self, NativeError> {
        let origin = server_login::origin(origin)
            .map_err(|e| failure(400, e))?
            .to_string();
        if !owner.starts_with('@') || !owner.contains(':') || owner.len() > 255 {
            return Err(failure(400, "invalid_owner_identity"));
        }
        // SDK mode must not create another login endpoint. The temporary
        // callback belongs only to standalone browser authorization.
        let (host, acceptor) = if login_mode == LoginMode::Browser {
            let acceptor = salvo::conn::TcpListener::new("127.0.0.1:0")
                .try_bind()
                .await
                .map_err(|_| failure(503, "native_callback_unavailable"))?;
            let address = acceptor
                .local_addr()
                .map_err(|_| failure(503, "native_callback_unavailable"))?;
            let host = OwnerHost::open_native(state, address)
                .map_err(|_| failure(409, "native_owner_state_unavailable"))?;
            (host, Some(acceptor))
        } else {
            let host = OwnerHost::open_embedded(state)
                .map_err(|_| failure(409, "native_owner_state_unavailable"))?;
            (host, None)
        };
        let address = host.authority.clone();
        *host
            .console
            .0
            .server_login
            .native_expected
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))? =
            Some((origin.clone(), owner.into()));
        let ticket = host
            .console
            .owner_access_ticket()
            .map_err(|_| failure(503, "native_state_unavailable"))?;
        let service = salvo::Service::new(host.clone().router());
        let mut grant = TestClient::post(format!("http://{address}/console/session"))
            .add_header("host", address.to_string(), true)
            .add_header("sec-fetch-site", "same-origin", true)
            .add_header("origin", format!("http://{address}"), true)
            .json(&json!({"ticket":ticket}))
            .send(&service)
            .await;
        if !grant.status_code.unwrap_or_default().is_success() {
            return Err(failure(503, "native_access_unavailable"));
        }
        let cookie = grant
            .headers()
            .get_all("set-cookie")
            .iter()
            .find_map(|v| {
                v.to_str()
                    .ok()
                    .filter(|s| s.starts_with("hagency_console="))
                    .map(|s| s.split(';').next().unwrap().to_owned())
            })
            .ok_or(failure(503, "native_access_unavailable"))?;
        let native = Self(Arc::new(Inner {
            host,
            #[cfg(feature = "resource-adoption")]
            current_profile: std::sync::OnceLock::new(),
            service,
            address,
            expected_origin: origin,
            expected_owner: owner.into(),
            cookie: Mutex::new(cookie),
            nonce: Mutex::new(None),
            calls: Mutex::new(()),
            matrix_sync: Mutex::new(()),
            configuration_stopped: std::sync::atomic::AtomicBool::new(false),
            configuration_tasks: StdMutex::new(Vec::new()),
            configuration_custody: Default::default(),
            account_logins: Default::default(),
            callback_task: StdMutex::new(None),
            matrix_source: StdMutex::new(None),
            management: management::Registry::default(),
            managed_runtime: managed_runtime::Manager::default(),
            shared_execution: shared_execution::Manager::default(),
        }));
        if let Some(acceptor) = acceptor {
            let router = Router::new()
                .hoop(Callback(Arc::downgrade(&native.0)))
                .push(Router::with_path("console/server-login/callback").get(callback));
            let task = tokio::spawn(async move {
                salvo::Server::new(acceptor).serve(router).await;
            });
            *native.0.callback_task.lock().unwrap() = Some(task);
        }
        // Consume any bootstrap response without exposing a grant to the caller.
        let _ = grant.take_json::<Value>().await;
        // A crashed shared child may have left a plaintext native copy. Sweep
        // existing profile roots before configuration or execution can start;
        // restoring an encrypted session still requires fresh scope authority.
        if let Some(root) = native.0.host.console.0.server_login.state_directory() {
            native.0.shared_execution.sweep_claude(&root)?;
        }
        if login_mode == LoginMode::Browser {
            configuration::start(&native);
        }
        Ok(native)
    }
    /// Compatibility constructor for hosts without an account-transition owner
    /// slot. Transition-capable hosts must use `open_with_matrix_guarded`.
    pub async fn open_with_matrix(
        state: &Path,
        origin: &str,
        owner: &str,
        source: Arc<dyn MatrixTokenSource>,
    ) -> Result<Self, NativeError> {
        Self::open_with_matrix_guarded(state, origin, owner, source, |_| Ok(())).await
    }
    /// Transition-capable hosts must register this owner in their synchronous
    /// retirement boundary before SDK authorization or background work starts.
    /// The guard is native-only, invoked once, and must not block or perform IO.
    pub async fn open_with_matrix_guarded(
        state: &Path,
        origin: &str,
        owner: &str,
        source: Arc<dyn MatrixTokenSource>,
        guard: impl FnOnce(&NativeOwner) -> Result<(), NativeError> + Send,
    ) -> Result<Self, NativeError> {
        Self::open_matrix_authorized(
            state,
            origin,
            owner,
            source,
            guard,
            MatrixStartup::Configuration,
        )
        .await
    }
    async fn open_matrix_authorized(
        state: &Path,
        origin: &str,
        owner: &str,
        source: Arc<dyn MatrixTokenSource>,
        guard: impl FnOnce(&NativeOwner) -> Result<(), NativeError> + Send,
        startup: MatrixStartup,
    ) -> Result<Self, NativeError> {
        let native = Self::open_inner(state, origin, owner, LoginMode::MatrixSdk).await?;
        #[cfg(feature = "resource-adoption")]
        if let MatrixStartup::Current { initialization_id, .. } = &startup
        {
            native
                .0
                .current_profile
                .set(initialization_id.clone())
                .map_err(|_| failure(409, "current_profile_scope_changed"))?;
        }
        if let Err(error) = guard(&native) {
            let _ = native.suspend().await;
            return Err(error);
        }
        *native
            .0
            .matrix_source
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))? = Some(source.clone());
        match native
            .0
            .host
            .console
            .0
            .server_login
            .authorize_matrix(
                &native.0.host.console,
                &native.0.expected_origin,
                &native.0.expected_owner,
                source,
            )
            .await
        {
            Ok(cookie) => {
                *native.0.cookie.lock().await = format!("hagency_console={cookie}");
                if native
                    .0
                    .configuration_stopped
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    let _ = native.suspend().await;
                    return Err(failure(401, "matrix_account_changed"));
                }
                match startup {
                    MatrixStartup::Configuration => configuration::start(&native),
                    #[cfg(feature = "resource-adoption")]
                    MatrixStartup::Current {
                        registry,
                        initialization_id,
                    } => {
                        if let Err(error) = native
                            .initialize_current_management(&registry, &initialization_id)
                            .await
                        {
                            let _ = native.suspend().await;
                            return Err(error);
                        }
                        // Local Agents of this owner are configured through
                        // the same device worker as without pools.
                        configuration::start(&native);
                    }
                }
                if native
                    .0
                    .configuration_stopped
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    let _ = native.suspend().await;
                    return Err(failure(401, "matrix_account_changed"));
                }
                #[cfg(feature = "resource-adoption")]
                let observe = native.0.current_profile.get().is_none() || native.0.management.current_ready().await;
                #[cfg(not(feature = "resource-adoption"))]
                let observe = true;
                if observe { alert_observer::start(&native); }
                Ok(native)
            }
            Err(e) => {
                let _ = native.suspend().await;
                Err(failure(
                    if e.ends_with("unavailable") { 503 } else { 401 },
                    e,
                ))
            }
        }
    }
    /// Local Agents run beside pools: a current profile syncs for its local
    /// execution under the same rule as for its management, once that
    /// management is installed.
    async fn sync_matrix(&self) -> Result<(), NativeError> {
        self.sync_management_matrix().await
    }
    async fn sync_management_matrix(&self) -> Result<(), NativeError> {
        #[cfg(feature = "resource-adoption")]
        if self.0.current_profile.get().is_some() && !self.0.management.current_ready().await {
            return Err(failure(403, "source_current_profile_only"));
        }
        self.sync_matrix_source().await
    }
    async fn sync_matrix_source(&self) -> Result<(), NativeError> {
        let _guard = self.0.matrix_sync.lock().await;
        let source = self
            .0
            .matrix_source
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))?
            .clone();
        if let Some(source) = source {
            let cookie = self.0.cookie.lock().await.clone();
            let raw = cookie
                .strip_prefix("hagency_console=")
                .ok_or(failure(401, "owner_authorization_required"))?;
            let next = match self
                .0
                .host
                .console
                .0
                .server_login
                .renew_matrix(&self.0.host.console, raw)
                .await
            {
                Ok(next) => next,
                Err(super::Error::Unavailable) => {
                    return Err(failure(503, "matrix_authorization_unavailable"));
                }
                Err(_) => Some(
                    self.0
                        .host
                        .console
                        .0
                        .server_login
                        .authorize_matrix(
                            &self.0.host.console,
                            &self.0.expected_origin,
                            &self.0.expected_owner,
                            source.clone(),
                        )
                        .await
                        .map_err(|e| {
                            failure(if e.ends_with("unavailable") { 503 } else { 401 }, e)
                        })?,
                ),
            };
            if let Some(next) = next {
                *self.0.cookie.lock().await = format!("hagency_console={next}");
            }
        }
        Ok(())
    }
    #[cfg(test)]
    pub(in crate::console) fn test_console(&self) -> &super::Console {
        &self.0.host.console
    }
    #[cfg(test)]
    pub(in crate::console) fn has_callback_listener(&self) -> bool {
        self.0.callback_task.lock().unwrap().is_some()
    }
    pub async fn identity(&self) -> Result<Identity, NativeError> {
        let d = self
            .0
            .host
            .console
            .authorized_device()
            .await
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        if d.origin() != self.0.expected_origin || d.owner_mxid() != self.0.expected_owner {
            return Err(failure(401, "matrix_account_mismatch"));
        }
        Ok(Identity {
            origin: d.origin().into(),
            issuer: d.issuer().into(),
            subject: d.subject().into(),
            owner: d.owner_mxid().into(),
        })
    }
    /// Observation for hosting dashboards: shared-execution tasks this owner
    /// currently retains. `None` when the count cannot be read.
    pub fn active_shared_executions(&self) -> Option<u64> {
        self.0.shared_execution.live()
    }
    /// Owner telemetry handle (usage ledger and provider quota windows) fed
    /// by this owner's verified shared-execution observations. Attach
    /// persistence with [`OwnerTelemetry::attach_store`].
    pub fn telemetry(&self) -> OwnerTelemetry {
        let telemetry = self.0.shared_execution.telemetry();
        *self.0.host.console.0.dashboard_telemetry.lock().unwrap_or_else(|e| e.into_inner()) = Some(telemetry.clone());
        telemetry
    }
    /// The existing public OAuth registration, for an independent daemon login.
    /// Does not return tokens, acquire execution, or change native device IDs.
    pub fn matrix_resume_client_id(state: &Path, origin: &str, owner: &str) -> Result<String, NativeError> {
        super::server_login::ServerLogin::resume_client_id(state, origin, owner)
            .map_err(|code| failure(409, code))
    }
    /// True once this owner has retired its local capabilities, by its host or
    /// by itself after a Matrix source loss. It never becomes false again; a
    /// host that sees it without having closed the owner must release it.
    pub fn is_retired(&self) -> bool {
        self.0
            .configuration_stopped
            .load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn retire(&self) {
        self.0.account_logins.close();
        self.0.shared_execution.close();
        self.0.managed_runtime.close();
        self.0.management.retire();
        self.0.host.console.retire();
        self.0
            .configuration_stopped
            .store(true, std::sync::atomic::Ordering::Release);
        if let Ok(tasks) = self.0.configuration_tasks.lock() {
            let current = tokio::task::try_id();
            for task in tasks.iter() {
                if current != Some(task.id()) {
                    task.abort();
                }
            }
        }
    }
    /// Recover only explicitly enabled bindings from this exact owner's current
    /// assigned device. Each request re-enters the normal start controller; no
    /// lease, tool permission or unknown model execution is replayed here.
    pub async fn restore_runtime_intents(&self) -> Result<Value, NativeError> {
        let intents = {
            let _guard = self.0.calls.lock().await;
            self.sync_matrix().await?;
            self.0
                .host
                .console
                .0
                .owned_runtime
                .recovery_intents(&self.0.host.console)
                .await
                .map_err(|_| failure(409, "runtime_recovery_state_unavailable"))?
        };
        let mut results = Vec::new();
        for intent in intents {
            let input = intent.start_input();
            let result = self
                .execute(Command::Local {
                    agent_id: intent.agent_id.clone(),
                    action: if intent.binding_id == "*" {
                        LocalAction::StartAgentService
                    } else {
                        LocalAction::RuntimeStart
                    },
                    input: Some(if intent.binding_id == "*" {
                        let mut input = input;
                        input.as_object_mut().unwrap().remove("bindingId");
                        input
                    } else {
                        input
                    }),
                    binding_id: if intent.binding_id == "*" {
                        None
                    } else {
                        Some(intent.binding_id.clone())
                    },
                    requester: None,
                })
                .await;
            results.push(match result {
                Ok(value) => json!({"agentId":intent.agent_id,"bindingId":intent.binding_id,"restored":true,"runtime":value["runtime"]}),
                Err(error) => json!({"agentId":intent.agent_id,"bindingId":intent.binding_id,"restored":false,"error":error.code}),
            });
        }
        Ok(json!({"recoveries":results}))
    }
    pub async fn execute(&self, command: Command) -> Result<Value, NativeError> {
        self.admit_owner_command(&command).await?;
        if configuration::is_wrapper(&command) {
            return configuration::remote(self, command).await;
        }
        let _guard = self.0.calls.lock().await;
        self.execute_locked(command).await
    }
    /// Per-controller admission without retiring the shared owner. A queued
    /// call rechecks its lease after acquiring the ordinary owner command lock.
    pub async fn execute_guarded(&self, command: Command, admitted: Arc<dyn Fn() -> bool + Send + Sync>) -> Result<Value, NativeError> {
        self.admit_owner_command(&command).await?;
        if configuration::is_wrapper(&command) {
            if !admitted() { return Err(failure(401, "management_scope_changed")); }
            return configuration::remote(self, command).await;
        }
        let _guard = self.0.calls.lock().await;
        if !admitted() { return Err(failure(401, "management_scope_changed")); }
        self.execute_admitted_locked(command, Some(&admitted)).await
    }
    async fn admit_owner_command(&self, command: &Command) -> Result<bool, NativeError> {
        #[cfg(feature = "resource-adoption")]
        if self.0.current_profile.get().is_some() {
            // An owner who hosts pools still owns local Agents: both command
            // sets pass, under the current profile's stricter authorization.
            if !(server_administration::allows(command)
                || server_administration::allows_local(command))
                || !self.0.management.current_ready().await
            {
                return Err(failure(403, "source_current_profile_only"));
            }
            return Ok(true);
        }
        let _ = command;
        Ok(false)
    }
    async fn execute_locked(&self, command: Command) -> Result<Value, NativeError> {
        self.execute_admitted_locked(command, None).await
    }
    async fn check_command_matrix_source(&self) -> Result<(), NativeError> {
        let cookie = self.0.cookie.lock().await.clone();
        let raw = cookie
            .strip_prefix("hagency_console=")
            .ok_or(failure(401, "owner_authorization_required"))?;
        self.0
            .host
            .console
            .0
            .server_login
            .check_matrix_source(raw)
            .await
            .map_err(|code| failure(if code.ends_with("unavailable") { 503 } else { 401 }, code))
    }
    async fn command_matrix_error(&self, error: NativeError, stage: &'static str) -> NativeError {
        if matrix_source_lost(&error.code) {
            // A source/account transition retires local execution before the
            // next await. Only explicit logout withdraws the saved session.
            tracing::warn!(
                stage,
                code = %error.code,
                "Matrix account is no longer current; retiring local execution"
            );
            let _ = self.suspend().await;
        } else {
            // Everything else says nothing about the account: the server was
            // unreachable or refused, its answer was unreadable, a device
            // registration is unfinished. Retiring here would stop every Agent
            // until the host restarts. The command fails; the next one retries.
            tracing::warn!(
                stage,
                code = %error.code,
                "Matrix authorization failed for this command; preserving recovery state"
            );
        }
        error
    }
    async fn execute_admitted_locked(&self, command: Command, admitted: Option<&Arc<dyn Fn() -> bool + Send + Sync>>) -> Result<Value, NativeError> {
        let server_administration = self.admit_owner_command(&command).await?;
        let matrix_managed = self
            .0
            .matrix_source
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))?
            .is_some();
        if matrix_managed && matches!(command, Command::Logout) {
            self.shutdown_checked().await?;
            return Ok(
                json!({"state":"signed_out","deviceAuthorized":false,"transportOnline":false}),
            );
        }
        if matrix_managed && matches!(command, Command::BeginLogin) {
            return Err(failure(400, "sdk_owns_matrix_login"));
        }
        if matrix_managed && !matches!(command, Command::Logout) {
            let authorization = async {
                if server_administration {
                    self.sync_management_matrix().await?;
                    // Renewal can reuse a cached server grant. The SDK's current
                    // account remains authority for each administration request.
                    self.check_command_matrix_source().await
                } else {
                    self.sync_matrix().await
                }
            }.await;
            if let Err(error) = authorization {
                return Err(self.command_matrix_error(error, "sdk_pre_request").await);
            }
        }
        // Authorization may await the SDK. Recheck custody and the originating
        // controller before dispatching a server mutation, not just on enqueue.
        self.admit_owner_command(&command).await?;
        if admitted.is_some_and(|admitted| !admitted()) {
            return Err(failure(401, "management_scope_changed"));
        }
        if matches!(command, Command::BeginLogin) {
            let ticket = self
                .0
                .host
                .console
                .owner_access_ticket()
                .map_err(|_| failure(503, "native_access_unavailable"))?;
            let bootstrap = self
                .call(
                    "POST",
                    "/console/session",
                    Some(json!({"ticket":ticket})),
                    "",
                )
                .await;
            if !bootstrap.status_code.unwrap_or(StatusCode::OK).is_success() {
                return Err(failure(503, "native_access_unavailable"));
            }
            let cookie = bootstrap
                .headers()
                .get_all("set-cookie")
                .iter()
                .find_map(|v| {
                    v.to_str()
                        .ok()
                        .filter(|s| s.starts_with("hagency_console="))
                        .map(|s| s.split(';').next().unwrap().to_owned())
                })
                .ok_or(failure(503, "native_access_unavailable"))?;
            *self.0.cookie.lock().await = cookie.clone();
            let mut response = self
                .call(
                    "POST",
                    "/console/server-login/start",
                    Some(json!({"server":self.0.expected_origin,"name":"Hagency Desktop"})),
                    &cookie,
                )
                .await;
            let nonce = response
                .headers()
                .get_all("set-cookie")
                .iter()
                .find_map(|v| {
                    v.to_str()
                        .ok()
                        .filter(|s| s.starts_with("hagency_server_login="))
                        .map(|s| s.split(';').next().unwrap().to_owned())
                });
            let value = read_response(&mut response).await?;
            *self.0.nonce.lock().await = nonce;
            return Ok(value);
        }
        if !matrix_managed && !matches!(command, Command::LoginStatus | Command::Logout) {
            self.identity().await?;
        }
        let is_logout = matches!(command, Command::Logout);
        let server_command = server_administration.then(|| command.clone());
        let cookie = self.0.cookie.lock().await.clone();
        let result = if let Some(action) = matrix_creations::action(&command)? {
            self.creation_locked(&cookie, action)
                .await
                .and_then(|reply| {
                    serde_json::to_value(reply).map_err(|_| failure(503, "invalid_native_response"))
                })
        } else if let Some((runtime, operation)) = provider::operation(&command)? {
            let raw = cookie
                .strip_prefix("hagency_console=")
                .ok_or(failure(401, "owner_authorization_required"))?;
            owner_provider::invoke(&self.0.host.console, raw, runtime, operation)
                .await
                .map_err(|error| failure(error.status, error.code))
        } else {
            let (method, path, body) = request(command)?;
            let mut response = self.call(method, &path, body, &cookie).await;
            if let Some(cookie) = response
                .headers()
                .get_all("set-cookie")
                .iter()
                .find_map(|v| {
                    v.to_str()
                        .ok()
                        .filter(|s| s.starts_with("hagency_console="))
                        .map(|s| s.split(';').next().unwrap().to_owned())
                })
            {
                *self.0.cookie.lock().await = cookie;
            }
            read_response(&mut response).await
        };
        if matrix_managed {
            // Logout is allowed to finish without needing a live SDK token.
            if !is_logout {
                if let Err(error) = self.check_command_matrix_source().await {
                    return Err(self.command_matrix_error(error, "sdk_post_request").await);
                }
            }
        }
        if let Some(command) = server_command {
            self.admit_owner_command(&command).await?;
        }
        if admitted.is_some_and(|admitted| !admitted()) {
            return Err(failure(401, "management_scope_changed"));
        }
        result
    }
    async fn call(&self, method: &str, path: &str, body: Option<Value>, cookie: &str) -> Response {
        let client = match method {
            "POST" => TestClient::post(format!("http://{}{path}", self.0.address)),
            "PUT" => TestClient::put(format!("http://{}{path}", self.0.address)),
            "DELETE" => TestClient::delete(format!("http://{}{path}", self.0.address)),
            _ => TestClient::get(format!("http://{}{path}", self.0.address)),
        };
        let client = client
            .add_header("host", self.0.address.to_string(), true)
            .add_header("sec-fetch-site", "same-origin", true)
            .add_header("origin", format!("http://{}", self.0.address), true)
            .add_header("cookie", cookie, true);
        match body {
            Some(body) => client.json(&body).send(&self.0.service).await,
            None => client.send(&self.0.service).await,
        }
    }
}
async fn read_response(response: &mut Response) -> Result<Value, NativeError> {
    let status = response.status_code.unwrap_or(StatusCode::OK).as_u16();
    let value = response
        .take_json::<Value>()
        .await
        .map_err(|_| failure(502, "invalid_native_response"))?;
    if status >= 400 {
        return Err(failure(
            status,
            value["code"].as_str().unwrap_or("native_operation_failed"),
        ));
    }
    Ok(value)
}
#[derive(Clone)]
struct Callback(Weak<Inner>);
#[handler]
impl Callback {
    async fn handle(&self, depot: &mut Depot) {
        depot.insert_typed(self.clone());
    }
}
#[handler]
async fn callback(req: &mut Request, depot: &Depot, res: &mut Response) {
    let result = async {
        let native = NativeOwner(
            depot
                .get_typed::<Callback>()
                .map_err(|_| failure(503, "native_state_unavailable"))?
                .0
                .upgrade()
                .ok_or(failure(410, "native_owner_stopped"))?,
        );
        let _guard = native.0.calls.lock().await;
        if native
            .0
            .matrix_source
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))?
            .is_some()
        {
            return Err(failure(400, "sdk_owns_matrix_login"));
        }
        let query = req
            .uri()
            .query()
            .filter(|q| q.len() <= 8192)
            .ok_or(failure(400, "invalid_oauth_state"))?;
        let nonce = native
            .0
            .nonce
            .lock()
            .await
            .clone()
            .ok_or(failure(400, "invalid_oauth_state"))?;
        let mut response = native
            .call(
                "GET",
                &format!("/console/server-login/callback?{query}"),
                None,
                &nonce,
            )
            .await;
        let cookie = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .find_map(|v| {
                v.to_str()
                    .ok()
                    .filter(|s| s.starts_with("hagency_console="))
                    .map(|s| s.split(';').next().unwrap().to_owned())
            })
            .ok_or(failure(401, "owner_authorization_required"))?;
        *native.0.cookie.lock().await = cookie;
        native.0.nonce.lock().await.take();
        native.identity().await?;
        response.take_string().await.ok();
        Ok::<_, NativeError>(())
    }
    .await;
    res.add_header("cache-control", "no-store", true).unwrap();
    res.add_header("content-security-policy", "default-src 'none'", true)
        .unwrap();
    match result {
        Ok(()) => res.render("Hagency authorization completed. Return to Hagency Desktop."),
        Err(e) => {
            res.status_code(StatusCode::from_u16(e.status).unwrap_or(StatusCode::BAD_REQUEST));
            res.render("Authorization failed. Return to Hagency Desktop and retry with the same Matrix account.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_provider_commands_allow_only_fixed_runtime_routes_and_local_actions() {
        for runtime in ["claude", "octos"] {
            for (action, method, suffix) in [
                (ProviderAction::Status, "GET", ""),
                (ProviderAction::Models, "GET", "/models"),
                (ProviderAction::UseLocal, "POST", "/use-local"),
                (ProviderAction::Disconnect, "POST", "/disconnect"),
            ] {
                let (actual, path, body) = request(Command::RuntimeProvider {
                    runtime: runtime.into(),
                    action,
                })
                .unwrap();
                assert_eq!(actual, method);
                assert_eq!(
                    path,
                    format!("/console/api/owner-provider/runtimes/{runtime}{suffix}")
                );
                assert!(body.is_none());
            }
            for action in [
                ProviderAction::Login,
                ProviderAction::Logout,
                ProviderAction::Cancel,
            ] {
                assert!(
                    request(Command::RuntimeProvider {
                        runtime: runtime.into(),
                        action
                    })
                    .is_err()
                );
            }
        }
        for runtime in ["../claude", "codex", "remote"] {
            assert!(
                request(Command::RuntimeProvider {
                    runtime: runtime.into(),
                    action: ProviderAction::Models
                })
                .is_err()
            );
        }
    }
    #[test]
    fn reply_policy_command_uses_exact_binding_and_boolean_body() {
        for enabled in [false, true] {
            let (method, path, body) = request(Command::Binding {
                agent_id: "agt_1".into(),
                binding_id: "bnd_1".into(),
                action: BindingAction::SetThreadAutoReply(enabled),
            })
            .unwrap();
            assert_eq!(method, "PUT");
            assert_eq!(
                path,
                "/console/api/owned-agents/agt_1/bindings/bnd_1/reply-policy"
            );
            assert_eq!(body, Some(serde_json::json!({"threadAutoReply":enabled})));
        }
    }
    #[test]
    fn native_commands_cannot_select_an_arbitrary_path_or_foreign_binding_route() {
        assert!(
            request(Command::PrepareAgentWorkspace {
                agent_id: "../other".into()
            })
            .is_err()
        );
        let (method, path, body) = request(Command::PrepareAgentWorkspace {
            agent_id: "agt_owned".into(),
        })
        .unwrap();
        assert_eq!(method, "POST");
        assert_eq!(path, "/console/api/owner-provider/workspaces/agt_owned");
        assert!(body.is_none());
        let (method, path, body) = request(Command::Provider {
            action: ProviderAction::Models,
        })
        .unwrap();
        assert_eq!(method, "GET");
        assert_eq!(path, "/console/api/owner-provider/models");
        assert!(body.is_none());
        assert!(
            request(Command::ResumeAgentCommand {
                id: "../other".into()
            })
            .is_err()
        );
        assert!(
            request(Command::Local {
                agent_id: "a".into(),
                action: LocalAction::AgentPolicy,
                input: None,
                binding_id: Some("b".into()),
                requester: None
            })
            .is_err()
        );
        let (_, path, _) = request(Command::Local {
            agent_id: "a".into(),
            action: LocalAction::AgentPolicy,
            input: None,
            binding_id: None,
            requester: None,
        })
        .unwrap();
        assert_eq!(path, "/console/api/owned-agents/a/agent-policy");

        for id in [
            "",
            "../other",
            "agt?token=x",
            "agt/retire",
            "https://evil.test",
        ] {
            assert!(
                request(Command::Bindings {
                    agent_id: id.into()
                })
                .is_err()
            );
        }
        assert!(
            request(Command::SpaceCandidates {
                cursor: Some("!s%3Atest".into())
            })
            .is_err()
        );
        assert_eq!(
            request(Command::Binding {
                agent_id: "agt_1".into(),
                binding_id: "bnd_1".into(),
                action: BindingAction::Leave
            })
            .unwrap()
            .0,
            "DELETE"
        );
        assert!(
            request(Command::Local {
                agent_id: "agt_1".into(),
                action: LocalAction::Policy,
                input: None,
                binding_id: None,
                requester: None
            })
            .is_err()
        );
        let (_, path, _) = request(Command::Local {
            agent_id: "agt_1".into(),
            action: LocalAction::Policy,
            input: None,
            binding_id: Some("bnd_1".into()),
            requester: Some("@user:test&url=https://evil.test".into()),
        })
        .unwrap();
        assert!(path.starts_with("/console/api/owned-agents/agt_1/local-policy?"));
        assert!(!path.contains("&url="));
    }
    #[tokio::test]
    async fn native_host_needs_no_web_assets_and_restart_never_restores_remote_authority() {
        let root = hagency_platform::legacy_writer::test_support::initialized_tempdir().unwrap();
        let state = root.path().join("state");
        let owner = NativeOwner::open(&state, "https://example.test/", "@owner:example.test")
            .await
            .unwrap();
        assert!(owner.has_callback_listener());
        assert_eq!(
            owner.execute(Command::Projects).await.unwrap_err().code,
            "owner_authorization_required"
        );
        assert!(owner.execute(Command::LoginStatus).await.is_ok());
        owner.shutdown().await;
        assert!(owner.execute(Command::BeginLogin).await.is_err());
        drop(owner);
        let restored = NativeOwner::open(&state, "https://example.test/", "@owner:example.test")
            .await
            .unwrap();
        assert_eq!(
            restored.execute(Command::Agents).await.unwrap_err().code,
            "owner_authorization_required"
        );
        restored.shutdown().await;
    }
}

#[cfg(test)]
mod owner_scope_command_tests {
    use super::*;
    #[test]
    fn native_scope_admin_commands_are_closed_and_do_not_claim_admin_identity() {
        let policy = ProjectCreationPolicy {
            default_allow: true,
            allow: Default::default(),
            deny: Default::default(),
        };
        let (method, path, body) = request(Command::SetProjectCreationPolicy {
            project_id: "p".into(),
            expected_revision: 4,
            policy,
        })
        .unwrap();
        assert_eq!(
            (method, path.as_str()),
            ("PUT", "/console/api/owner-projects/p/creation-policy")
        );
        assert_eq!(body.unwrap()["expectedRevision"], 4);
        let (method, path, body) = request(Command::SetScopeServicePause {
            project_id: "p".into(),
            room_id: Some("!r:test".into()),
            paused: true,
        })
        .unwrap();
        assert_eq!(method, "POST");
        assert!(path.ends_with("/pause-service"));
        assert!(path.contains("%21r%3Atest"));
        assert!(body.is_none());
        assert!(
            request(Command::ScopeServiceState {
                project_id: "../p".into(),
                room_id: None
            })
            .is_err()
        );
        assert!(
            request(Command::SetRoomCreationPolicy {
                project_id: "p".into(),
                room_id: "!r:test".into(),
                expected_revision: -1,
                policy: RoomCreationPolicy::Disabled
            })
            .is_err()
        );
    }
    #[test]
    fn policy_dtos_reject_unknown_fields_and_disabled_payload() {
        assert!(
            serde_json::from_value::<RoomCreationPolicy>(json!({"mode":"disabled","allow":[]}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<ProjectCreationPolicy>(
                json!({"defaultAllow":true,"allow":[],"deny":[],"owner":"override"})
            )
            .is_err()
        );
        assert_eq!(
            serde_json::to_value(RoomCreationPolicy::Disabled).unwrap(),
            json!({"mode":"disabled"})
        );
    }
    #[test]
    fn agent_resource_commands_need_no_synthetic_room_and_reject_room_scope() {
        for retired in ["Model", "SaveModel"] {
            assert!(serde_json::from_value::<LocalAction>(json!(retired)).is_err());
        }
        let (_, path, _) = request(Command::Local {
            agent_id: "a".into(),
            action: LocalAction::AgentPolicy,
            input: None,
            binding_id: None,
            requester: None,
        })
        .unwrap();
        assert_eq!(path, "/console/api/owned-agents/a/agent-policy");
        assert!(
            request(Command::Local {
                agent_id: "a".into(),
                action: LocalAction::SaveAgentModel,
                input: Some(json!({})),
                binding_id: Some("b".into()),
                requester: None
            })
            .is_err()
        );
        assert!(request(Command::ResumeAgentCommand { id: "../x".into() }).is_err());
        assert_eq!(
            request(Command::AgentCommands).unwrap().1,
            "/console/api/owned-agents/commands"
        );
    }
    #[test]
    fn device_and_assignment_commands_expose_no_bearer_or_arbitrary_path() {
        assert_eq!(
            request(Command::Devices).unwrap().1,
            "/console/api/owned-agents/devices"
        );
        let (method, path, body) = request(Command::LocalApprovals).unwrap();
        assert_eq!((method, path.as_str(), body), ("GET", "/console/api/owned-agents/pending-approvals", None));
        let (method, path, body) = request(Command::AssignAgentToCurrentDevice {
            agent_id: "a".into(),
            expected_generation: 3,
        })
        .unwrap();
        assert_eq!(
            (method, path.as_str()),
            ("PUT", "/console/api/owned-agents/a/execution-device")
        );
        assert_eq!(body, Some(json!({"expectedGeneration":3})));
        assert!(
            request(Command::AssignAgentToCurrentDevice {
                agent_id: "a".into(),
                expected_generation: -1
            })
            .is_err()
        );
        assert!(
            request(Command::AgentDetails {
                agent_id: "../x".into()
            })
            .is_err()
        );
        assert_eq!(
            request(Command::AgentDetails {
                agent_id: "a".into()
            })
            .unwrap()
            .1,
            "/console/api/owned-agents/a"
        );
        assert!(
            request(Command::AgentOwnerDirect {
                agent_id: "a".into(),
                room_id: "!r:test/escape".into()
            })
            .is_err()
        );
    }
}

mod workspaces;
pub use hagency_store::management::{WorkspaceListQuery, WorkspaceQuery, WorkspaceReleaseBlock, WorkspaceStatus, WorkspacePage, ClearWorkspaceDirty, WorkspaceRelease};

mod interrupted_calls;
pub use hagency_store::owner_resources::{
    InterruptedCall, InterruptedCallPage, InterruptedCallQuery, InterruptedCallReconciliation,
    ReconcileInterruptedCall,
};
mod stopped;
pub use hagency_store::management::{StoppedListQuery, StoppedQuery, StoppedAttempt, StoppedPage, StoppedInventoryEntry, StoppedInspection, ReviewStopped, StoppedReview, StoppedDecision, ResolveStopped, StoppedResolution};

mod web_catalog;
pub use web_catalog::PreparedWebCatalog;
pub use hagency_store::management::{WebCatalogQuery, WebCatalogPrepare, WebCatalogChange, WebCatalogView, WebOfferPage, WebTrustPage, WebCatalogResult};
mod web_overview;
pub use web_overview::{WebOverviewQuery, WebAccountCapacityQuery, WebTerminalQuery, WebAlertsQuery, WebAlertsPage};
mod web_connections;
pub use web_connections::{WebConnectionProbeInput, WebConnectionProbe, WebConnectionEdit, WebConnectionReview, WebConnectionReviewSummary, NativeConnectionRegistration};
pub use hagency_store::management::{WebConnection, WebConnectionPage, WebConnectionResult};

pub use management_server::pools::PoolAgentInput;
