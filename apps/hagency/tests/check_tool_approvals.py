#!/usr/bin/env python3
"""Evaluate shipped EN/CN tool-management views and exact retry callbacks.

Uses the pinned script VM with inert widgets; native windows are a separate gate.
"""
import argparse
import json
import subprocess
from pathlib import Path
from check_agent_admission import APP, PREFIX

p=argparse.ArgumentParser()
p.add_argument('vm',type=Path)
p.add_argument('--output',type=Path,required=True)
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
source=(APP/'bundle/main.splash').read_text()
helpers=source[source.index('A.tool_error = fn('):source.index('A.personal_reason = fn(')]
retry=source[source.index('A.retry_command = fn('):source.index('A.enroll = fn(')]
recover=source[source.index('A.recover_native = fn('):source.index('A.saved_label = fn(')]
pages=source[source.index('        if H.page=="tool_approvals" || H.page=="tool_grants" {'):source.index('        if H.page=="approval_bindings" {mod.widgets.HagencyDetailCard{')]
redraw=source[source.index('A.redraw = fn('):source.index('let on_app_locale_changed')]
fixture='''
let Down=3 let Ellipsis=4 mod.widgets.RINX_SPACING=1 mod.rinx_icons.inbox=1 mod.rinx_icons.close=2
H.page="tool_grants" H.tool_resource="resource" H.tool_engagement="engagement" H.tool_after="" H.tool_kind="rules" H.tool_page=nil H.tool_draft=nil H.tool_result=nil H.needs_signin=false H.ack=nil H.tool_selected=nil H.tool_request_id=""
A.is_task_operation=fn(operation){return false}
A.open_resource=fn(id){H.page="resource_detail" H.resource_id=id}
let row={id:"grant" resourceId:"resource" engagementId:"engagement" agent:"Writer" kind:"exact_command" mode:"always" taskId:nil revoked:false bindingCurrent:true details:{description:"echo approved" projectId:"project" ownerMxid:"@owner:matrix.test"} revision:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
let page={resourceId:"resource" engagementId:"engagement" grants:[row] nextAfter:nil observedAtMs:1000}
let request={id:"approval" engagementId:"engagement" projectRoomId:"!project:matrix.test" state:"pending" choice:nil reusableScope:true expiresAt:12000}
let subject={engagementId:"engagement" agent:"Writer" projectId:"project" projectName:"Project Alpha"}
let requests={resourceId:"resource" engagementId:"" approvals:[request] subjects:[subject] nextAfter:nil observedAtMs:1000}
let snapshot={resourceId:"resource" approval:request subject:subject observedAtMs:1000}
let result={resourceId:row.resourceId engagementId:row.engagementId grantId:row.id reviewedRevision:row.revision revoked:true}
let command={service:"hagency.tool_grants.revoke" args:{commandId:"original" resourceId:row.resourceId engagementId:row.engagementId grantId:row.id expectedRevision:row.revision}}
'''
cases={
 'loading':('H.busy=true','assert(has(A.t("Reading tool permissions…","正在读取工具权限…"))) assert(!button("tools_refresh").enabled)'),
 'error_not_empty':('','assert(has(A.t("Tool records unavailable","工具记录暂不可用"))) assert(!has(A.t("No saved tool permissions on this page","本页暂无已保存的工具权限")))'),
 'empty_rules':('H.tool_page=page page.grants=[]','assert(has(A.t("No saved tool permissions on this page","本页暂无已保存的工具权限")))'),
 'requests':('H.page="tool_approvals" H.tool_kind="requests" H.tool_engagement="" H.tool_page=requests','assert(has(A.t("Awaiting tool approval","等待工具审批"))) assert(!has(A.t("Awaiting administrator","等待管理员审批"))) assert(has("Writer") && has(A.t("Project: ","项目：")+"Project Alpha")) button("tool_request_open").on_click() assert(H.calls[0].service=="hagency.tool_approvals.get" && H.calls[0].args.resourceId=="resource" && H.calls[0].args.approvalId==request.id)'),
 'request_subject_fallback':('H.page="tool_approvals" H.tool_kind="requests" H.tool_engagement="" H.tool_page=requests requests.subjects=[]','assert(has(A.t("Agent information unavailable","Agent 信息暂不可用"))) assert(has(A.t("Project: ","项目：")+A.t("Project information unavailable","项目信息暂不可用")))'),
 'request_project_id':('H.page="tool_approvals" H.tool_kind="requests" H.tool_engagement="" H.tool_page=requests subject.projectName=nil','assert(has(A.t("Project: ","项目：")+"project"))'),
 'detail':('H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot','assert(has("Writer") && has("Project Alpha")) assert(has(A.t("Awaiting tool approval","等待工具审批"))) assert(has(A.t("Time left at refresh: ","刷新时剩余时间：")+"11"+A.t(" seconds"," 秒"))) button("tool_request_inbox").on_click() assert(H.calls.len()==1 && H.calls[0].service=="hagency.tool_approvals.open") assert(H.calls[0].args.resourceId=="resource" && H.calls[0].args.approvalId=="approval") assert(H.calls[0].args.to_json()=={resourceId:"resource" approvalId:"approval"}.to_json())'),
 'detail_expired':('H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot snapshot.observedAtMs=request.expiresAt','assert(has(A.t("Approval window ended","审批已截止"))) assert(has(A.t("Approval deadline passed","审批期限已过"))) assert(button("tool_request_inbox").enabled)'),
 'detail_decided':('H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot request.state="decided" request.choice="deny" snapshot.observedAtMs=request.expiresAt+1','assert(has(A.t("Decision recorded","决定已记录"))) assert(has(A.t("Denied","已拒绝"))) assert(!has(A.t("Approval window ended","审批已截止")))'),
 'detail_missing_room':('H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot request.projectRoomId=nil','assert(!button("tool_request_inbox").enabled) button("tool_request_inbox").on_click() assert(H.calls.len()==0 && H.notice!="")'),
 'detail_loading':('H.page="tool_request" H.busy=true','assert(has(A.t("Reading current request…","正在读取请求最新状态…"))) assert(!button("tool_request_refresh").enabled) assert(button("tool_request_inbox")==nil)'),
 'detail_error':('H.page="tool_request"','assert(has(A.t("Request information unavailable","请求信息暂不可用"))) assert(button("tool_request_inbox")==nil)'),
 'detail_back':('H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot H.tool_engagement="" H.tool_after="page_cursor"','button("tool_request_back").on_click() assert(H.calls[0].service=="hagency.tool_approvals.list" && H.calls[0].args.after=="page_cursor" && H.calls[0].args.engagementId=="") assert(H.tool_selected==nil && H.tool_request_id=="")'),
 'detail_rules':('H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot','button("tools_agent_rules").on_click() assert(H.calls[0].service=="hagency.tool_grants.list" && H.calls[0].args.engagementId==request.engagementId && H.calls[0].args.after=="")'),
 'detail_refresh':('H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot','button("tool_request_refresh").on_click() assert(H.tool_selected==nil && H.calls[0].service=="hagency.tool_approvals.get" && H.calls[0].args.approvalId==request.id)'),
 'pending_deadline_reached':('H.page="tool_approvals" H.tool_kind="requests" H.tool_engagement="" H.tool_page=requests requests.observedAtMs=request.expiresAt','assert(has(A.t("Approval window ended","审批已截止"))) assert(!has(A.t("Awaiting tool approval","等待工具审批"))) assert(request.state=="pending")'),
 'pending_deadline_passed':('H.page="tool_approvals" H.tool_kind="requests" H.tool_engagement="" H.tool_page=requests requests.observedAtMs=request.expiresAt+1','assert(has(A.t("Approval window ended","审批已截止"))) assert(!has(A.t("Awaiting tool approval","等待工具审批"))) assert(request.state=="pending")'),
 'recorded_decision_past_deadline':('H.page="tool_approvals" H.tool_kind="requests" H.tool_engagement="" H.tool_page=requests request.state="decided" request.choice="always" requests.observedAtMs=request.expiresAt+1','assert(has(A.t("Decision recorded","决定已记录"))) assert(!has(A.t("Approval window ended","审批已截止"))) assert(request.state=="decided")'),
 'delivered_decision_past_deadline':('H.page="tool_approvals" H.tool_kind="requests" H.tool_engagement="" H.tool_page=requests request.state="applied" request.choice="always" requests.observedAtMs=request.expiresAt+1','assert(has(A.t("Decision delivered","决定已传递"))) assert(!has(A.t("Approval window ended","审批已截止"))) assert(request.state=="applied")'),
 'empty_requests':('H.page="tool_approvals" H.tool_kind="requests" H.tool_engagement="" H.tool_page=requests requests.approvals=[]','assert(has(A.t("No tool requests on this page","本页暂无工具请求")))'),
 'rule':('H.tool_page=page','assert(has("echo approved")) assert(has("@owner:matrix.test · project")) button("tool_grant_review").on_click() assert(H.page=="tool_grant_review" && H.tool_draft.revision==row.revision && H.calls.len()==0)'),
 'task':('H.tool_page=page row.mode="task" row.taskId="task_42"','assert(has(A.t("Allow for this task","此任务内允许"))) assert(has(A.t("Task: ","任务：")+"task_42"))'),
 'revoked':('H.tool_page=page row.revoked=true','assert(has(A.t("Revoked","已撤销"))) assert(button("tool_grant_review")==nil)'),
 'inactive':('H.tool_page=page row.bindingCurrent=false','assert(has(A.t("Approval-room link needs verification","审批房间绑定需要验证"))) assert(button("tool_grant_review").enabled)'),
 'missing_description':('H.tool_page=page row.details=nil','assert(has(A.t("Original rule description unavailable","原始权限说明不可用"))) assert(button("tool_grant_review").enabled)'),
 'next_page':('H.tool_page=page page.nextAfter="cursor"','button("tools_next").on_click() assert(H.calls[0].args.after=="cursor" && H.calls[0].args.engagementId=="engagement")'),
 'first_page':('H.tool_page=page H.tool_after="cursor"','button("tools_first").on_click() assert(H.calls[0].args.after=="")'),
 'review':('H.tool_page=page A.review_tool_grant(row)','assert(has("echo approved") && has("@owner:matrix.test") && has("project")) button("tool_grant_revoke").on_click() assert(H.calls[0].service==command.service && H.calls[0].args.grantId==row.id && H.calls[0].args.expectedRevision==row.revision)'),
 'cancel':('H.tool_page=page A.review_tool_grant(row)','button("tool_grant_cancel").on_click() assert(H.page=="tool_grants" && H.tool_draft==nil && H.calls.len()==0)'),
 'pending_guard':('H.tool_page=page A.review_tool_grant(row) H.command=command','assert(!button("tool_grant_revoke").enabled) button("tool_grant_revoke").on_click() assert(H.calls.len()==0)'),
 'result':('A.tool_grant_received(result)','assert(has(A.t("Tool permission revoked","工具权限已撤销"))) button("tool_result_refresh").on_click() assert(H.calls[0].service=="hagency.tool_grants.list" && H.calls[0].args.after=="")'),
}
callbacks={
 'detail_valid':'A.open_tool_request("approval") H.callback(snapshot) assert(H.tool_selected.subject.agent=="Writer" && H.tool_selected.approval.state=="pending")',
 'detail_wrong_resource':'A.open_tool_request("approval") snapshot.resourceId="foreign" H.callback(snapshot) assert(H.tool_selected==nil && H.notice!="")',
 'detail_wrong_id':'A.open_tool_request("approval") request.id="foreign" H.callback(snapshot) assert(H.tool_selected==nil && H.notice!="")',
 'detail_wrong_subject':'A.open_tool_request("approval") subject.engagementId="foreign" H.callback(snapshot) assert(H.tool_selected==nil && H.notice!="")',
 'detail_malformed':'A.open_tool_request("approval") H.callback({resourceId:"resource"}) assert(H.tool_selected==nil && H.notice!="")',
 'detail_late_page':'A.open_tool_request("approval") H.page="overview" H.callback(snapshot) assert(H.tool_selected==nil)',
 'detail_late_resource':'A.open_tool_request("approval") H.tool_resource="foreign" H.callback(snapshot) assert(H.tool_selected==nil)',
 'detail_late_id':'A.open_tool_request("approval") H.tool_request_id="other" H.callback(snapshot) assert(H.tool_selected==nil)',
 'detail_latest_state':'A.open_tool_request("approval") H.callback(snapshot) A.open_tool_request("approval") assert(H.tool_selected==nil) request.state="decided" request.choice="deny" H.callback(snapshot) assert(H.tool_selected.approval.state=="decided" && H.tool_selected.approval.choice=="deny" && H.calls.len()==2)',
 'detail_busy_guard':'H.busy=true A.open_tool_request("approval") A.open_tool_inbox() assert(H.calls.len()==0)',
 'inbox_scope_guard':'H.page="tool_request" H.tool_request_id="other" H.tool_selected=snapshot A.open_tool_inbox() assert(H.calls.len()==0)',
 'inbox_late_response':'H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot A.open_tool_inbox() H.page="overview" H.callback({requested:true}) assert(H.notice=="")',
 'inbox_requested':'H.page="tool_request" H.tool_request_id=request.id H.tool_selected=snapshot A.open_tool_inbox() H.callback({requested:true}) assert(H.notice==A.t("Opening the verified approval inbox…","正在打开已验证的审批收件箱…")) assert(request.state=="pending" && request.choice==nil && H.command==nil)',
 'valid': 'A.open_tools("resource","engagement","","rules") H.callback(page) assert(H.tool_page.grants.len()==1)',
 'wrong_scope':'A.open_tools("resource","engagement","","rules") page.resourceId="foreign" H.callback(page) assert(H.tool_page==nil && H.notice!="")',
 'foreign_row':'A.open_tools("resource","engagement","","rules") row.engagementId="foreign" H.callback(page) assert(H.tool_page==nil && H.notice!="")',
 'foreign_request':'A.open_tools("resource","engagement","","requests") requests.engagementId="engagement" request.engagementId="foreign" H.callback(requests) assert(H.tool_page==nil && H.notice!="")',
 'late_response':'A.open_tools("resource","engagement","","rules") H.page="overview" H.callback(page) assert(H.tool_page==nil)',
 'late_agent_response':'A.open_tools("resource","engagement","","rules") H.tool_engagement="other" H.callback(page) assert(H.tool_page==nil)',
 'stale_review':'H.tool_page=page A.review_tool_grant(row) row.revision="changed" A.submit_tool_grant() assert(H.calls.len()==0 && H.tool_draft==nil && H.notice!="")',
 'foreign_review':'H.tool_page=page row.resourceId="foreign" A.review_tool_grant(row) assert(H.tool_draft==nil)',
 'revoked_review':'H.tool_page=page row.revoked=true A.review_tool_grant(row) assert(H.tool_draft==nil)',
 'busy_guard':'H.busy=true A.open_tools("resource","engagement","","rules") A.review_tool_grant(row) A.submit_tool_grant() assert(H.calls.len()==0)',
 'unscoped_rules':'A.open_tools("resource","","","rules") assert(H.calls.len()==0)',
 'lost_reply':'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command.args.commandId=="original" && H.ack==nil)',
 'authority_loss':'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_access_required"}) assert(H.command.args.commandId=="original" && H.ack==nil)',
 'signin_required':'H.command=command A.retry_command() H.host_callback({is_ok:false error:"native_resume_missing"}) assert(H.needs_signin && H.command!=nil)',
 'conflict':'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_revision_conflict"}) assert(H.command==nil && H.tool_page==nil && H.ack=="original")',
 'exact_success':'H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.command==nil && H.tool_result.revoked && H.ack=="original")',
 'wrong_reply':'H.command=command A.retry_command() result.grantId="foreign" H.host_callback({is_ok:true data:result}) assert(H.command.args.commandId=="original" && H.ack==nil)',
 'malformed_reply':'H.command=command A.retry_command() H.host_callback({is_ok:true data:{revoked:true}}) assert(H.command.args.commandId=="original" && H.ack==nil)',
 'save_failure':'A.save=fn(){return false} H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.ack==nil && H.tool_result.revoked)',
 'restart_recovery':'A.recover_native({operation:"revoke_tool_grant" commandId:"original" input:command.args}) H.callback({operation:"revoke_tool_grant" data:result}) assert(H.page=="tool_grant_result" && H.ack=="original" && H.calls.len()==1)',
 'wrong_recovery':'H.command=command A.recover_native({operation:"revoke_tool_grant" commandId:"original" input:command.args}) result.grantId="foreign" H.callback({operation:"revoke_tool_grant" data:result}) assert(H.ack==nil && H.command.args.commandId=="original" && H.notice!="")',
 'page_resets_scroll':redraw+'''
let ui={heading:{render:fn(){}} navigation:{render:fn(){}} notice:{set_text:fn(text){} set_visible:fn(visible){}} content:{reset_scroll:fn(){H.reset_count=H.reset_count+1} body:{render:fn(){}}}}
H.rendered_route="" H.reset_count=0
A.redraw() assert(H.reset_count==1)
H.tool_after="next" A.redraw() assert(H.reset_count==2)
H.tool_engagement="other" A.redraw() assert(H.reset_count==3)
H.tool_resource="different" A.redraw() assert(H.reset_count==4)
H.page="tool_request" H.tool_request_id="one" A.redraw() assert(H.reset_count==5)
H.tool_request_id="two" A.redraw() assert(H.reset_count==6)
A.redraw() assert(H.reset_count==6)
''',
}
records=[]
for lang in ['en','zh']:
 base=PREFIX+helpers+retry+recover+fixture+('A.t=fn(en,zh){return zh}\n' if lang=='zh' else '')
 for name,(state,check) in cases.items():
  code=base+state+'\nlet root=View{'+pages+'}\ncollect(root)\n'+check
  script=a.output/(name+'-'+lang+'.splash');script.write_text(code+'\ntrue\n')
  r=subprocess.run([str(a.vm.resolve()),'eval',str(script)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  script.with_suffix('.log').write_text(r.stdout);records.append({'case':name+'-'+lang,'passed':r.returncode==0})
 for name,check in callbacks.items():
  script=a.output/(name+'-'+lang+'.splash');script.write_text(base+check+'\ntrue\n')
  r=subprocess.run([str(a.vm.resolve()),'eval',str(script)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  script.with_suffix('.log').write_text(r.stdout);records.append({'case':name+'-'+lang,'passed':r.returncode==0})
(a.output/'report.json').write_text(json.dumps(records,indent=2)+'\n')
print(json.dumps({'passed':sum(x['passed'] for x in records),'total':len(records),'failed':[x['case'] for x in records if not x['passed']]}))
raise SystemExit(not all(x['passed'] for x in records))
