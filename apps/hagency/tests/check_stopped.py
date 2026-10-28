#!/usr/bin/env python3
"""Evaluate shipped interrupted-task flows and original-command recovery in VM.
This is not native pixel, real provider or deployment acceptance.
"""
import argparse,json,subprocess
from pathlib import Path
from check_agent_admission import APP,PREFIX
p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
source=(APP/'bundle/main.splash').read_text()
helpers=source[source.index('A.stopped_attempt_valid=fn('):source.index('A.workspace_valid=fn(')]
retry=source[source.index('A.retry_command = fn('):source.index('A.enroll = fn(')]
recover=source[source.index('A.recover_native = fn('):source.index('A.saved_label = fn(')]
pages=source[source.index('        if H.page=="stopped_tasks" {'):source.index('        if H.page=="workspaces" {')]
fixture='''
let Down=3 mod.widgets.RINX_SPACING=1 mod.rinx_icons.inbox=1 mod.rinx_icons.close=2
H.page="stopped_task" H.stopped_resource="resource" H.stopped_engagement="engagement" H.stopped_dispatch="dispatch" H.stopped_workspace="" H.stopped_after="" H.stopped_offset=0 H.stopped_detail=nil H.stopped_review=nil H.stopped_result=nil H.stopped_list=nil H.stopped_mode="inspect" H.stopped_action="" H.stopped_note="" H.stopped_instruction="" H.ack=nil
A.is_task_operation=fn(operation){return false}
A.open_resource=fn(id){H.page="resource_detail" H.resource_id=id}
let row={attempt:{resourceId:"resource" engagementId:"engagement" dispatchId:"dispatch" agent:"Worker" projectId:"project" taskId:"task" fence:1 reason:"owned_runner_failure" inspectionAvailable:true sharedResourceAuthority:true} taskTitle:"Review output" taskStatus:"blocked" revision:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" resolvable:true canContinue:true entryCount:1 entries:[{relativePath:"result.txt" kind:"file" bytes:12 sha256:"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}] nextEntry:nil}
let reviewed={reviewCommandId:"review" inspectionId:"inspection" expiresAt:9999999999999 inspection:row}
let inspect_command={service:"hagency.stopped.review" args:{commandId:"review" resourceId:"resource" engagementId:"engagement" dispatchId:"dispatch" expectedRevision:row.revision}}
let command={service:"hagency.stopped.resolve" args:{commandId:"resolve" resourceId:"resource" engagementId:"engagement" dispatchId:"dispatch" reviewCommandId:"review" reviewRevision:row.revision action:"keep_blocked" operatorNote:"Reviewed output and effects" continuationInstruction:nil}}
let result={commandId:"resolve" resourceId:"resource" engagementId:"engagement" dispatchId:"dispatch" action:"keep_blocked" taskId:"task" taskStatus:"blocked" replacementId:nil resolvedAt:12345}
let page={resourceId:"resource" dispatches:[row.attempt] nextAfter:nil}
'''
views={
 'loading':('H.busy=true','assert(has(A.t("Reading inspection…","正在读取检查结果…"))) assert(!button("stopped_refresh").enabled)'),
 'unavailable':('','assert(!has(A.t("No recorded file entries","暂无文件记录"))) assert(button("stopped_review")==nil)'),
 'empty':('H.page="stopped_tasks" H.stopped_list=page page.dispatches=[]','assert(has(A.t("No unresolved tasks on this page","本页暂无未处理的任务")))'),
 'list_unavailable':('H.page="stopped_tasks"','assert(has(A.t("Task list unavailable. Refresh to try again.","任务列表暂不可用，请刷新重试。")))'),
 'open':('H.page="stopped_tasks" H.stopped_list=page','button("stopped_list_open").on_click() assert(H.calls.len()==1 && H.calls[0].service=="hagency.stopped.get" && H.calls[0].args.dispatchId=="dispatch")'),
 'shared':('H.page="stopped_tasks" H.stopped_list=page row.attempt.sharedResourceAuthority=false','assert(!button("stopped_list_open").enabled) button("stopped_list_open").on_click() assert(H.calls.len()==0)'),
 'no_evidence':('H.page="stopped_tasks" H.stopped_list=page row.attempt.inspectionAvailable=false','assert(!button("stopped_list_open").enabled) button("stopped_list_open").on_click() assert(H.calls.len()==0)'),
 'inspect':('H.stopped_detail=row','assert(has("result.txt")) button("stopped_review").on_click() assert(H.calls.len()==1 && H.calls[0].service=="hagency.stopped.review" && H.calls[0].args.expectedRevision==row.revision)'),
 'historical':('H.stopped_detail=row row.resolvable=false row.canContinue=false','assert(button("stopped_review")==nil) A.review_stopped() assert(H.calls.len()==0)'),
 'files_next':('H.stopped_detail=row row.entryCount=9 row.nextEntry=8','button("stopped_inventory_next").on_click() assert(H.calls[0].args.entryOffset==8)'),
 'choose':('A.stopped_review_received(reviewed)','assert(button("stopped_commit")==nil) assert(button("stopped_choose_continue")!=nil) button("stopped_choose_blocked").on_click() assert(H.stopped_action=="keep_blocked" && H.calls.len()==0)'),
 'lost_settle_only':('row.canContinue=false A.stopped_review_received(reviewed)','assert(button("stopped_choose_continue")==nil) A.choose_stopped("continue") assert(H.stopped_action=="")'),
 'note_required':('A.stopped_review_received(reviewed) A.choose_stopped("accept_completed")','button("stopped_confirm_review").on_click() assert(H.stopped_mode=="choose" && H.calls.len()==0)'),
 'continue_instruction_required':('A.stopped_review_received(reviewed) A.choose_stopped("continue") H.stopped_note="Reviewed effects"','button("stopped_confirm_review").on_click() assert(H.stopped_mode=="choose" && H.calls.len()==0)'),
 'confirm':('A.stopped_review_received(reviewed) A.choose_stopped("keep_blocked") H.stopped_note="Reviewed effects" A.confirm_stopped()','assert(H.stopped_mode=="confirm") button("stopped_commit").on_click() assert(H.calls.len()==1 && H.calls[0].args.reviewCommandId=="review" && H.calls[0].args.action=="keep_blocked" && H.calls[0].args.continuationInstruction==nil)'),
 'continue':('A.stopped_review_received(reviewed) A.choose_stopped("continue") H.stopped_note="Reviewed effects" H.stopped_instruction="Finish remaining work" A.confirm_stopped()','button("stopped_commit").on_click() assert(H.calls[0].args.continuationInstruction=="Finish remaining work")'),
 'cancel':('A.stopped_review_received(reviewed) A.choose_stopped("keep_blocked") H.stopped_note="Reviewed effects" A.confirm_stopped()','button("stopped_cancel").on_click() assert(H.stopped_mode=="choose" && H.calls.len()==0)'),
 'pending':('A.stopped_review_received(reviewed) A.choose_stopped("keep_blocked") H.stopped_note="Reviewed effects" A.confirm_stopped() H.command=command','assert(!button("stopped_commit").enabled) button("stopped_commit").on_click() assert(H.calls.len()==0)'),
 'result':('A.stopped_result_received(result)','assert(has(A.t("Decision recorded","处理决定已记录"))) assert(button("stopped_commit")==nil) button("stopped_refresh").on_click() assert(H.stopped_result==nil && H.calls[0].service=="hagency.stopped.get")'),
}
callbacks={
 'detail':'A.open_stopped("resource","engagement","dispatch",0) H.callback(row) assert(H.stopped_detail.revision==row.revision)',
 'wrong_dispatch':'A.open_stopped("resource","engagement","dispatch",0) row.attempt.dispatchId="other" H.callback(row) assert(H.stopped_detail==nil && H.notice!="")',
 'malformed':'A.open_stopped("resource","engagement","dispatch",0) H.callback({}) assert(H.stopped_detail==nil && H.notice!="")',
 'late_page':'A.open_stopped("resource","engagement","dispatch",0) H.page="resources" H.callback(row) assert(H.stopped_detail==nil)',
 'late_offset':'A.open_stopped("resource","engagement","dispatch",0) H.stopped_offset=8 H.callback(row) assert(H.stopped_detail==nil)',
 'inconsistent':'A.open_stopped("resource","engagement","dispatch",0) row.resolvable=false H.callback(row) assert(H.stopped_detail==nil)',
 'wrong_list':'A.open_stopped_tasks("resource","","") row.attempt.resourceId="foreign" H.callback(page) assert(H.stopped_list==nil && H.notice!="")',
 'stale_scope':'A.stopped_review_received(reviewed) A.choose_stopped("keep_blocked") H.stopped_note="Reviewed" A.confirm_stopped() H.stopped_dispatch="other" A.submit_stopped() assert(H.calls.len()==0)',
 'new_revision':'A.stopped_review_received(reviewed) H.stopped_detail=row.to_json().parse_json() H.stopped_detail.revision="changed" A.choose_stopped("accept_completed") assert(H.stopped_action=="")',
 'review_reply':'H.command=inspect_command A.retry_command() H.host_callback({is_ok:true data:reviewed}) assert(H.command==nil && H.stopped_mode=="choose" && H.ack=="review")',
 'wrong_review_reply':'H.command=inspect_command A.retry_command() reviewed.reviewCommandId="other" H.host_callback({is_ok:true data:reviewed}) assert(H.command!=nil && H.ack==nil)',
 'decision_reply':'H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.command==nil && H.stopped_mode=="result" && H.ack=="resolve")',
 'wrong_decision':'H.command=command A.retry_command() result.action="accept_completed" H.host_callback({is_ok:true data:result}) assert(H.command!=nil && H.ack==nil)',
 'lost_reply':'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command.args.commandId=="resolve" && H.ack==nil)',
 'lost_authority':'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_access_required"}) assert(H.command.args.commandId=="resolve" && H.ack==nil)',
 'expired':'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_stopped_review_expired"}) assert(H.command==nil && H.stopped_review==nil && H.ack=="resolve")',
 'conflict':'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_revision_conflict"}) assert(H.command==nil && H.stopped_detail==nil && H.ack=="resolve")',
 'save_failed':'A.save=fn(){return false} H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.stopped_mode=="result" && H.ack==nil)',
 'recover_review':'A.recover_native({operation:"review_stopped" commandId:"review" input:inspect_command.args}) H.callback({operation:"review_stopped" data:reviewed}) assert(H.stopped_mode=="choose" && H.ack=="review")',
 'recover_decision':'A.recover_native({operation:"resolve_stopped" commandId:"resolve" input:command.args}) H.callback({operation:"resolve_stopped" data:result}) assert(H.stopped_mode=="result" && H.ack=="resolve")',
 'wrong_recovery':'A.recover_native({operation:"resolve_stopped" commandId:"resolve" input:command.args}) result.dispatchId="other" H.callback({operation:"resolve_stopped" data:result}) assert(H.ack==nil && H.notice!="")',
}
records=[]
for lang in ['en','zh']:
 base=PREFIX+helpers+retry+recover+fixture+('A.t=fn(en,zh){return zh}\n' if lang=='zh' else '')
 cases={name:state+'\nlet root=View{'+pages+'}\ncollect(root)\n'+check for name,(state,check) in views.items()};cases.update(callbacks)
 for name,check in cases.items():
  script=a.output/(name+'-'+lang+'.splash');script.write_text(base+check+'\ntrue\n')
  r=subprocess.run([str(a.vm.resolve()),'eval',str(script)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);script.with_suffix('.log').write_text(r.stdout);records.append({'case':name+'-'+lang,'passed':r.returncode==0})
(a.output/'report.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps({'passed':sum(x['passed'] for x in records),'total':len(records),'failed':[x['case'] for x in records if not x['passed']]}));raise SystemExit(not all(x['passed'] for x in records))
