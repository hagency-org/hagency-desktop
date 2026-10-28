#!/usr/bin/env python3
"""Human top-up handlers from the entire shipped bundle, EN/CN; no HTTP effects."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
MANIFEST=json.loads((APP/'bundle/manifest.json').read_text())
for capability in ['hagency.agents.budget.get','hagency.agents.topups.list','hagency.agents.topups.get','hagency.agents.topups.request','hagency.agents.topups.decide','hagency.agents.topups.cancel']:
    assert capability in MANIFEST['capabilities'], capability
SETUP='''
let H=mod.hagency let A=mod.budget_actions
A.redraw=fn(){return nil} H.busy=false H.ready=true H.notice="" H.command=nil H.saved_commands=[]
H.session={account:"@manager:test" features:["agent-admission-v1","agent-cleanup-v1"]}
let calls=[] let callbacks=[]
A.request=fn(service,args,done){calls.push({service:service args:args}) callbacks.push(done)}
A.submit=fn(service,args){calls.push({service:service args:args})}
let budget={requestId:"request_original" requestRevision:3 assignmentGeneration:2 assignmentId:"assignment" agentReservationId:"reservation" originalCeilingTokens:4096 appliedDeltaTokens:1024 effectiveCeilingTokens:5120 budgetRevision:7 budgetDigest:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" knownConsumedTokens:4813 conservativeCommitmentTokens:6144 nativeRetainedTokens:nil nativeRemainingTokens:nil usageComplete:nil executionSupported:false canRequest:true blockingReason:nil}
let topup={id:"topup_original" requestId:"request_original" authorityRevision:4 revision:2 assignmentId:"assignment" addTokens:1024 purpose:"Continue writing" expectedCeilingTokens:4096 state:"approved" nativeState:"unknown" canDecide:true canCancel:true expiresAtUtc:"2026-10-16 12:00:00 UTC"}
H.agent_budget={supported:true budget:budget observedAtUtc:"2026-10-16 11:00:00 UTC"}
H.budget_request_id="request_original" H.budget_agent_name="Writer" H.budget_topups={topUps:{items:[] nextAfter:nil}}
H.budget_topup={topUp:topup observedAtUtc:"2026-10-16 11:00:00 UTC"} H.budget_topup_id="topup_original"
let widgets=[] let visit=fn(node){if !node.is_object(){return} for index key child in node{if key.is_string() || key.is_id(){widgets.push({name:key.to_string() value:child})} visit(child)}}
let find=fn(name){for row in widgets{if row.name==name{return row.value}} return nil}
'''
CASES={
'approval_rejection_cancellation_have_distinct_explanations': '''
A.review_topup_action("approve") let approve=A.topup_review_hint()
assert(approve==A.t("Approval authorizes this increase. The execution device must confirm application before it can be used. Current resource authority and budget are checked when submitted.","批准后允许追加此额度，执行设备确认应用后才能使用。提交时会核对当前资源权限与预算。"))
A.review_topup_action("reject") let reject=A.topup_review_hint() assert(reject!=approve)
A.review_topup_action("cancel") assert(A.topup_review_hint()!=reject && A.topup_review_hint()!=approve && calls.len()==0)
assert(A.topup_application_hint("applied")!=A.topup_application_hint("unknown"))
assert(A.topup_application_hint("applied")==A.t("Capacity received. This does not confirm that a task is running. Check the task status and errors in the Agent chat.","额度已到账，不代表任务已开始运行。请在 Agent 聊天中查看任务状态和错误提示。"))
''',
'manager_request_has_exact_snapshot': '''
A.begin_topup() H.budget_tokens="1024" H.budget_purpose=" Continue writing " A.review_topup()
assert(calls.len()==0 && H.budget_confirmation.args.expectedRequestRevision==3 && H.budget_confirmation.args.expectedAssignmentGeneration==2 && H.budget_confirmation.args.expectedBudgetRevision==7 && H.budget_confirmation.args.expectedBudgetDigest==budget.budgetDigest)
A.apply_topup() assert(calls.len()==1 && calls[0].service=="hagency.agents.topups.request" && calls[0].args.requestId=="request_original" && calls[0].args.addTokens==1024 && calls[0].args.purpose=="Continue writing")
''',
'cancel_review_has_no_effect_and_only_current_roles_act': '''
A.review_topup_action("cancel") H.budget_confirmation=nil assert(calls.len()==0)
topup.canDecide=false A.review_topup_action("approve") assert(H.budget_confirmation==nil)
A.review_topup_action("cancel") assert(H.budget_confirmation.args.topUpId=="topup_original" && H.budget_confirmation.args.expectedRevision==2)
A.apply_topup() assert(calls.len()==1 && calls[0].service=="hagency.agents.topups.cancel")
''',
'admin_decisions_keep_authority_revision': '''
topup.canCancel=false A.review_topup_action("approve") A.apply_topup()
assert(calls.len()==1 && calls[0].args.expectedAuthorityRevision==4 && calls[0].args.expectedRevision==2 && calls[0].args.approve)
A.review_topup_action("reject") A.apply_topup() assert(calls.len()==2 && !calls[1].args.approve)
''',
'unknown_saved_request_blocks_replacement_but_read_is_allowed': '''
H.saved_commands=[{operation:"agent_top_up_request" commandId:"saved" reply:nil}]
A.begin_topup() A.review_topup_action("approve") A.apply_topup() assert(calls.len()==0)
A.open_agent_budget("request_original","Writer",nil,1) assert(calls.len()==1 && calls[0].service=="hagency.agents.budget.get")
''',
'old_server_unsupported_does_not_query_or_mutate': '''
A.open_agent_budget("request_original","Writer",nil,1) callbacks[0]({supported:false budget:nil observedAtUtc:nil}) A.begin_topup() A.review_topup() A.apply_topup()
assert(calls.len()==1 && !H.agent_budget.supported)
''',
'exact_page_cursor_and_wrong_request_refused': '''
A.open_agent_budget("request_original","Writer","topup_cursor",2) callbacks[0]({supported:true budget:budget observedAtUtc:"time"})
assert(calls.len()==2 && calls[1].service=="hagency.agents.topups.list" && calls[1].args.after=="topup_cursor" && calls[1].args.limit==16)
A.open_agent_budget("request_original","Writer",nil,1) budget.requestId="wrong" callbacks[2]({supported:true budget:budget}) assert(H.agent_budget==nil && calls.len()==3)
''',
'invalid_amounts_never_create_confirmation': '''
A.begin_topup() H.budget_purpose="Reason"
for value in ["0","-1","1.5","9007199254740992","nonsense"]{H.budget_tokens=value A.review_topup() assert(H.budget_confirmation==nil)}
H.budget_tokens="1" H.budget_purpose=" " A.review_topup() assert(H.budget_confirmation==nil && calls.len()==0)
''',
'lost_reply_retries_original_without_new_command': '''
host.request=fn(service,args,done){calls.push({service:service args:args.to_json().parse_json()}) callbacks.push(done)}
A.save=fn(){return true} let ack=[] A.acknowledge=fn(id){ack.push(id)}
H.command={service:"hagency.agents.topups.request" args:{commandId:"saved_original" requestId:"request_original" expectedRequestRevision:3 expectedAssignmentGeneration:2 expectedBudgetRevision:7 expectedBudgetDigest:budget.budgetDigest addTokens:1024 purpose:"Continue writing"}}
A.retry_command() callbacks[0]({is_ok:false error:"management_outcome_unknown"})
assert(H.command.args.commandId=="saved_original" && H.command.args.expectedBudgetRevision==7 && ack.len()==0)
A.retry_command() assert(calls.len()==2 && calls[1].args.to_json()==calls[0].args.to_json())
callbacks[1]({is_ok:true data:{receipt:{commandId:"saved_original"} topUp:topup}})
assert(H.command==nil && ack.len()==1 && ack[0]=="saved_original" && calls.len()==3 && calls[2].service=="hagency.agents.topups.get")
''',
'saved_native_recovery_returns_latest_without_replacement': '''
A.save=fn(){return true} A.acknowledge=fn(id){assert(id=="saved_original")}
A.recover_native({operation:"agent_top_up_request" commandId:"saved_original"})
assert(calls.len()==1 && calls[0].service=="hagency.intent.retry" && calls[0].args.commandId=="saved_original")
callbacks[0]({operation:"agent_top_up_request" data:{receipt:{commandId:"saved_original"} topUp:topup}})
assert(H.page=="agent_topup" && H.budget_request_id=="request_original" && calls.len()==2 && calls[1].service=="hagency.agents.topups.get")
''',
'auth_and_conflict_keep_original_request_pending': '''
host.request=fn(service,args,done){callbacks.push(done)} A.save=fn(){return true} let ack=[] A.acknowledge=fn(id){ack.push(id)}
for error in ["management_scope_changed","management_command_conflict","management_agent_budget_invalid","agent_topup_not_found"]{
H.command={service:"hagency.agents.topups.decide" args:{commandId:"saved_original" topUpId:"topup_original" expectedRevision:2 expectedAuthorityRevision:4 approve:true}}
A.retry_command() callbacks[callbacks.len()-1]({is_ok:false error:error}) assert(H.command.args.commandId=="saved_original" && ack.len()==0)
}
''',
'first_confirmed_refusal_is_terminal_but_unknown_is_not': '''
assert(A.topup_not_applied("hagency.agents.topups.request","agent_budget_issuance_unavailable"))
assert(!A.topup_not_applied("hagency.agents.topups.request","management_outcome_unknown"))
assert(!A.topup_not_applied("hagency.agents.topups.request","management_command_conflict"))
assert(A.topup_native_state("applied")!=A.topup_state("approved"))
'''
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);source=a.bundle.read_text()
 start=source.index('        if H.page == "agent_budget" {mod.widgets.HagencyDetailCard{');end=source.index('        if H.page == "agent_control" {mod.widgets.HagencyDetailCard{',start);section=source[start:end]
 CASES['role_controls_and_accounting_only_amounts_render']='''
H.page="agent_budget" let tree=View{'''+section+'''} visit(tree)
assert(find("budget_request")!=nil && find("topup_first")==nil && find("topup_next")==nil)
assert(budget.nativeRemainingTokens==nil && budget.knownConsumedTokens==4813 && budget.effectiveCeilingTokens==5120 && !budget.executionSupported)
widgets=[] H.page="agent_topup" let top=View{'''+section+'''} visit(top)
assert(find("topup_approve")!=nil && find("topup_reject")!=nil && find("topup_cancel")!=nil)
find("topup_approve").on_click() assert(calls.len()==0 && H.budget_confirmation.args.expectedAuthorityRevision==4)
'''
 CASES['owner_capacity_is_explicit_and_pool_back_uses_pool_route']='''
H.page="agent_budget" budget.canRequest=false budget.canOwnerFund=true budget.canOwnerFundDirect=true H.session.features.push("resource-pool-agents-v1")
let tree=View{'''+section+'''} visit(tree)
assert(find("budget_request")!=nil)
find("budget_request").on_click() H.budget_tokens="1000000" H.budget_purpose=""
A.review_topup() assert(H.budget_confirmation.args.ownerFunded && H.budget_confirmation.args.addTokens==1000000)
A.apply_topup() assert(calls[0].service=="hagency.agents.topups.request" && calls[0].args.ownerFunded && calls[0].args.approveImmediately)
find("budget_back").on_click() assert(calls[1].service=="hagency.pools.agents.get" && calls[1].args.id=="request_original")
'''
 CASES['owner_direct_result_needs_no_second_approval']='''
H.page="agent_budget" budget.canRequest=false budget.canOwnerFund=true budget.canOwnerFundDirect=true
A.begin_topup() H.budget_tokens="1024" A.review_topup() assert(calls.len()==0)
A.apply_topup() A.apply_topup() assert(calls.len()==1 && calls[0].args.approveImmediately)
topup.ownerFunded=true topup.canDecide=false topup.canCancel=false topup.state="approved"
A.topup_received({topUp:topup observedAtUtc:"time"})
let tree=View{'''+section+'''} visit(tree)
assert(find("topup_approve")==nil && find("topup_reject")==nil && H.page=="agent_topup")
'''
 CASES['old_owner_server_never_silently_falls_back_to_self_approval']='''
budget.canOwnerFund=true budget.canRequest=true
A.begin_topup() H.budget_tokens="1024" H.budget_purpose="Reason" A.review_topup() A.apply_topup()
assert(calls.len()==0 && H.budget_confirmation==nil)
'''
 CASES['multiple_history_cards_keep_distinct_original_selection']='''
H.page="agent_budget" let other=topup.to_json().parse_json() other.id="topup_second"
H.budget_topups={topUps:{items:[topup,other] nextAfter:nil}}
let tree=View{'''+section+'''} visit(tree) let actions=[] for row in widgets{if row.name=="topup_open"{actions.push(row.value)}}
assert(actions.len()==2) actions[0].on_click() assert(calls[0].args.id=="topup_original") actions[1].on_click() assert(calls[1].args.id=="topup_second")
'''
 results=[]
 for lang in ['en','zh-CN']:
  for name,test in CASES.items():
   path=a.output/f'{lang}-{name}.splash';path.write_text(fixture(source+'\nmod.budget_actions=A\n',lang)+SETUP+test+'\ntrue\n')
   r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);path.with_suffix('.log').write_text(r.stdout);results.append({'language':lang,'case':name,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
