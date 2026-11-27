//! Exact original event, immutable startup retry, and Run/Stop regressions.
use super::*;
fn offer(c:&SharedRuntimeCommand,id:&str,revision:u64)->SharedDemandOffer {
    SharedDemandOffer {version:1,demand_id:id.into(),assignment_id:c.assignment_id.clone(),event_id:format!("${id}"),event_digest:"e".repeat(64),control_revision:revision,received_at_ms:graphs::now_ms().unwrap(),predecessor_command_id:c.command_id.clone(),predecessor_command_digest:digest(c)}
}
fn proof_for(f:&Fixture,o:&SharedDemandOffer)->VerifiedSharedDemand {
    VerifiedSharedDemand::new(enrollment(f),o.clone(),digest(o),graphs::now_ms().unwrap(),Instant::now()+Duration::from_secs(20)).unwrap()
}
async fn status(f:&Fixture,c:&SharedRuntimeCommand,o:&SharedDemandOffer)->SharedBudgetDemandStatus {
    let s=f.registry.shared_budget_successor_selection(enrollment(f),c.clone()).await.unwrap();
    SharedBudgetDemandStatus {offer:o.clone(),demand_digest:digest(o),state:"offered".into(),execution_budget:s.execution_budget,original_block:None,runtime_command_id:None,binding:None}
}
fn verified(f:&Fixture,s:SharedBudgetDemandStatus)->VerifiedSharedBudgetDemand {VerifiedSharedBudgetDemand::new(proof_for(f,&s.offer),s).unwrap()}
async fn prepare(f:&Fixture,c:&SharedRuntimeCommand,s:SharedBudgetDemandStatus)->Result<SharedBudgetDemandIntent,Error>{
    f.registry.prepare_shared_budget_demand(verified(f,s),c.clone()).await
}
async fn command_for(f:&Fixture,c:&SharedRuntimeCommand,input:&SharedBudgetDemandStartInput)->SharedBudgetAttemptCommand {
    let mut result=next(f,c,"demand_runtime").await;
    result.start_intent_id=input.start.start_intent_id.clone();result.predecessor=input.start.predecessor.clone();
    result.execution_budget=input.start.execution_budget.clone();result.budget_observation=input.start.budget_observation.clone();result
}
async fn count(f:&Fixture,table:&'static str)->u64 {
    f.registry.0.store.call(1024,move|db|Ok(db.db.query_row(&format!("SELECT count(*) FROM {table}"),[],|r|r.get(0))?)).await.unwrap()
}
#[tokio::test]
async fn budget_execution_v3_demand_retry_restarts_keep_exact_request_and_manual_lanes_separate(){
    let(mut f,c)=synthetic().await;let o=offer(&c,"durable_budget",1);let s=status(&f,&c,&o).await;
    let(a,b)=tokio::join!(prepare(&f,&c,s.clone()),prepare(&f,&c,s.clone()));
    let input=a.unwrap().input().unwrap().clone();assert_eq!(digest(&input),digest(b.unwrap().input().unwrap()));
    assert_eq!(count(&f,"owner_shared_budget_start_intents").await,1);
    assert!(f.registry.recover_shared_budget_attempt_start(cleanup(&f,&c,1).await,c.clone(),input.start.predecessor.command_digest.clone()).await.unwrap().is_none());
    assert!(f.registry.prepare_shared_budget_attempt_start(enrollment(&f),c.clone(),input.start.start_intent_id.clone()).await.is_err());
    f.registry.shutdown().await.unwrap();f.registry=OwnerResourceRegistry::open_root(&f._temp.path().join("owner")).unwrap();
    assert_eq!(digest(prepare(&f,&c,s).await.unwrap().input().unwrap()),digest(&input));
    increase(&f,&c,"after_uncertain_start",25).await;
    let newer=status(&f,&c,&o).await;
    assert_eq!(digest(prepare(&f,&c,newer).await.unwrap().input().unwrap()),digest(&input),"never rebase an uncertain start onto more budget");
    let command=command_for(&f,&c,&input).await;
    assert!(f.registry.claim_shared_budget_successor(delivery(&f,command)).await.is_err());
    assert_eq!(count(&f,"owner_shared_budget_start_intents").await,1);
    f.registry.shutdown().await.unwrap();
}
#[tokio::test]
async fn budget_execution_v3_demand_reassesses_original_v2_budget_block_without_rewriting_it(){
    let(f,c)=synthetic().await;
    call(&f,&c,"retained_overrun","settled",150,231,true,false).await;
    let o=offer(&c,"original_block",1);
    let old=f.registry.prepare_shared_demand(proof_for(&f,&o),c.clone(),None).await.unwrap();
    assert_eq!(*old.disposition(),SharedDemandDisposition::BudgetBlocked);
    let original_hash=digest(&json!({"demandDigest":digest(&o),"expectedControlRevision":1,"reason":"budget_blocked","budget":old.budget().unwrap().observation}));
    increase(&f,&c,"restore_capacity",100).await;
    let mut s=status(&f,&c,&o).await;
    assert!(prepare(&f,&c,s.clone()).await.is_err(),"missing original block cannot be substituted");
    s.original_block=Some(SharedOriginalBudgetBlock{reason:"budget_blocked".into(),payload_digest:original_hash});
    let input=prepare(&f,&c,s).await.unwrap().input().unwrap().clone();
    assert_eq!(input.start.budget_observation.known_consumed_tokens,231);
    assert_eq!(input.start.budget_observation.remaining_tokens,69);
    assert_eq!(input.demand_digest,digest(&o));assert_eq!(input.expected_control_revision,1);
    assert_eq!(count(&f,"owner_shared_demands").await,1);
    assert_eq!(count(&f,"owner_shared_demand_decisions").await,1);
    assert_eq!(count(&f,"owner_shared_budget_demand_decisions").await,1);
    let unchanged=f.registry.recover_shared_demand(cleanup(&f,&c,1).await,o.demand_id).await.unwrap().unwrap();
    assert_eq!(digest(unchanged.budget().unwrap()),digest(old.budget().unwrap()));
    let command=command_for(&f,&c,&input).await;
    let SharedBudgetAttemptClaim::Issued(custody)=f.registry.claim_shared_budget_successor(delivery(&f,command)).await.unwrap() else{panic!("one claim")};
    assert_eq!(custody.demand_control_revision(),Some(1));
    f.registry.shutdown().await.unwrap();
}
#[tokio::test]
async fn budget_execution_v3_demand_each_head_appends_block_then_start_for_same_event(){
    let(f,c)=synthetic().await;
    call(&f,&c,"complete_overrun","settled",150,231,true,false).await;
    let o=offer(&c,"v3_block",1);let s=status(&f,&c,&o).await;
    let blocked=prepare(&f,&c,s.clone()).await.unwrap();assert_eq!(*blocked.disposition(),SharedDemandDisposition::BudgetBlocked);
    let block_hash=digest(blocked.block().unwrap());
    assert_eq!(digest(prepare(&f,&c,s).await.unwrap().block().unwrap()),block_hash);
    increase(&f,&c,"too_small",10).await;
    assert_eq!(*prepare(&f,&c,status(&f,&c,&o).await).await.unwrap().disposition(),SharedDemandDisposition::BudgetBlocked);
    increase(&f,&c,"enough",30).await;
    let started=prepare(&f,&c,status(&f,&c,&o).await).await.unwrap();assert!(started.input().is_some());
    assert_eq!(count(&f,"owner_shared_budget_demand_decisions").await,3);
    assert_eq!(count(&f,"owner_shared_budget_start_intents").await,1);
    assert_eq!(count(&f,"owner_shared_demands").await,1);
    f.registry.shutdown().await.unwrap();
}
#[tokio::test]
async fn budget_execution_v3_demand_stop_cannot_be_bypassed_by_delta_or_old_run(){
    let(f,c)=synthetic().await;
    let o=offer(&c,"old_run",1);let s=status(&f,&c,&o).await;let input=prepare(&f,&c,s.clone()).await.unwrap().input().unwrap().clone();
    let command=command_for(&f,&c,&input).await;
    f.registry.stop_shared_demand_revision(cleanup(&f,&c,1).await,1).await.unwrap();
    assert!(prepare(&f,&c,s).await.is_err());
    assert!(f.registry.claim_shared_budget_successor(delivery(&f,command)).await.is_err());
    increase(&f,&c,"after_stop",25).await;
    assert!(prepare(&f,&c,status(&f,&c,&o).await).await.is_err());
    let later=offer(&c,"new_run",3);
    let later=prepare(&f,&c,status(&f,&c,&later).await).await.unwrap();assert_eq!(*later.disposition(),SharedDemandDisposition::Pending,"uncertain older start still owns predecessor");
    assert_eq!(count(&f,"owner_shared_budget_start_intents").await,1);
    f.registry.shutdown().await.unwrap();
}
#[tokio::test]
async fn budget_execution_v3_demand_new_run_after_stop_has_exact_control_pin(){
    let(f,c)=synthetic().await;
    f.registry.stop_shared_demand_revision(cleanup(&f,&c,1).await,1).await.unwrap();
    let o=offer(&c,"fresh_run",3);let input=prepare(&f,&c,status(&f,&c,&o).await).await.unwrap().input().unwrap().clone();
    let command=command_for(&f,&c,&input).await;
    let SharedBudgetAttemptClaim::Issued(custody)=f.registry.claim_shared_budget_successor(delivery(&f,command)).await.unwrap()else{panic!("new run claim")};
    assert_eq!(custody.demand_control_revision(),Some(3));
    f.registry.shutdown().await.unwrap();
}
#[tokio::test]
async fn budget_execution_v3_demand_failed_decision_rolls_back_offer_control_and_start(){
    let(f,c)=synthetic().await;let o=offer(&c,"atomic",1);let s=status(&f,&c,&o).await;
    f.registry.0.store.call(1024,|db|{db.db.execute_batch("CREATE TRIGGER fail_budget_demand BEFORE INSERT ON owner_shared_budget_demand_decisions BEGIN SELECT RAISE(ABORT,'test injected failure'); END;")?;Ok(())}).await.unwrap();
    assert!(prepare(&f,&c,s.clone()).await.is_err());
    for t in ["owner_shared_demands","owner_shared_demand_controls","owner_shared_budget_start_intents","owner_shared_budget_demand_decisions"] {assert_eq!(count(&f,t).await,0,"{t}");}
    f.registry.0.store.call(1024,|db|{db.db.execute_batch("DROP TRIGGER fail_budget_demand;")?;Ok(())}).await.unwrap();
    assert!(prepare(&f,&c,s).await.unwrap().input().is_some());
    f.registry.shutdown().await.unwrap();
}

