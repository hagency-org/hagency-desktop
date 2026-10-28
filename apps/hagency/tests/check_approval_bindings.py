#!/usr/bin/env python3
"""Evaluate actual binding pages and recovery handlers in the pinned script VM."""
import argparse
import json
import subprocess
from pathlib import Path
from check_agent_admission import APP, PREFIX

p = argparse.ArgumentParser()
p.add_argument('vm', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=True)
source = (APP / 'bundle/main.splash').read_text()
redraw = source[source.index('A.redraw = fn('):source.index('let on_app_locale_changed')]
helpers = source[source.index('A.binding_error = fn('):source.index('A.personal_reason = fn(')]
retry = source[source.index('A.retry_command = fn('):source.index('A.enroll = fn(')]
recover = source[source.index('A.recover_native = fn('):source.index('A.saved_label = fn(')]
pages = source[source.index('        if H.page=="approval_bindings" {'):source.index('        if H.page == "resource_detail" {mod.widgets.HagencyDetailCard{')]
fixture = '''
let Down=3 mod.widgets.RINX_SPACING=1 mod.rinx_icons.inbox=1 mod.rinx_icons.close=2 mod.rinx_icons.search=3
H.binding_filters={agent:"",project:""} H.binding_filter_draft={agent:"",project:""}
H.page="approval_bindings" H.binding_resource="resource" H.binding_after="" H.binding_page=nil H.binding_draft=nil H.binding_result=nil H.needs_signin=false H.ack=nil
A.is_task_operation=fn(operation){return false}
A.open_resource=fn(id){H.page="resource_detail" H.resource_id=id}
let row={resourceId:"resource" engagementId:"engagement" agent:"Writer" fleetId:"fleet" projectId:"project" serverName:"matrix.test" roomId:"!private:matrix.test" ownerMxid:"@owner:matrix.test" roomGeneration:3 incarnation:2 active:true revision:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
let page={resourceId:row.resourceId agent:"" project:"" bindings:[row] nextAfter:nil observedAtMs:1000}
let result={binding:row revokedGrants:2 unbound:true}
let command={service:"hagency.approval_bindings.retire" args:{commandId:"original" resourceId:row.resourceId engagementId:row.engagementId expectedRevision:row.revision}}
'''
cases = {
    'loading': ('H.busy=true', 'assert(has(A.t("Reading approval rooms…","正在读取审批房间…"))) assert(!button("bindings_refresh").enabled)'),
    'error_not_empty': ('', 'assert(has(A.t("Room list unavailable","房间列表暂不可用"))) assert(!has(A.t("No recorded approval-room links on this page","本页暂无已记录的审批房间绑定")))'),
    'empty': ('H.binding_page=page page.bindings=[]', 'assert(has(A.t("No recorded approval-room links on this page","本页暂无已记录的审批房间绑定")))'),
    'inspect': ('H.binding_page=page', 'assert(has(row.agent) && has(row.roomId) && has(row.ownerMxid)) button("binding_review").on_click() assert(H.page=="approval_binding_review" && H.binding_draft.expectedRevision==row.revision && H.calls.len()==0)'),
    'inactive': ('H.binding_page=page row.active=false', 'assert(has(A.t("Link needs verification","绑定需要验证"))) assert(button("binding_review").enabled)'),
    'pagination': ('H.binding_page=page page.nextAfter="cursor"', 'button("bindings_next").on_click() assert(H.calls[0].args.after=="cursor" && H.calls[0].args.resourceId=="resource")'),
    'filter_controls': ('H.binding_page=page', 'button("binding_agent_filter").on_change(" Writer ") button("binding_project_filter").on_change(" !project:matrix.test ") assert(H.calls.len()==0) button("bindings_filter").on_click() assert(H.calls.len()==1 && H.calls[0].args.agent=="Writer" && H.calls[0].args.project=="!project:matrix.test" && H.calls[0].args.after=="")'),
    'clear_filters': ('H.binding_filters={agent:"Writer",project:"project"} H.binding_filter_draft={agent:"Writer",project:"project"}', 'button("bindings_clear").on_click() assert(H.binding_filter_draft.agent=="" && H.binding_filter_draft.project=="" && H.calls[0].args.agent=="" && H.calls[0].args.project=="")'),
    'filtered_page': ('H.binding_page=page H.binding_filters={agent:"Writer",project:"project"} page.nextAfter="cursor"', 'button("bindings_next").on_click() assert(H.calls[0].args.after=="cursor" && H.calls[0].args.agent=="Writer" && H.calls[0].args.project=="project")'),
    'first_page': ('H.binding_page=page H.binding_after="cursor"', 'button("bindings_first").on_click() assert(H.calls[0].args.after=="")'),
    'back_resource': ('', 'button("bindings_back").on_click() assert(H.page=="resource_detail" && H.resource_id=="resource")'),
    'review': ('H.binding_page=page A.review_binding(row)', 'assert(has(row.engagementId) && has(row.projectId) && has(row.serverName)) button("binding_remove").on_click() assert(H.calls.len()==1 && H.calls[0].service==command.service && H.calls[0].args.expectedRevision==row.revision)'),
    'cancel': ('H.binding_page=page A.review_binding(row)', 'button("binding_cancel").on_click() assert(H.binding_draft==nil && H.page=="approval_bindings" && H.calls.len()==0)'),
    'pending_guard': ('H.binding_page=page A.review_binding(row) H.command=command', 'assert(!button("binding_remove").enabled) button("binding_remove").on_click() assert(H.calls.len()==0)'),
    'result': ('A.binding_received(result)', 'assert(has(A.t("Approval-room link removed","审批房间绑定已解除"))) assert(has(A.t("Saved tool approvals revoked: ","已撤销的工具授权：")+2)) button("binding_result_refresh").on_click() assert(H.calls[0].service=="hagency.approval_bindings.list")'),
}
callbacks = {
    'query_changes_reset_scroll': '''
let ui={heading:{render:fn(){}} navigation:{render:fn(){}} notice:{set_text:fn(text){} set_visible:fn(visible){}} content:{reset_scroll:fn(){H.reset_count=H.reset_count+1} body:{render:fn(){}}}}
H.reset_count=0 H.rendered_route=""
''' + redraw + '''
A.redraw() assert(H.reset_count==1)
H.binding_filter_draft.agent="unapplied" A.redraw() assert(H.reset_count==1)
H.binding_after="cursor" A.redraw() assert(H.reset_count==2)
A.redraw() assert(H.reset_count==2)
H.binding_filters.project="!project:matrix.test" A.redraw() assert(H.reset_count==3)
H.binding_resource="other" A.redraw() assert(H.reset_count==4)
''',
    'late_filter_response': 'A.open_bindings("resource","") H.binding_filters={agent:"Writer",project:"project"} H.callback(page) assert(H.binding_page==nil)',
    'wrong_filter_response': 'A.open_bindings("resource","") page.project="foreign" H.callback(page) assert(H.binding_page==nil && H.notice!="")',
    'resource_change_resets_filters': 'H.binding_filters={agent:"Writer",project:"project"} H.binding_filter_draft={agent:"Writer",project:"project"} A.open_bindings("other","") assert(H.calls[0].args.agent=="" && H.calls[0].args.project=="" && H.binding_filter_draft.agent=="")',
    'busy_filter_guard': 'H.busy=true A.filter_bindings() A.clear_binding_filters() assert(H.calls.len()==0)',
    'recovered_resource_clears_filters': 'H.binding_resource="other" H.binding_after="old_cursor" H.binding_filters={agent:"Old",project:"old_project"} H.binding_filter_draft={agent:"Old",project:"old_project"} A.binding_received(result) assert(H.binding_resource=="resource" && H.binding_after=="" && H.binding_filters.agent=="" && H.binding_filter_draft.project=="")',
    'late_response': 'A.open_bindings("resource","") H.page="overview" H.callback(page) assert(H.binding_page==nil)',
    'wrong_resource': 'A.open_bindings("resource","") page.resourceId="foreign" H.callback(page) assert(H.binding_page==nil && H.notice!="")',
    'foreign_row': 'A.open_bindings("resource","") row.resourceId="foreign" H.callback(page) assert(H.binding_page==nil && H.notice!="")',
    'valid': 'A.open_bindings("resource","") H.callback(page) assert(H.binding_page.bindings.len()==1)',
    'stale_review': 'H.binding_page=page A.review_binding(row) row.revision="changed" A.submit_binding() assert(H.calls.len()==0 && H.binding_draft==nil && H.notice!="")',
    'foreign_review': 'H.binding_page=page row.resourceId="foreign" A.review_binding(row) assert(H.binding_draft==nil)',
    'busy_guard': 'H.busy=true A.open_bindings("resource","") A.review_binding(row) A.submit_binding() assert(H.calls.len()==0)',
    'lost_reply_retained': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'authority_loss_retained': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_access_required"}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'signin_required': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"native_resume_missing"}) assert(H.needs_signin && H.command!=nil)',
    'stale_conflict': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_revision_conflict"}) assert(H.command==nil && H.binding_page==nil && H.ack=="original")',
    'success_exact': 'H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.command==nil && H.binding_result.revokedGrants==2 && H.ack=="original")',
    'wrong_reply_retained': 'H.command=command A.retry_command() result.binding={resourceId:"foreign" engagementId:"engagement" revision:row.revision} H.host_callback({is_ok:true data:result}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'save_failure_no_ack': 'A.save=fn(){return false} H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.ack==nil && H.binding_result.unbound)',
    'restart_recovery': 'A.recover_native({operation:"retire_approval_binding" commandId:"original" input:command.args}) assert(H.calls[0].service=="hagency.intent.retry" && H.calls[0].args.commandId=="original") H.callback({operation:"retire_approval_binding" data:result}) assert(H.page=="approval_binding_result" && H.ack=="original" && H.calls.len()==1)',
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
