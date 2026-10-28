#!/usr/bin/env python3
"""Pinned VM checks; native pixels and live effects are separate evidence."""
import argparse,json,subprocess
from pathlib import Path
from check_agent_admission import APP,PREFIX

def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 s=(APP/'bundle/main.splash').read_text();helpers=s[s.index('A.cleanup_enabled = fn('):s.index('A.setup_enabled = fn(')]
 helpers+='\n'+'\n'.join(line for line in s.splitlines() if line.startswith(('A.name_not_applied = fn(', 'A.is_task_operation=fn(')))+'\n'
 pages=s[s.index('        if H.page == "agent_removal" {mod.widgets.'):s.index('        if H.page == "agent_work_list" {mod.widgets.')]
 fixture='''
H.session={features:["agent-cleanup-v1","agent-settlement-v1"]}
H.removal_id="request" H.removal_project=nil H.removal_after=nil H.removal_page=1 H.removal_confirmation="" H.removal_details=false H.page="agent_removal"
let r={requestId:"request" projectName:"Project Alpha" agentName:"Writer" requestRevision:7 admissionState:"revoked" canCancel:false canRevoke:false centralFenced:true nativeFence:nil processObservation:{state:"stopped"} matrix:{state:"fenced" cleanupObservedAtUtc:"2026-10-17 00:00:00 UTC"} usage:{periodKey:"2026-10" knownConsumedTokens:40 retainedTokensFloor:100 fenceUsageComplete:false requiresReconciliation:true}}
H.removal={protocol:"hagency-management-v1" removal:r observedAtUtc:"2026-10-17 00:00:00 UTC"}
let original=H.removal
let s={requestId:"request" requestRevision:7 fenceReceiptDigest:"original_digest" pendingCommandId:nil latestReceipt:nil settlementCurrent:false canRequest:true blockingReason:nil knownConsumedTokens:40 retainedTokensFloor:100}
let receipt={requestId:"request" commandId:"mst_previous" ledgerRevision:20 state:"settled" processState:"stopped" knownConsumedTokens:40 retainedTokens:40 unresolvedTokens:0 usageComplete:true}
H.settlement={protocol:"hagency-management-v1" settlement:s observedAtUtc:"2026-10-17 00:00:01 UTC"}
A.navigate=fn(page){H.page=page}
'''
 cases={
 'manual_recheck':('', 'assert(button("settlement_recheck").enabled)\nbutton("settlement_recheck").on_click()\nassert(H.calls.len()==1 && H.calls[0].service=="hagency.agents.settlement.request")\nassert(H.calls[0].args.requestId=="request" && H.calls[0].args.expectedRequestRevision==7 && H.calls[0].args.expectedFenceReceiptDigest=="original_digest")'),
 'pending':('s.pendingCommandId="mst_pending" s.canRequest=false', 'assert(has(A.t("Waiting for the original device","等待原执行设备核算")))\nassert(!button("settlement_recheck").enabled)\nA.recheck_settlement()\nassert(H.calls.len()==0)'),
 'current':('s.latestReceipt=receipt s.settlementCurrent=true s.canRequest=false s.retainedTokensFloor=40', 'assert(has(A.t("Accounting complete for recorded evidence","已根据当前记录完成核算")))\nassert(has("40 token") && !has("100 token"))\nassert(!has(A.t("Cleanup accounting incomplete","清理核算尚未完成")))\nassert(!button("settlement_recheck").enabled)\nA.recheck_settlement()\nassert(H.calls.len()==0)'),
 'original_details':('s.latestReceipt=receipt s.settlementCurrent=true s.canRequest=false H.removal_details=true', 'assert(has(A.t("Accounting complete for recorded evidence","已根据当前记录完成核算")))\nassert(has(A.t("Cleanup accounting incomplete","清理核算尚未完成")))\nassert(has(A.t("Original cleanup receipt","原始清理回执")))'),
 'stale_proof':('s.latestReceipt=receipt s.knownConsumedTokens=60', 'assert(has(A.t("Newer evidence needs reconciliation","新记录需要重新核算")))\nassert(button("settlement_recheck").enabled && has("60 token") && has("100 token"))'),
 'unresolved':('s.latestReceipt=receipt receipt.state="unresolved" receipt.usageComplete=false receipt.unresolvedTokens=60 receipt.retainedTokens=100', 'assert(has(A.t("Unresolved usage is still retained","未核清的用量仍保留记账")))\nassert(has("60 token") && has("100 token"))'),
 'native_fence_pending':('s.fenceReceiptDigest=nil s.canRequest=false', 'assert(has(A.t("Waiting for the device cleanup receipt","等待设备清理回执")))\nA.recheck_settlement()\nassert(H.calls.len()==0)'),
 'unknown_amount':('s.knownConsumedTokens=nil s.retainedTokensFloor=nil', 'assert(has(A.t("Unknown","未知")))\nassert(!has("0 token"))'),
 'failed_read':('H.settlement=nil', 'assert(has(A.t("Settlement could not be read. Refresh to try again.","未能读取结算，请刷新后重试。")))\nassert(button("settlement_recheck")==nil)'),
 'saved_command':('H.command={service:"hagency.agents.settlement.request" args:{commandId:"same"}}', 'assert(!button("settlement_recheck").enabled)\nA.recheck_settlement()\nassert(H.calls.len()==0 && H.command.args.commandId=="same")'),
 'foreign_context':('H.removal_id="foreign"', 'A.recheck_settlement()\nassert(H.calls.len()==0)'),
 'older_server':('H.session.features=["agent-cleanup-v1"]', 'assert(button("settlement_recheck")==nil)\nA.open_settlement("request")\nassert(H.calls.len()==0)'),
 'refresh_read_only':('', 'button("removal_refresh").on_click()\nassert(H.settlement==nil && H.calls[0].service=="hagency.agents.removal.get")\nH.callback(original)\nassert(H.calls[1].service=="hagency.agents.settlement.get")\nH.callback({protocol:"hagency-management-v1" settlement:{requestId:"foreign"}})\nassert(H.settlement==nil && H.notice!="")\nH.callback({protocol:"hagency-management-v1" settlement:s})\nassert(H.settlement.settlement.requestId=="request" && H.calls.len()==2)'),
 'navigate_during_read':('', 'A.open_settlement("request")\nH.page="overview"\nH.callback({protocol:"hagency-management-v1" settlement:s})\nassert(H.settlement==nil && H.page=="overview")'),
 }
 results=[]
 def run(name,code):
  path=a.output/(name+'.splash');path.write_text(code+'\ntrue\n');r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(a.output/(name+'.log')).write_text(r.stdout);results.append({'case':name,'passed':r.returncode==0})
 for lang in ['en','zh']:
  for name,(state,checks) in cases.items():
   run(name+'-'+lang,PREFIX+helpers+fixture+('A.t=fn(en,zh){return zh}\n' if lang=='zh' else '')+state+'\nlet root=View{'+pages+'}\ncollect(root)\n'+checks)
 retry=s[s.index('A.retry_command = fn('):s.index('A.enroll = fn(')]
 for outcome in ['success','unknown']:
  checks='H.command={service:"hagency.agents.settlement.request" args:{commandId:"original" requestId:"request" expectedRequestRevision:7 expectedFenceReceiptDigest:"digest"}}\nA.retry_command()\nassert(H.calls[0].args.commandId=="original")\n'
  checks+=('H.host_callback({is_ok:true data:{requested:{requestId:"request" commandId:"mst_different"}}})\nassert(H.command==nil && H.page=="agent_removal" && H.calls[1].service=="hagency.agents.removal.get" && H.ack=="original")' if outcome=='success' else 'H.host_callback({is_ok:false error:"management_outcome_unknown"})\nassert(H.command.args.commandId=="original" && H.settlement==nil)')
  run('retry-'+outcome,PREFIX+helpers+fixture+retry+checks)
 recover=s[s.index('A.recover_native = fn('):s.index('A.saved_label = fn(')]
 run('native_recovery',PREFIX+helpers+fixture+recover+'A.recover_native({commandId:"original" operation:"agent_settlement_request"})\nassert(H.calls[0].service=="hagency.intent.retry" && H.calls[0].args.commandId=="original")\nH.callback({operation:"agent_settlement_request" data:{requested:{requestId:"request" commandId:"mst_different"}}})\nassert(H.page=="agent_removal" && H.removal_id=="request" && H.calls[1].service=="hagency.agents.removal.get" && H.ack=="original")')
 (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
