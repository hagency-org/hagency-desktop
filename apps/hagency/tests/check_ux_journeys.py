#!/usr/bin/env python3
"""Run the shipped journey helpers against explicit host-boundary responses.
These test navigation/authority/state continuity, not backend transactions or pixels.
"""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
SETUP='''
let H=mod.hagency let A=mod.ux_actions
let calls=[]
A.redraw=fn(){} A.save=fn(){return true}
H.session={account:"@manager:test" administrator:"@admin:test" canAdmitResources:false features:["project-requests-v1","project-allocation-review-v1","agent-admission-v1","project-setup-v1"]}
H.ready=true H.busy=false H.command=nil
let project={id:"project_one" state:"pending_approval" allocationStatus:"awaiting_approval" managerMxid:"@manager:test" coordinatorMxid:"@owner:test" request:{canDecide:false approverMxid:"@admin:test"}}
'''
CASES={
'number_and_unknown': '''
assert(A.amount(100000)=="100,000") assert(A.amount(1000)=="1,000") assert(A.amount(0)=="0") assert(A.amount(nil)!=A.amount(0))
assert(A.amount(1.5)=="1.5")
''',
'all_navigation_parents': '''
for page in ["account_prepare","account_enrollment","web_connection","provider_accounts","workspace","templates","tool_request"] {H.page=page assert(A.nav_parent()=="resources")}
for page in ["agent_home","personal_agent","agent_budget","agent_topup"] {H.page=page assert(A.nav_parent()=="agents")}
H.page="project_action" assert(A.nav_parent()=="projects") H.ux.origin="inbox" assert(A.nav_parent()=="inbox")
H.page="account_prepare" assert(A.nav_parent()=="resources")
''',
'approval_does_not_imply_readiness': '''
assert(A.project_actor(project)=="@admin:test")
let waiting=A.project_next(project) project.request.canDecide=true assert(A.project_next(project)!=waiting)
project.state="approved" H.ux.projectProgress=nil assert(A.project_plan(project).target=="setup")
H.ux.projectProgress={readiness:{capacityReady:false matrixReady:false administrator:nil}}
let needs_capacity=A.project_next(project)
H.ux.projectProgress.readiness.capacityReady=true assert(A.project_actor(project)=="@manager:test") assert(A.project_next(project)!=needs_capacity)
H.ux.projectProgress.readiness.administrator={mxid:"@manager:test"} assert(A.project_actor(project)=="@manager:test")
let needs_workspace=A.project_next(project) H.ux.projectProgress.readiness.matrixReady=true assert(A.project_next(project)!=needs_workspace)
project.state="rejected" assert(A.project_next(project)!=waiting)
''',
'next_step_uses_distinct_records': '''
H.project_selected=project project.state="approved"
A.open_allocations=fn(id,after,page){calls.push({kind:"allocation" id:id})}
A.open_project_setup=fn(id){calls.push({kind:"workspace" id:id})}
A.open_project_agents=fn(id){calls.push({kind:"agents" id:id})}
H.ux.projectProgress={readiness:{capacityReady:false matrixReady:false administrator:nil}}
A.project_continue() assert(calls[0].kind=="allocation" && calls[0].id=="project_one")
H.ux.projectProgress.readiness.capacityReady=true A.project_continue() assert(calls[1].kind=="workspace")
H.ux.projectProgress.readiness.administrator={mxid:"@manager:test"} H.ux.projectProgress.readiness.matrixReady=true A.project_continue() assert(calls[2].kind=="agents")
''',
'inbox_return_keeps_filter_and_both_cursors': '''
H.page="project_action" H.ux.origin="inbox" H.needs_action=false H.ux.inboxCursor="second_page" H.ux.agentCursor="agent_page"
A.request=fn(service,args,done){calls.push({service:service args:args}) if service=="hagency.inbox.list"{done({items:[] nextAfter:nil})} else {done({items:[] nextAfter:nil})}}
A.return_origin()
assert(H.page=="inbox" && H.ux.origin=="" && H.ux.restore)
assert(!H.needs_action && calls[0].args.after=="second_page" && !calls[0].args.needsMyAction)
assert(calls[1].args.after=="agent_page" && !calls[1].args.needsMyAction)
A.inbox_filter(true) assert(H.ux.inboxCursor==nil && H.ux.agentCursor==nil && H.needs_action)
''',
'partial_queue_is_never_exact_total': '''
assert(A.queue_count()=="")
H.ux.dashboard={requests:[{} {}] agents:[{}] requestAfter:"more" agentAfter:nil complete:true}
assert(A.queue_count()=="3+") H.ux.dashboard.requestAfter=nil assert(A.queue_count()=="3")
H.ux.dashboard.complete=false assert(A.queue_count()=="3+")
''',
'resource_resume_preserves_account': '''
H.ux.accountId="account_original" H.wizard.resource=nil H.ux.resourceJourney=true
A.open_account=fn(id,after){calls.push(id)}
A.begin_setup() assert(calls.len()==1 && calls[0]=="account_original")
H.command={commandId:"original" service:"hagency.accounts.prepare"} A.begin_setup() assert(calls.len()==1)
''',
'identity_reset_for_second_record': '''
H.page="project_action" H.project_selected=project let first=A.route_identity()
H.project_selected={id:"project_two"} assert(A.route_identity()!=first)
H.page="resource_detail" H.resource_id="resource_one" first=A.route_identity() H.resource_id="resource_two" assert(A.route_identity()!=first)
''',
'role_labels_do_not_grant_authority': '''
let original=H.session.to_json()
H.ux.dashboard={projects:[{managerMxid:"@manager:test" coordinatorMxid:"@owner:test"}] agents:[]}
let manager=A.role_label() H.session.account="@admin:test" assert(A.role_label()!=manager)
assert(!H.session.canAdmitResources)
H.session.account="@manager:test" assert(H.session.to_json()==original)
'''
}
CASES.update({
'closed_and_unavailable_project_has_no_dead_cta': '''project.state="approved" project.allocationStatus="resource_unavailable" assert(A.project_plan(project).target=="")''',
'eligible_replacement_needs_workspace_review': '''
project.state="approved" H.project_selected=project
H.ux.projectProgress={readiness:{capacityReady:false matrixReady:true eligibleAllocation:{id:"replacement"} administrator:{mxid:"@manager:test"} setup:{id:"original"}}}
assert(A.project_capacity()) assert(A.project_plan(project).target=="setup")
''',
'no_agents_feature_has_workspace_label_and_target': '''
project.state="approved" H.session.features=["project-requests-v1","project-setup-v1"]
H.ux.projectProgress={readiness:{capacityReady:true matrixReady:true administrator:{mxid:"@manager:test"}}}
assert(A.project_plan(project).target=="setup") assert(A.project_plan(project).label!=A.t("View project agents","查看项目 Agent"))
''',
'inbox_failure_never_claims_empty_success': '''
H.page="inbox" H.inbox=[{id:"old_filter"}] H.agents=[{id:"old_agent"}] H.ux.inboxReady=true
A.request=fn(service,args,done){calls.push(service)}
A.inbox_filter(false) assert(!H.ux.inboxReady && H.inbox.len()==0 && H.agents.len()==0)
''',
'failed_second_connection_does_not_show_first': '''
H.page="resource_detail" H.selected={contribution:{id:"connection_first"}} H.connection_project="previous_project"
A.request=fn(service,args,done){calls.push({service:service args:args})}
A.open_connection("connection_second")
assert(H.page=="action" && H.selected==nil && H.connection_project==nil)
assert(calls[0].args.id=="connection_second")
''',
'connection_response_identity_must_match': '''
H.page="resource_detail" H.selected=nil
A.request=fn(service,args,done){done({admission:{contribution:{id:"wrong_connection"}}})}
A.open_connection("expected_connection") assert(H.selected==nil && H.notice!="")
''',
'historical_agent_role_is_not_current_authority': '''
H.ux.dashboard={projects:[] agents:[{administratorMxid:"@manager:test" canDecide:false}]}
let previous=A.role_label()
H.ux.dashboard.agents[0].canDecide=true assert(A.role_label()!=previous)
assert(A.role_label()==A.t("Project manager","项目经理"))
H.ux.dashboard.projects=[project] assert(A.role_label()==A.t("Project manager","项目经理"))
assert(!H.session.canAdmitResources)
''',
'resource_connection_uses_exact_paged_match': '''
H.page="resource_detail" H.session.device="this_device"
let resource={id:"resource_second" framework:"codex" reasoning:"high" revision:2}
A.open_connection=fn(id){calls.push(id)}
A.request=fn(service,args,done){
 if args.after==nil{done({items:[{id:"first" resourceId:"resource_first" ownerMxid:H.session.account executorDeviceId:H.session.device verification:nil}] nextAfter:"page2" observedAtMs:0})}
 else{done({items:[{id:"second" resourceId:"resource_second" ownerMxid:H.session.account executorDeviceId:H.session.device verification:nil}] nextAfter:nil observedAtMs:0})}
}
A.resource_connection(resource) assert(calls.len()==1 && calls[0]=="second")
'''
})