#[tokio::test]
async fn budget_execution_v3_terminal_unstarted_demand_unblocks_only_new_event() {
    let(mut f,c)=synthetic().await;
    call(&f,&c,"retained_unknown","unknown",40,7,false,false).await;
    let old=offer(&c,"old_failed_demand",1);let old_status=status(&f,&c,&old).await;
    let input=prepare(&f,&c,old_status.clone()).await.unwrap().input().unwrap().clone();
    let failed=command_for(&f,&c,&input).await;
    f.registry.fence_shared_budget_successor(cleanup(&f,&c,1).await,failed.clone()).await.unwrap();
    f.registry.record_unstarted_shared_budget_attempt(cleanup(&f,&c,1).await,failed.clone()).await.unwrap().unwrap();
    let mut new=offer(&c,"new_research",1);new.received_at_ms=old.received_at_ms+1;
    new.predecessor_command_id=failed.command_id.clone();new.predecessor_command_digest=digest(&failed);
    let new_status=status(&f,&c,&new).await;
    assert_eq!(*prepare(&f,&c,new_status.clone()).await.unwrap().disposition(),SharedDemandDisposition::Pending);
    assert!(f.registry.retain_shared_budget_demand_terminal(verified(&f,old_status.clone()),c.clone()).await.is_err());
    let mut terminal=old_status;terminal.state="head_changed".into();terminal.runtime_command_id=Some(failed.command_id.clone());
    f.registry.retain_shared_budget_demand_terminal(verified(&f,terminal.clone()),c.clone()).await.unwrap();
    f.registry.retain_shared_budget_demand_terminal(verified(&f,terminal),c.clone()).await.unwrap();
    assert_eq!(count(&f,"owner_shared_budget_demand_terminals").await,1);
    f.registry.shutdown().await.unwrap();f.registry=OwnerResourceRegistry::open_root(&f._temp.path().join("owner")).unwrap();
    let input=prepare(&f,&c,new_status).await.unwrap().input().unwrap().clone();
    assert_eq!(input.demand_digest,digest(&new));assert!(input.start.predecessor.process_digest.is_empty());
    assert_eq!((input.start.budget_observation.known_consumed_tokens,input.start.budget_observation.retained_tokens),(7,7));
    let mut fresh=next(&f,&c,"new_research_runtime").await;fresh.start_intent_id=input.start.start_intent_id.clone();fresh.predecessor=input.start.predecessor.clone();fresh.execution_budget=input.start.execution_budget.clone();fresh.budget_observation=input.start.budget_observation.clone();
    assert!(matches!(f.registry.claim_shared_budget_successor(delivery(&f,fresh)).await.unwrap(),SharedBudgetAttemptClaim::Issued(_)));
    assert_eq!(count(&f,"owner_shared_demands").await,2);assert_eq!(count(&f,"owner_shared_budget_start_intents").await,2);
    assert!(f.registry.claim_shared_budget_successor(delivery(&f,failed)).await.is_err());
    f.registry.shutdown().await.unwrap();
}

