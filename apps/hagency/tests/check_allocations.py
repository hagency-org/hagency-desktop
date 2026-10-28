#!/usr/bin/env python3
"""Run real shipped allocation helpers/subtrees in the pinned VM (not pixels)."""
import argparse
import json
from pathlib import Path
import subprocess
from check_agent_admission import PREFIX, APP


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('vm', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    src = (APP / 'bundle/main.splash').read_text()
    helpers = src[src.index('A.allocations_enabled = fn('):src.index('A.agents_enabled = fn(')]
    helpers+='\n'+'\n'.join(line for line in src.splitlines() if line.startswith(('A.name_not_applied = fn(', 'A.is_task_operation=fn(')))+'\n'
    pages = src[src.index('        if H.page == "allocations" {'):src.index('        if H.page == "project_wizard" && A.projects_enabled()')]
    fixture = '''
H.session={features:["project-allocation-review-v1"]}
H.page="allocation_action" H.allocation_project="project" H.allocation_after=nil H.allocation_page=1 H.allocation_list=nil H.allocation_id="allocation" H.allocation_detail=nil H.allocation_confirmation="" H.allocation_from_inbox=false
A.navigate=fn(page){H.page=page}
A.open_project=fn(id){H.opened_project=id}
A.agent_not_applied=fn(service,code){return false}
A.agent_reason=fn(code){return code}
A.open_agent=fn(id){}
let balance={state:"reserved" retainedTokens:100}
let detail={projectName:"Project Alpha" resourceLabel:"Owner workstation" allocation:{id:"allocation" projectId:"project" requestId:"request" requestedTokens:100 periodKey:"2026-10" revision:7 state:"pending_coordinator" canDecide:true canRelease:false blockingReason:nil balance:nil observedUsage:nil requesterMxid:"@manager:test" coordinatorMxid:"@coordinator:test"} request:{id:"request" revision:3 canDecide:true} grant:{state:"active" totalTokens:700 nativeAvailableTokens:nil periodStartUtc:"2026-10-01 UTC" periodEndUtc:"2026-11-01 UTC" delegationExpiresAtUtc:"2026-10-31 UTC" ownerMxid:"@owner:test"}}
let reply={detail:detail observedAtUtc:"2026-10-16 UTC"}
let index={allocations:{items:[detail] nextAfter:nil} observedAtUtc:"2026-10-16 UTC"}
'''
    cases = {
        'old_reservation_protocol': ('H.session={features:["project-reservations-v1"]}', 'assert(!A.allocations_enabled())\nA.open_allocations("project",nil,1)\nassert(H.calls.len()==0)'),
        'old_server': ('H.session={}', '''assert(has(A.t("Project reservations are not enabled on this server.","此服务器尚未启用项目预留。")))
assert(!button("allocation_refresh").enabled)
A.open_allocation("allocation")
assert(H.calls.len()==0)'''),
        'loading': ('H.busy=true', 'assert(has(A.t("Reading allocation records…","正在读取分配记录…")))\nassert(!button("allocation_refresh").enabled)'),
        'read_failure': ('', 'assert(has(A.t("The record could not be read. Refresh or recover the original saved request.","未能读取记录，请刷新或恢复原始保存的请求。")))'),
        'coordinator': ('H.allocation_detail=reply', '''assert(has("Project Alpha") && has("Owner workstation"))
assert(has(A.t("Unknown","未知")+" / 700 token"))
assert(has(A.t("No applied native balance recorded","尚无已生效的原生余额记录")))
assert(button("allocation_approve")!=nil && button("allocation_release")==nil)
button("allocation_approve").on_click()
assert(H.allocation_confirmation=="approve" && H.calls.len()==0)'''),
        'approve_exact_request_revision': ('H.allocation_detail=reply H.allocation_confirmation="approve"', '''button("allocation_confirm").on_click()
assert(H.calls.len()==1 && H.calls[0].service=="hagency.allocations.decide")
assert(H.calls[0].args.expectedRevision==3 && H.calls[0].args.requestId=="request" && H.calls[0].args.approve)
let {requestedTokens=nil}=H.calls[0].args
assert(requestedTokens==nil)'''),
        'reject': ('H.allocation_detail=reply H.allocation_confirmation="reject"', '''button("allocation_confirm").on_click()
assert(H.calls[0].service=="hagency.allocations.decide" && !H.calls[0].args.approve)'''),
        'cancel_confirmation': ('H.allocation_detail=reply H.allocation_confirmation="approve"', '''button("allocation_cancel").on_click()
assert(H.allocation_confirmation=="" && H.calls.len()==0)'''),
        'manager_release_exact_allocation_revision': ('H.allocation_detail=reply detail.allocation.canDecide=false detail.request.canDecide=false detail.allocation.canRelease=true H.allocation_confirmation="release"', '''button("allocation_confirm").on_click()
assert(H.calls[0].service=="hagency.allocations.release" && H.calls[0].args.allocationId=="allocation" && H.calls[0].args.expectedRevision==7)'''),
        'read_only': ('H.allocation_detail=reply detail.allocation.canDecide=false detail.request.canDecide=false', '''assert(button("allocation_approve")==nil && button("allocation_release")==nil)
A.allocation_decide(true) A.allocation_release()
assert(H.calls.len()==0)'''),
        'request_flag_disagrees': ('H.allocation_detail=reply detail.request.canDecide=false', '''assert(button("allocation_approve")==nil)
A.allocation_decide(true)
assert(H.calls.len()==0)'''),
        'saved_command_blocks_new': ('H.allocation_detail=reply H.command={commandId:"original"}', '''assert(button("allocation_approve")==nil)
A.allocation_decide(true)
assert(H.calls.len()==0 && H.command.commandId=="original")'''),
        'release_blocked_retains_balance': ('H.allocation_detail=reply detail.allocation.state="release_blocked" detail.allocation.canDecide=false detail.request.canDecide=false detail.allocation.balance=balance detail.allocation.observedUsage={knownConsumedTokens:17 retainedTokensFloor:100 requiresReconciliation:true}', '''assert(has("100 token") && has("17 token"))
assert(has(A.t("Retained tokens","保留 token")) && has(A.t("Known consumed","已知消耗")) && has(A.t("Retained lower bound","保留量下限")))
assert(has(A.t("Usage requires native reconciliation","用量需要原生核算")))
assert(has(A.t("Release requires reconciliation","释放前需要核算")))'''),
        'expired_history': ('H.allocation_detail=reply detail.allocation.state="expired" detail.allocation.canDecide=false detail.request.canDecide=false', 'assert(has(A.t("Allocation expired","分配已过期")))\nassert(button("allocation_refresh")!=nil)'),
        'inbox_back': ('H.allocation_detail=reply H.allocation_from_inbox=true', 'button("allocation_back").on_click()\nassert(H.page=="inbox")'),
        'mismatched_detail': ('H.allocation_detail=reply', '''A.open_allocation("original")
assert(H.allocation_detail==nil)
H.callback(reply)
assert(H.allocation_detail==nil && H.notice!="")'''),
        'empty_list': ('H.page="allocations" H.allocation_list=index index.allocations.items=[]', 'assert(has(A.t("No allocations in this view","当前视图暂无分配")))\nassert(button("allocation_list_open")==nil)'),
        'paged_list': ('H.page="allocations" H.allocation_list=index index.allocations.nextAfter="allocation"', '''H.command={commandId:"original"}
button("allocation_next").on_click()
assert(H.calls[0].service=="hagency.allocations.list" && H.calls[0].args.projectId=="project" && H.calls[0].args.after=="allocation" && H.calls[0].args.limit==16)
assert(H.allocation_page==2 && H.allocation_list==nil && H.command.commandId=="original")'''),
        'exact_list_selection': ('H.page="allocations" H.allocation_list=index', '''button("allocation_list_open").on_click()
assert(H.calls[0].service=="hagency.allocations.get" && H.calls[0].args.id=="allocation")'''),
    }
    results=[]
    for lang in ['en','zh']:
        for name,(state,checks) in cases.items():
            script=PREFIX+helpers+fixture
            if lang=='zh': script+='A.t=fn(en,zh){return zh}\n'
            script+=state+'\nlet root=View{'+pages+'}\ncollect(root)\n'+checks+'\n'
            script+='for id in ["execute","grant_approve","launch_agent","top_up"] {assert(button(id)==nil)}\ntrue\n'
            path=args.output/(lang+'-'+name+'.splash');path.write_text(script)
            result=subprocess.run([str(args.vm),'eval',str(path)],capture_output=True,text=True)
            results.append({'case':lang+'-'+name,'passed':result.returncode==0,'output':result.stdout+result.stderr})
    # Real shipped retry handler preserves an unknown original, clears stale UI,
    # and uses the allocation's latest read only after a known native result.
    retry=src[src.index('A.retry_command = fn('):src.index('A.enroll = fn(')]
    for error,cleared in [('management_outcome_unknown',False),('owner_authorization_required',False),('management_command_conflict',False),('allocation_exceeds_grant',True)]:
        script=PREFIX+helpers+fixture+retry+'''H.allocation_detail=reply H.command={service:"hagency.allocations.decide" args:{commandId:"original" requestId:"request" expectedRevision:3 approve:true}}
A.retry_command()
'''+f'H.host_callback({{is_ok:false error:{json.dumps(error)}}})\nassert(H.allocation_detail==nil)\nassert((H.command==nil)=={str(cleared).lower()})\ntrue\n'
        path=args.output/('retry-'+error+'.splash');path.write_text(script)
        result=subprocess.run([str(args.vm),'eval',str(path)],capture_output=True,text=True)
        results.append({'case':'retry-'+error,'passed':result.returncode==0,'output':result.stdout+result.stderr})
    recovery=src[src.index('A.recover_native = fn('):src.index('A.saved_label = fn(')]
    script=PREFIX+helpers+fixture+recovery+'''A.recover_native({commandId:"original" operation:"allocation_decide"})
assert(H.calls[0].service=="hagency.intent.retry" && H.calls[0].args.commandId=="original")
H.callback({operation:"allocation_decide" data:{allocation:{id:"allocation" state:"released"}}})
assert(H.ack=="original" && H.command==nil)
assert(H.calls[1].service=="hagency.allocations.get" && H.calls[1].args.id=="allocation")
assert(H.allocation_detail==nil)
H.callback(reply)
assert(H.allocation_detail.detail.allocation.id=="allocation")
true
'''
    path=args.output/'recover-original.splash';path.write_text(script)
    result=subprocess.run([str(args.vm),'eval',str(path)],capture_output=True,text=True)
    results.append({'case':'recover-original','passed':result.returncode==0,'output':result.stdout+result.stderr})
    (args.output/'report.json').write_text(json.dumps(results,indent=2))
    bad=[v for v in results if not v['passed']]
    print(json.dumps({'passed':len(results)-len(bad),'failed':bad},indent=2))
    raise SystemExit(bool(bad))

if __name__=='__main__': main()