CASES.update({
'enrollment_edit_clears_only_its_validation': """
A.enrollment_hide_notice=fn(){}
H.enrollment_draft={tokens:"0"}
H.notice=A.t("Enter a whole number of tokens greater than zero.","请输入大于零的整数 token 数量。")
A.enrollment_tokens_changed("42000") assert(H.enrollment_draft.tokens=="42000" && H.notice=="")
H.notice="authorization expired" A.enrollment_tokens_changed("44000") assert(H.notice=="authorization expired")
""",
'authorization_recovery_is_not_retry': """
H.ux.errorCode="management_permission_required" assert(A.must_reopen())
H.ux.errorCode="management_scope_changed" assert(A.must_reopen())
H.ux.errorCode="network_error" assert(!A.must_reopen())
"""
})

CASES.update({'lost_authorization_cannot_start_cached_agent_request': """
H.agent_readiness={canRequest:true requestContext:{}} H.page="agents"
H.ux.errorCode="management_permission_required" H.needs_signin=false
A.begin_agent() assert(H.page=="agents")
H.ux.errorCode="" H.needs_signin=true A.begin_agent() assert(H.page=="agents")
"""})

def main():
 ap=argparse.ArgumentParser();ap.add_argument('vm',type=Path);ap.add_argument('--output',type=Path,required=True);args=ap.parse_args();args.output.mkdir(parents=True,exist_ok=True)
 source=(APP/'bundle/main.splash').read_text();results=[]
 for lang in ['en','zh-CN']:
  for name,test in CASES.items():
   script=fixture(source+'\nmod.ux_actions=A\n',lang)+SETUP+test+'\ntrue\n'
   path=args.output/f'{name}-{lang}.splash';path.write_text(script)
   result=subprocess.run([str(args.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
   path.with_suffix('.log').write_text(result.stdout);results.append({'case':name,'language':lang,'passed':result.returncode==0})
 (args.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n')
 print(json.dumps({'passed':sum(x['passed'] for x in results),'total':len(results)}));raise SystemExit(not all(x['passed'] for x in results))
if __name__=='__main__':main()