/// Start intents of one lane whose place the end of their event gave back.
async fn released_in(f:&Fixture,table:&'static str)->Vec<String> {
    f.registry.0.store.call(4096,move|db|{
        let mut rows=db.db.prepare(&format!("SELECT start_intent_id FROM {table} ORDER BY start_intent_id"))?;
        let ids=rows.query_map([],|r|r.get(0))?.collect::<Result<Vec<String>,_>>()?;
        Ok(ids)
    }).await.unwrap()
}
/// SQL run as is, with SQLite's own message on failure.
async fn store_sql(f:&Fixture,statement:String)->Result<(),String> {
    f.registry.0.store.call(4096,move|db|Ok(db.db.execute_batch(&statement)?)).await.map_err(|e|match e {Error::Sqlite(e)=>e.to_string(),e=>e.to_string()})
}
/// The event's status once it ended; it names the start the server made of
/// it, if any.
fn ended(s:&SharedBudgetDemandStatus,state:&str,runtime:Option<&str>)->SharedBudgetDemandStatus {
    let mut value=s.clone();value.state=state.into();value.runtime_command_id=runtime.map(Into::into);value
}
/// A start of this predecessor that is no event's: refused while another
/// start holds the place.
async fn manual_start(f:&Fixture,c:&SharedRuntimeCommand)->Result<SharedBudgetAttemptStartIntent,Error> {
    f.registry.prepare_shared_budget_attempt_start(enrollment(f),c.clone(),"sbsi_manual_start".into()).await
}
fn place_held(r:&Result<SharedBudgetAttemptStartIntent,Error>)->bool {
    matches!(r,Err(Error::Sqlite(e)) if e.to_string().contains("budget start place held"))
}

