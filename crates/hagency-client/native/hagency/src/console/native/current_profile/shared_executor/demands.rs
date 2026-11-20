//! Accepted owner-scope consumer. No mini-app permission or Run authority.
use super::*;
use crate::console::native::{
    SharedDemandPoll, SharedDemandResume, SharedDemandStopReceipt, SharedDemandStopResume,
};
impl CurrentSharedExecutor {
    pub(crate) async fn discover_for_worker(&self, budget: bool, after: Option<String>) -> Result<(Option<String>,Option<(String,String)>),NativeError> {
        self.selection.check_executor_owner(&self.owner).await?;
        if budget {
            #[cfg(feature="shared-budget-execution")]
            return self.transport.discover_budget_demand(after).await;
            #[cfg(not(feature="shared-budget-execution"))]
            return Err(failure(409,"shared_budget_execution_disabled"));
        }
        self.transport.discover_demand(after).await
    }

    pub(crate) async fn consume_for_worker(&self,budget:bool,id:&str,digest:&str)->Result<crate::console::native::SharedDemandProgress,NativeError>{
        self.selection.check_executor_owner(&self.owner).await?;
        if budget {
            #[cfg(feature="shared-budget-execution")]
            return self.transport.consume_budget_ref(&self.selection.registry,id,digest).await;
            #[cfg(not(feature="shared-budget-execution"))]
            return Err(failure(409,"shared_budget_execution_disabled"));
        }
        self.transport.consume_demand_ref(&self.selection.registry,id,digest).await
    }
    pub async fn poll_demand(
        &self,
        after: Option<String>,
    ) -> Result<SharedDemandPoll, NativeError> {
        self.selection.check_executor_owner(&self.owner).await?;
        self.transport
            .poll_demand(&self.selection.registry, after)
            .await
    }
    pub async fn resume_next_demand(
        &self,
        after: Option<String>,
    ) -> Result<Option<SharedDemandResume>, NativeError> {
        self.selection.check_executor_owner(&self.owner).await?;
        self.transport
            .resume_next_demand(&self.selection.registry, after)
            .await
    }
    pub async fn resume_next_unfinished_demand(
        &self,
        after: Option<String>,
    ) -> Result<Option<SharedDemandResume>, NativeError> {
        self.selection.check_executor_owner(&self.owner).await?;
        self.transport
            .resume_next_unfinished_demand(&self.selection.registry, after)
            .await
    }
    pub async fn stop_demand_assignment(
        &self,
        assignment: String,
    ) -> Result<SharedDemandStopReceipt, NativeError> {
        self.selection.check_executor_owner(&self.owner).await?;
        self.transport
            .stop_demand_assignment(&self.selection.registry, assignment)
            .await
    }
    pub async fn resume_next_demand_stop(
        &self,
        after: Option<String>,
    ) -> Result<Option<SharedDemandStopResume>, NativeError> {
        self.selection.check_executor_owner(&self.owner).await?;
        self.transport
            .resume_next_demand_stop(&self.selection.registry, after)
            .await
    }
}

#[cfg(feature="shared-budget-execution")]
impl CurrentSharedExecutor {
    pub async fn poll_budget_demand(&self,after:Option<String>)->Result<SharedDemandPoll,NativeError>{
        self.selection.check_executor_owner(&self.owner).await?;
        self.transport.poll_budget_demand(&self.selection.registry,after).await
    }
    pub async fn resume_next_budget_demand(&self,after:Option<String>)->Result<Option<SharedDemandResume>,NativeError>{
        self.selection.check_executor_owner(&self.owner).await?;
        self.transport.resume_next_budget_demand(&self.selection.registry,after).await
    }
    pub async fn poll_budget_successor(&self,after:Option<String>)->Result<SharedExecutionProgress,NativeError>{
        self.selection.check_executor_owner(&self.owner).await?;
        self.transport.poll_budget_successor(&self.selection.registry,after).await
    }
}
