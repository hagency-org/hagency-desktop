#!/usr/bin/env python3
"""Pinned VM checks of shipped cleanup views and original command recovery.

Assertions inspect actual script children/callbacks; no native pixel acceptance.
"""
import argparse,json,subprocess
from pathlib import Path
from check_agent_admission import APP,PREFIX

def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 s=(APP/'bundle/main.splash').read_text();helpers=s[s.index('A.cleanup_enabled = fn('):s.index('A.setup_enabled = fn(')]
 helpers+='\n'+'\n'.join(line for line in s.splitlines() if line.startswith(('A.name_not_applied = fn(', 'A.is_task_operation=fn(')))+'\n'
 pages=s[s.index('        if H.page == "agent_removals" {mod.widgets.'):s.index('        if H.page == "agent_work_list" {mod.widgets.')]
 fixture='''
mod.rinx_icons.inbox=5
H.session={features:["agent-cleanup-v1"]}
H.removals=nil H.removal=nil H.removal_id="request" H.removal_project=nil H.removal_after=nil H.removal_page=1 H.removal_confirmation="" H.removal_details=true H.page="agent_removal"
let row={requestId:"request" projectId:"project" projectName:"Project Alpha" proposedAgentId:"agent_proposed" agentName:"Writer" requestRevision:7 admissionState:"approved_waiting_executor" canCancel:false canRevoke:false centralFenced:false nativeFence:nil processObservation:nil matrix:nil usage:{periodKey:"2026-10" knownConsumedTokens:nil retainedTokensFloor:nil fenceUsageComplete:nil requiresReconciliation:false}}
let detail={protocol:"hagency-management-v1" removal:row observedAtUtc:"2026-10-16 00:00:00 UTC"}
let index={protocol:"hagency-management-v1" observedAtUtc:"2026-10-16 00:00:00 UTC" requests:{items:[{requestId:"request" projectId:"project" projectName:"Project Alpha" agentName:"Writer"}] nextAfter:nil}}
A.navigate=fn(page){H.page=page}
'''
 cases={
 'compact_history':('H.removal=detail H.removal_details=false row.nativeFence={deliveryState:"recorded" nativeState:"fenced" processState:"unknown"} row.processObservation={state:"stopped"} row.matrix={state:"fenced" cleanupObservedAtUtc:"2026-10-17 02:22:00 UTC"} row.usage.fenceUsageComplete=false', 'assert(has(A.t("Stop observed","已记录到停止")))\nassert(has(A.t("Project-room exit observed","已确认退出项目房间")))\nassert(!has(A.t("Process at cleanup","清理时进程")))\nassert(has(A.t("Cleanup accounting incomplete","清理核算尚未完成")))\nbutton("removal_details").on_click()\nassert(H.removal_details && H.removal.removal.requestId=="request" && H.calls.len()==0)'),
 'historical_unknown':('H.removal=detail row.nativeFence={deliveryState:"recorded" nativeState:"fenced" processState:"unknown"} row.processObservation={state:"stopped"} row.usage.fenceUsageComplete=false', 'assert(has(A.t("Result recorded","结果已记录")))\nassert(has(A.t("Outcome unknown","结果未知")))\nassert(has(A.t("Stop observed","已记录到停止")))\nbutton("removal_details").on_click()\nassert(!H.removal_details && H.calls.len()==0)'),
 'matrix_unconfirmed':('H.removal=detail row.matrix={state:"fenced" cleanupObservedAtUtc:nil}', 'assert(!has(A.t("Project-room exit observed","已确认退出项目房间")))'),
 'old_server':('H.session={}','assert(has(A.t("Not enabled on this server","此服务器尚未启用")))\nA.open_removal("request")\nassert(H.calls.len()==0)'),
 'unknown':('H.removal=detail','assert(has(A.t("Unknown","未知")))\nassert(has(A.t("No cleanup accounting receipt","尚无清理核算回执")))\nassert(button("removal_cancel")==nil && button("removal_revoke")==nil)\nassert(!has("0 token"))'),
 'independent_stages':('H.removal=detail row.centralFenced=true row.nativeFence={deliveryState:"pending" nativeState:nil processState:nil} row.processObservation={kind:"shared_text" state:"live" ledgerRevision:3} row.matrix={state:"cleanup_pending" cleanupObservedAtUtc:nil} row.usage.retainedTokensFloor=100','assert(has(A.t("New work is blocked by the server","服务器已阻止新任务")))\nassert(has(A.t("Live at the recorded observation","记录时仍在运行")))\nassert(has("100 token"))\nassert(has(A.t("No cleanup accounting receipt","尚无清理核算回执")))'),
 'qualified_snapshot':('H.removal=detail row.processObservation={kind:"qualification" state:"qualified" ledgerRevision:4} row.nativeFence={deliveryState:"completed" nativeState:"absent_fenced" processState:"not_started"}', 'assert(has(A.t("Qualification recorded","已记录资格检查")))\nassert(has(A.t("No reservation · new work blocked","未创建预留 · 已阻止新任务")))'),
 'late_consumed':('H.removal=detail row.usage.knownConsumedTokens=150 row.usage.retainedTokensFloor=150 row.usage.fenceUsageComplete=true row.usage.requiresReconciliation=true','assert(has("150 token"))\nassert(has(A.t("Later usage needs reconciliation with the cleanup receipt.","后续用量需要与清理回执重新核算。")))\nassert(has(A.t("Complete in the cleanup receipt","清理回执中的核算完整")))'),
 'partial_zero':('H.removal=detail row.usage.knownConsumedTokens=0 row.usage.retainedTokensFloor=100 row.usage.fenceUsageComplete=false','assert(has("0 token") && has("100 token"))\nassert(has(A.t("Cleanup accounting incomplete","清理核算尚未完成")))'),
 'manager':('H.removal=detail row.canCancel=true','assert(button("removal_cancel")!=nil && button("removal_revoke")==nil)\nbutton("removal_cancel").on_click()\nassert(H.removal_confirmation=="cancel" && H.calls.len()==0)\nA.confirm_removal()\nassert(H.calls[0].service=="hagency.agents.cancel" && H.calls[0].args.expectedRevision==7 && H.calls[0].args.requestId=="request")'),
 'administrator':('H.removal=detail row.canRevoke=true','assert(button("removal_cancel")==nil && button("removal_revoke")!=nil)\nbutton("removal_revoke").on_click()\nassert(H.removal_confirmation=="revoke" && H.calls.len()==0)\nA.confirm_removal()\nassert(H.calls[0].service=="hagency.agents.revoke" && H.calls[0].args.expectedRevision==7)'),
 'confirm_cancel':('H.removal=detail row.canRevoke=true H.removal_confirmation="revoke"','assert(button("removal_confirm")!=nil)\nbutton("removal_keep").on_click()\nassert(H.removal_confirmation=="" && H.calls.len()==0)\nA.confirm_removal()\nassert(H.calls.len()==0)'),
 'pending_original':('H.removal=detail row.canRevoke=true H.command={commandId:"original"}','assert(button("removal_revoke")==nil && button("removal_confirm")==nil)\nH.removal_confirmation="revoke"\nA.confirm_removal()\nassert(H.calls.len()==0 && H.command.commandId=="original")'),
 'changed_role':('H.removal=detail H.removal_confirmation="revoke"','A.confirm_removal()\nassert(H.calls.len()==0)'),
 'refresh':('H.removal=detail','button("removal_refresh").on_click()\nassert(H.removal==nil && H.calls[0].service=="hagency.agents.removal.get")\nH.callback({protocol:"hagency-management-v1" removal:{requestId:"foreign"}})\nassert(H.removal==nil && H.notice!="")\nH.callback(detail)\nassert(H.removal.removal.requestId=="request")'),
 'list_issuance_off':('H.page="agent_removals" H.removals=index','assert(A.cleanup_enabled() && !A.agents_enabled())\nassert(has("Writer") && has("Project Alpha"))\nbutton("removal_open").on_click()\nassert(H.calls[0].service=="hagency.agents.removal.get" && H.calls[0].args.id=="request")'),
 'list_paging':('H.page="agent_removals" H.removals=index H.removal_project="project" index.requests.nextAfter="request"','button("removals_next").on_click()\nassert(H.removals==nil && H.removal_page==2)\nassert(H.calls[0].args.projectId=="project" && H.calls[0].args.after=="request" && H.calls[0].args.limit==16)'),
 'empty':('H.page="agent_removals" H.removals=index index.requests.items=[]','assert(has(A.t("No agent requests are visible in this scope.","当前访问范围内没有可见的 Agent 申请。")))'),
 }
 results=[]
 def run(name,script):
  path=a.output/(name+'.splash');path.write_text(script+'\ntrue\n');r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(a.output/(name+'.log')).write_text(r.stdout);results.append({'case':name,'passed':r.returncode==0})
 for lang in ['en','zh']:
  for name,(state,checks) in cases.items():
   run(name+'-'+lang,PREFIX+helpers+fixture+('A.t=fn(en,zh){return zh}\n' if lang=='zh' else '')+state+'\nlet root=View{'+pages+'}\ncollect(root)\n'+checks)
 # Actual recovery/retry handlers preserve original IDs and route cleanup even
 # when admission discovery is absent. Unknown failure retains the saved intent.
 retry=s[s.index('A.retry_command = fn('):s.index('A.enroll = fn(')]
 recover=s[s.index('A.recover_native = fn('):s.index('A.saved_label = fn(')]
 for operation in ['cancel','revoke']:
  for outcome in ['success','unknown']:
   checks='''H.command={service:"hagency.agents.OP" args:{commandId:"original" requestId:"request" expectedRevision:7}}
A.retry_command()
assert(H.calls[0].args.commandId=="original")
'''.replace('OP',operation)
   checks+=('H.host_callback({is_ok:true data:{agentRequest:agent}})\nassert(H.command==nil && H.page=="agent_removal" && H.calls[1].service=="hagency.agents.removal.get")' if outcome=='success' else 'H.host_callback({is_ok:false error:"management_outcome_unknown"})\nassert(H.command.args.commandId=="original" && H.removal==nil)')
   run(operation+'-'+outcome,PREFIX+helpers+fixture+retry+checks)
  run(operation+'-native',PREFIX+helpers+fixture+recover+'''A.recover_native({commandId:"original" operation:"agent_OP"})
assert(H.calls[0].service=="hagency.intent.retry" && H.calls[0].args.commandId=="original")
H.callback({operation:"agent_OP" data:{agentRequest:agent}})
assert(H.page=="agent_removal" && H.calls[1].service=="hagency.agents.removal.get")
'''.replace('OP',operation))
 start=s.index('            if A.cleanup_enabled() {mod.widgets.HagencyCard{');end=s.index('            if A.work_enabled()',start)
 run('restart_overview',PREFIX+helpers+fixture+'let root=View{'+s[start:end]+'}\ncollect(root)\nbutton("overview_cleanup").on_click()\nassert(H.calls[0].service=="hagency.agents.removal.list" && H.calls[0].args.projectId==nil)')
 (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