#[tokio::test]
async fn budget_execution_v3_a_refused_start_gives_its_place_back_when_its_message_ends(){
    let mut waiting=Vec::new();
    for state in ["expired","cancelled","head_changed"] {
        let(f,c)=synthetic().await;
        let refused=offer(&c,"refused_start",1);let s=status(&f,&c,&refused).await;
        let held=prepare(&f,&c,s.clone()).await.unwrap().input().unwrap().start.start_intent_id.clone();
        // The server refused that start (a 409 such as
        // `shared_demand_not_current`) and saved nothing for it. Until the
        // message ends it may be sent again, so it keeps its place. The next
        // message names the same predecessor; a cancelled message's comes in
        // the next Run.
        assert!(manual_start(&f,&c).await.is_err(),"{state}");
        let run=if state=="cancelled"{2}else{1};
        let mut next=offer(&c,"next_message",run);next.received_at_ms=refused.received_at_ms+1;
        if run==1 {
            let saved=prepare(&f,&c,status(&f,&c,&next).await).await.unwrap();
            assert_eq!(*saved.disposition(),SharedDemandDisposition::Pending,"{state}: the earlier message still holds the queue");
        }
        // The next read of the message shows it ended, with no start of it;
        // that read may come twice.
        for _ in 0..2 {
            f.registry.retain_shared_budget_demand_terminal(verified(&f,ended(&s,state,None)),c.clone()).await.unwrap();
        }
        if state=="cancelled" {f.registry.stop_shared_demand_revision(cleanup(&f,&c,1).await,1).await.unwrap();}
        let saved=prepare(&f,&c,status(&f,&c,&next).await).await.unwrap();
        if *saved.disposition()!=SharedDemandDisposition::StartPrepared {
            waiting.push(state);f.registry.shutdown().await.unwrap();continue;
        }
        let start=&saved.input().unwrap().start;
        assert_eq!(start.predecessor.command_id,c.command_id);
        assert_ne!(start.start_intent_id,held);
        assert_eq!(released_in(&f,"owner_shared_budget_start_releases").await,vec![held.clone()],"{state}: released once");
        // The place is the new start's now: no other start of this predecessor.
        let manual=manual_start(&f,&c).await;
        assert!(place_held(&manual),"{state}: {:?}",manual.err().map(|e|e.to_string()));
        f.registry.shutdown().await.unwrap();
    }
    assert!(waiting.is_empty(),"the next message waits after: {waiting:?}");
}

#[tokio::test]
async fn budget_execution_v3_a_start_the_server_may_have_made_keeps_its_place(){
    // The message's status names a start of it: the server made this start
    // and its reply was lost.
    let(f,c)=synthetic().await;
    let made=offer(&c,"made_start",1);let s=status(&f,&c,&made).await;
    let held=prepare(&f,&c,s.clone()).await.unwrap().input().unwrap().start.start_intent_id.clone();
    f.registry.retain_shared_budget_demand_terminal(verified(&f,ended(&s,"expired",Some("sat_reply_lost"))),c.clone()).await.unwrap();
    let mut next=offer(&c,"next_message",1);next.received_at_ms=made.received_at_ms+1;
    assert_eq!(*prepare(&f,&c,status(&f,&c,&next).await).await.unwrap().disposition(),SharedDemandDisposition::Pending);
    assert!(released_in(&f,"owner_shared_budget_start_releases").await.is_empty());
    let forged=format!("INSERT INTO owner_shared_budget_start_releases VALUES('{held}','{}')",made.demand_id);
    assert!(store_sql(&f,forged).await.unwrap_err().contains("unproven budget start release"));
    f.registry.shutdown().await.unwrap();
    // This computer holds the command issued for the start, fenced or
    // claimed: whatever the message's status says, the start keeps its place.
    for claimed in [false,true] {
        let(f,c)=synthetic().await;
        let issued=offer(&c,"issued_start",1);let s=status(&f,&c,&issued).await;
        let input=prepare(&f,&c,s.clone()).await.unwrap().input().unwrap().clone();
        let command=command_for(&f,&c,&input).await;
        if claimed {
            assert!(matches!(f.registry.claim_shared_budget_successor(delivery(&f,command)).await.unwrap(),SharedBudgetAttemptClaim::Issued(_)));
        } else {
            f.registry.fence_shared_budget_successor(cleanup(&f,&c,1).await,command).await.unwrap();
        }
        f.registry.retain_shared_budget_demand_terminal(verified(&f,ended(&s,"expired",None)),c.clone()).await.unwrap();
        assert!(released_in(&f,"owner_shared_budget_start_releases").await.is_empty(),"claimed: {claimed}");
        let forged=format!("INSERT INTO owner_shared_budget_start_releases VALUES('{}','{}')",input.start.start_intent_id,issued.demand_id);
        assert!(store_sql(&f,forged).await.unwrap_err().contains("unproven budget start release"),"claimed: {claimed}");
        f.registry.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn budget_execution_v3_a_plain_start_given_back_holds_no_place_after_a_budget_is_applied(){
    // A message of the plain lane ended after the server refused its start,
    // and the start gave its place back. A budget is applied, and the Agent's
    // next message comes in the budget lane, after the same process.
    let f=fixture().await;let c=command(&f).await;
    f.registry.claim_shared_runtime(proof(&f,c.clone())).await.unwrap();
    stopped(&f,&c,"stopped").await;
    let refused=offer(&c,"refused_plain_start",1);
    let held=f.registry.prepare_shared_demand(proof_for(&f,&refused),c.clone(),None).await.unwrap().input().unwrap().start.start_intent_id.clone();
    f.registry.retain_shared_demand_terminal(cleanup(&f,&c,1).await,refused.clone(),"expired".into(),None).await.unwrap();
    assert_eq!(released_in(&f,"owner_shared_start_releases").await,vec![held.clone()]);
    increase(&f,&c,"applied_after_the_end",50).await;
    let mut next=offer(&c,"next_message",1);next.received_at_ms=refused.received_at_ms+1;
    let saved=prepare(&f,&c,status(&f,&c,&next).await).await.unwrap();
    assert_eq!(*saved.disposition(),SharedDemandDisposition::StartPrepared,"the next message waits behind a start that gave its place back");
    let start=&saved.input().unwrap().start;
    assert_eq!(start.predecessor.command_id,c.command_id);
    assert_ne!(start.start_intent_id,held);
    f.registry.shutdown().await.unwrap();
}

#[tokio::test]
async fn budget_execution_v3_a_plain_start_gives_its_place_back_when_the_budget_lane_read_its_end_first(){
    // A plain message's start was refused, and a budget is applied while the
    // message waits. Both lanes read every saved message, and the budget lane
    // reads this one's end first; its status names no plain start.
    let f=fixture().await;let c=command(&f).await;
    f.registry.claim_shared_runtime(proof(&f,c.clone())).await.unwrap();
    stopped(&f,&c,"stopped").await;
    let refused=offer(&c,"refused_plain_start",1);
    let held=f.registry.prepare_shared_demand(proof_for(&f,&refused),c.clone(),None).await.unwrap().input().unwrap().start.start_intent_id.clone();
    increase(&f,&c,"applied_while_it_waits",50).await;
    let s=status(&f,&c,&refused).await;
    f.registry.retain_shared_budget_demand_terminal(verified(&f,ended(&s,"expired",None)),c.clone()).await.unwrap();
    assert!(released_in(&f,"owner_shared_start_releases").await.is_empty());
    let mut next=offer(&c,"next_message",1);next.received_at_ms=refused.received_at_ms+1;
    assert_eq!(*prepare(&f,&c,status(&f,&c,&next).await).await.unwrap().disposition(),SharedDemandDisposition::Pending);
    // The plain lane's read of the same end, which may come twice.
    for _ in 0..2 {
        f.registry.retain_shared_demand_terminal(cleanup(&f,&c,1).await,refused.clone(),"expired".into(),None).await.unwrap();
    }
    assert_eq!(released_in(&f,"owner_shared_start_releases").await,vec![held.clone()],"the plain end gives the plain start back");
    assert!(f.registry.retain_shared_demand_terminal(cleanup(&f,&c,1).await,refused.clone(),"cancelled".into(),None).await.is_err(),"another plain end");
    // Only the other lane's end goes beside the saved one.
    let copy=format!("INSERT INTO owner_shared_demand_second_terminals SELECT demand_id,payload,digest,ledger_revision FROM owner_shared_budget_demand_terminals WHERE demand_id='{}'",refused.demand_id);
    assert!(store_sql(&f,copy).await.unwrap_err().contains("unpaired second demand terminal"));
    let saved=prepare(&f,&c,status(&f,&c,&next).await).await.unwrap();
    assert_eq!(*saved.disposition(),SharedDemandDisposition::StartPrepared,"the next message waits behind a start that gave its place back");
    assert_ne!(saved.input().unwrap().start.start_intent_id,held);
    f.registry.shutdown().await.unwrap();
}

#[tokio::test]
async fn budget_execution_v3_a_budget_start_gives_its_place_back_when_the_plain_lane_read_its_end_first(){
    // Both lanes read every saved message, and the plain lane may read this
    // one's end first; its status names no budget start.
    let(f,c)=synthetic().await;
    let refused=offer(&c,"refused_start",1);let s=status(&f,&c,&refused).await;
    let held=prepare(&f,&c,s.clone()).await.unwrap().input().unwrap().start.start_intent_id.clone();
    f.registry.retain_shared_demand_terminal(cleanup(&f,&c,1).await,refused.clone(),"expired".into(),None).await.unwrap();
    assert!(released_in(&f,"owner_shared_budget_start_releases").await.is_empty());
    let mut next=offer(&c,"next_message",1);next.received_at_ms=refused.received_at_ms+1;
    assert_eq!(*prepare(&f,&c,status(&f,&c,&next).await).await.unwrap().disposition(),SharedDemandDisposition::Pending);
    // The budget lane's read of the same end, which may come twice.
    for _ in 0..2 {
        f.registry.retain_shared_budget_demand_terminal(verified(&f,ended(&s,"expired",None)),c.clone()).await.unwrap();
    }
    assert_eq!(released_in(&f,"owner_shared_budget_start_releases").await,vec![held.clone()],"the budget end gives the budget start back");
    assert!(f.registry.retain_shared_budget_demand_terminal(verified(&f,ended(&s,"cancelled",None)),c.clone()).await.is_err(),"another budget end");
    assert!(store_sql(&f,"DELETE FROM owner_shared_demand_second_terminals".into()).await.unwrap_err().contains("retained second demand terminal"));
    assert!(store_sql(&f,"UPDATE owner_shared_demand_second_terminals SET digest=digest".into()).await.unwrap_err().contains("immutable second demand terminal"));
    let saved=prepare(&f,&c,status(&f,&c,&next).await).await.unwrap();
    assert_eq!(*saved.disposition(),SharedDemandDisposition::StartPrepared,"the next message waits behind a start that gave its place back");
    assert_ne!(saved.input().unwrap().start.start_intent_id,held);
    f.registry.shutdown().await.unwrap();
}

#[tokio::test]
async fn budget_execution_v3_a_budget_start_given_back_still_bars_a_plain_start(){
    // The server refuses every plain start of an Agent with an applied budget
    // (`shared_budget_execution_required`). A plain start saved after a budget
    // start of the same predecessor could only be refused, and it would hold
    // the place the budget lane's next message needs.
    let(f,c)=synthetic().await;
    let refused=offer(&c,"refused_start",1);let s=status(&f,&c,&refused).await;
    let held=prepare(&f,&c,s.clone()).await.unwrap().input().unwrap().start.start_intent_id.clone();
    f.registry.retain_shared_budget_demand_terminal(verified(&f,ended(&s,"expired",None)),c.clone()).await.unwrap();
    assert_eq!(released_in(&f,"owner_shared_budget_start_releases").await,vec![held]);
    let mut next=offer(&c,"next_message",1);next.received_at_ms=refused.received_at_ms+1;
    let plain=f.registry.prepare_shared_demand(proof_for(&f,&next),c.clone(),None).await;
    let plain=plain.as_ref().map(|s|s.disposition().clone()).map_err(|e|e.to_string());
    assert_eq!(plain,Err(Error::Conflict.to_string()));
    let saved=prepare(&f,&c,status(&f,&c,&next).await).await.unwrap();
    assert_eq!(*saved.disposition(),SharedDemandDisposition::StartPrepared);
    f.registry.shutdown().await.unwrap();
}

/// The budget start table as schema 114 left it: one start for each child
/// and predecessor, for good; no release rows and no second ends.
const SCHEMA114_BUDGET_STARTS: &str = "DROP TRIGGER owner_shared_budget_start_intent_place;DROP VIEW owner_shared_budget_start_releasable;DROP TABLE owner_shared_budget_start_releases;PRAGMA foreign_keys=OFF;
CREATE TABLE owner_shared_budget_start_intents_schema114 (
 id TEXT PRIMARY KEY,
 child_id TEXT NOT NULL REFERENCES owner_shared_runtime_lifetimes(child_id),
 profile TEXT NOT NULL CHECK(length(profile)=64),
 device_id TEXT NOT NULL,
 device_generation INTEGER NOT NULL CHECK(device_generation>0 AND device_generation<=9007199254740991),
 predecessor_digest TEXT NOT NULL CHECK(length(predecessor_digest)=64),
 original_runtime_digest TEXT NOT NULL CHECK(length(original_runtime_digest)=64),
 original_runtime_payload TEXT NOT NULL CHECK(json_valid(original_runtime_payload) AND length(CAST(original_runtime_payload AS BLOB))<=16384),
 input_payload TEXT NOT NULL CHECK(json_valid(input_payload) AND length(CAST(input_payload AS BLOB))<=16384),
 input_digest TEXT NOT NULL CHECK(length(input_digest)=64),
 ledger_revision INTEGER NOT NULL UNIQUE CHECK(ledger_revision>0 AND ledger_revision<=9007199254740991),
 UNIQUE(child_id,predecessor_digest)
) STRICT;
INSERT INTO owner_shared_budget_start_intents_schema114 SELECT * FROM owner_shared_budget_start_intents;
DROP TABLE owner_shared_budget_start_intents;
ALTER TABLE owner_shared_budget_start_intents_schema114 RENAME TO owner_shared_budget_start_intents;
CREATE TRIGGER owner_shared_budget_start_intents_immutable BEFORE UPDATE ON owner_shared_budget_start_intents BEGIN SELECT RAISE(ABORT,'immutable budget start intent'); END;
CREATE TRIGGER owner_shared_budget_start_intents_retained BEFORE DELETE ON owner_shared_budget_start_intents BEGIN SELECT RAISE(ABORT,'retained budget start intent'); END;
DROP TABLE owner_shared_demand_second_terminals;
PRAGMA foreign_keys=ON;PRAGMA user_version=114;";

#[tokio::test]
async fn budget_execution_v3_schema115_gives_back_the_place_an_ended_message_held(){
    let(mut f,c)=synthetic().await;
    let refused=offer(&c,"refused_before_upgrade",1);let s=status(&f,&c,&refused).await;
    let held=prepare(&f,&c,s.clone()).await.unwrap().input().unwrap().start.start_intent_id.clone();
    f.registry.retain_shared_budget_demand_terminal(verified(&f,ended(&s,"expired",None)),c.clone()).await.unwrap();
    assert_eq!(released_in(&f,"owner_shared_budget_start_releases").await,vec![held.clone()]);
    assert!(store_sql(&f,"DELETE FROM owner_shared_budget_start_releases".into()).await.unwrap_err().contains("retained budget start release"));
    assert!(store_sql(&f,"UPDATE owner_shared_budget_start_releases SET demand_id=demand_id".into()).await.unwrap_err().contains("immutable budget start release"));
    f.registry.shutdown().await.unwrap();
    // The rows the previous schema saved: the same start and end, no release.
    let path=f._temp.path().join("owner/domain.sqlite3");
    let unique_pair=|db:&Connection|->bool {db.query_row("SELECT EXISTS(SELECT 1 FROM pragma_index_list('owner_shared_budget_start_intents') l JOIN pragma_index_info(l.name) i WHERE l.\"unique\"=1 AND i.name='predecessor_digest')",[],|r|r.get(0)).unwrap()};
    let db=Connection::open(&path).unwrap();
    db.execute_batch(SCHEMA114_BUDGET_STARTS).unwrap();
    assert!(unique_pair(&db),"schema 114's unique pair");
    drop(db);
    f.registry=OwnerResourceRegistry::open_root(&f._temp.path().join("owner")).unwrap();
    let db=Connection::open(&path).unwrap();
    assert_eq!(db.pragma_query_value::<u64,_>(None,"user_version",|r|r.get(0)).unwrap(),115);
    assert!(!unique_pair(&db),"the place is the trigger's now");
    let decisions:u64=db.query_row("SELECT count(*) FROM owner_shared_budget_demand_decisions d JOIN owner_shared_budget_start_intents i ON i.id=d.start_intent_id",[],|r|r.get(0)).unwrap();
    assert_eq!(decisions,1,"the rebuilt table keeps the start its decision names");
    drop(db);
    assert_eq!(released_in(&f,"owner_shared_budget_start_releases").await,vec![held],"the upgrade gives the place back");
    let mut next=offer(&c,"next_message",1);next.received_at_ms=refused.received_at_ms+1;
    let saved=prepare(&f,&c,status(&f,&c,&next).await).await.unwrap();
    assert_eq!(*saved.disposition(),SharedDemandDisposition::StartPrepared);
    f.registry.shutdown().await.unwrap();
}
