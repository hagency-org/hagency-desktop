#!/usr/bin/env python3
"""Exercise shipped permission pages and original-command recovery in the VM.

Widgets are inert objects. This does not replace native window acceptance.
"""
import argparse
import json
import subprocess
from pathlib import Path
from check_agent_admission import APP, PREFIX

p = argparse.ArgumentParser()
p.add_argument('vm', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=False)
source = (APP / 'bundle/main.splash').read_text()
helpers = source[source.index('A.tool_error = fn('):source.index('A.personal_reason = fn(')]
retry = source[source.index('A.retry_command = fn('):source.index('A.enroll = fn(')]
recover = source[source.index('A.recover_native = fn('):source.index('A.saved_label = fn(')]
pages = source[source.index('        if H.page=="execution_policies" {'):source.index('        if H.page=="tool_approvals" || H.page=="tool_grants" {')]
fixture = '''
let Down=3 mod.widgets.RINX_SPACING=1 mod.rinx_icons.inbox=1
H.page="execution_policy" H.policy_resource="resource" H.policy_engagement="engagement" H.policy_after="" H.policy_list=nil H.policy_detail=nil H.policy_draft=nil H.policy_mode="detail" H.ack=nil H.needs_signin=false
A.is_task_operation=fn(operation){return false}
A.personal_config_operation=fn(operation){return false}
A.is_template_operation=fn(operation){return false}
A.is_graph_operation=fn(operation){return false}
A.open_resource=fn(id){H.page="resource_detail" H.resource_id=id}
let row={resourceId:"resource" engagementId:"engagement" agent:"Writer" projectId:"project" projectName:"Project Alpha" framework:"codex" executionPolicy:{yolo:false} appliesTo:"next_dispatch" revision:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
let page={resourceId:"resource" agents:[row] nextAfter:nil}
let result=row.to_json().parse_json()
result.executionPolicy.yolo=true result.revision="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
let command={service:"hagency.execution_policy.edit" args:{commandId:"original" resourceId:"resource" engagementId:"engagement" expectedRevision:row.revision executionPolicy:{yolo:true}}}
'''
views = {
    'shared_chat_to_standard': ('row.sharedMode="chat" H.policy_detail=row A.choose_policy(false)', 'assert(button("policy_review").enabled) A.review_policy() A.submit_policy() assert(H.calls.len()==1 && !H.calls[0].args.executionPolicy.yolo)'),
    'shared_pending_requests': ('row.sharedMode="standard" H.policy_detail=row', 'button("policy_grants").on_click() assert(H.page=="shared_approvals" && H.calls[0].service=="hagency.shared_approvals.list")'),
    'saved_permissions': ('H.policy_detail=row', 'button("policy_grants").on_click() assert(H.page=="tool_grants" && H.calls.len()==1 && H.calls[0].service=="hagency.tool_grants.list" && H.calls[0].args.resourceId=="resource" && H.calls[0].args.engagementId=="engagement" && H.calls[0].args.after=="")'),
    'saved_permissions_pending': ('H.policy_detail=row H.command=command', 'assert(!button("policy_grants").enabled) button("policy_grants").on_click() assert(H.calls.len()==0 && H.command.args.commandId=="original")'),
    'saved_permissions_draft': ('H.policy_detail=row A.choose_policy(true)', 'assert(button("policy_grants")==nil) A.open_policy_grants() assert(H.calls.len()==0 && H.policy_mode=="edit" && H.policy_draft.executionPolicy.yolo)'),
    'saved_permissions_result': ('A.policy_received(result)', 'button("policy_grants").on_click() assert(H.page=="tool_grants" && H.calls[0].args.engagementId=="engagement")'),
    'loading': ('H.busy=true', 'assert(has(A.t("Reading current permissions…","正在读取当前权限…"))) assert(!button("policy_refresh").enabled)'),
    'unavailable': ('', 'assert(has(A.t("Refresh to read current permissions.","请刷新以读取当前权限。"))) assert(button("policy_save")==nil)'),
    'list_empty': ('H.page="execution_policies" H.policy_list=page page.agents=[]', 'assert(has(A.t("No agents on this page","本页暂无 Agent")))'),
    'list_unavailable': ('H.page="execution_policies"', 'assert(has(A.t("Agent list unavailable","Agent 列表暂不可用"))) assert(!has(A.t("No agents on this page","本页暂无 Agent")))'),
    'list_agent': ('H.page="execution_policies" H.policy_list=page', 'assert(has("Writer") && has("Project Alpha")) button("policy_list_open").on_click() assert(H.calls.len()==1 && H.calls[0].service=="hagency.execution_policy.get" && H.calls[0].args.engagementId=="engagement")'),
    'list_next': ('H.page="execution_policies" H.policy_list=page page.nextAfter="cursor"', 'button("policy_list_next").on_click() assert(H.calls[0].args.after=="cursor" && H.calls[0].args.resourceId=="resource")'),
    'list_first': ('H.page="execution_policies" H.policy_list=page H.policy_after="cursor"', 'button("policy_list_first").on_click() assert(H.calls[0].args.after=="")'),
    'detail': ('H.policy_detail=row', 'assert(has("Writer") && has(A.t("Project: ","项目：")+"Project Alpha")) assert(button("policy_save")==nil) button("policy_unrestricted").on_click() assert(H.policy_mode=="edit" && H.policy_draft.executionPolicy.yolo && H.calls.len()==0)'),
    'unsupported': ('H.policy_detail=row row.framework="claude"', 'assert(button("policy_unrestricted")==nil) A.choose_policy(true) assert(H.policy_draft==nil)'),
    'unchanged': ('H.policy_detail=row A.choose_policy(false)', 'assert(!button("policy_review").enabled) button("policy_review").on_click() assert(H.policy_mode=="edit" && H.calls.len()==0)'),
    'cancel': ('H.policy_detail=row A.choose_policy(true)', 'button("policy_cancel").on_click() assert(H.policy_draft==nil && H.policy_mode=="detail" && H.calls.len()==0)'),
    'review': ('H.policy_detail=row A.choose_policy(true) A.review_policy()', 'assert(has(A.t("Review permissions","核对权限"))) assert(has(A.t("Standard permissions","标准权限")) && has(A.t("YOLO — unrestricted","YOLO — 不受限"))) button("policy_save").on_click() assert(H.calls.len()==1 && H.calls[0].service==command.service && H.calls[0].args.expectedRevision==row.revision && H.calls[0].args.executionPolicy.yolo)'),
    'review_back': ('H.policy_detail=row A.choose_policy(true) A.review_policy()', 'button("policy_review_back").on_click() assert(H.policy_mode=="edit" && H.policy_draft.executionPolicy.yolo && H.calls.len()==0)'),
    'pending': ('H.policy_detail=row A.choose_policy(true) A.review_policy() H.command=command', 'assert(!button("policy_save").enabled) button("policy_save").on_click() assert(H.calls.len()==0)'),
    'result': ('A.policy_received(result)', 'assert(has(A.t("Permissions saved","权限已保存"))) assert(button("policy_save")==nil) button("policy_refresh").on_click() assert(H.calls[0].service=="hagency.execution_policy.get" && H.policy_detail==nil && H.policy_mode=="detail")'),
}
callbacks = {
    'valid_detail': 'A.open_policy("resource","engagement") H.callback(row) assert(H.policy_detail.agent=="Writer")',
    'wrong_resource': 'A.open_policy("resource","engagement") row.resourceId="other" H.callback(row) assert(H.policy_detail==nil && H.notice!="")',
    'wrong_agent': 'A.open_policy("resource","engagement") row.engagementId="other" H.callback(row) assert(H.policy_detail==nil && H.notice!="")',
    'malformed': 'A.open_policy("resource","engagement") H.callback({resourceId:"resource"}) assert(H.policy_detail==nil && H.notice!="")',
    'nonboolean': 'A.open_policy("resource","engagement") row.executionPolicy.yolo="true" H.callback(row) assert(H.policy_detail==nil && H.notice!="")',
    'wrong_application': 'A.open_policy("resource","engagement") row.appliesTo="current_dispatch" H.callback(row) assert(H.policy_detail==nil && H.notice!="")',
    'late_page': 'A.open_policy("resource","engagement") H.page="overview" H.callback(row) assert(H.policy_detail==nil)',
    'late_agent': 'A.open_policy("resource","engagement") H.policy_engagement="other" H.callback(row) assert(H.policy_detail==nil)',
    'list_scope': 'A.open_policies("resource","") row.resourceId="other" H.callback(page) assert(H.policy_list==nil && H.notice!="")',
    'list_malformed': 'A.open_policies("resource","") page.agents=[nil] H.callback(page) assert(H.policy_list==nil && H.notice!="")',
    'list_late_page': 'A.open_policies("resource","") H.policy_after="other" H.callback(page) assert(H.policy_list==nil)',
    'stale_review': 'H.policy_detail=row A.choose_policy(true) A.review_policy() row.revision="changed" A.submit_policy() assert(H.calls.len()==0)',
    'scope_changed': 'H.policy_detail=row A.choose_policy(true) A.review_policy() H.policy_resource="other" A.submit_policy() assert(H.calls.len()==0)',
    'lost_reply': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'authority_loss': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_access_required"}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'conflict': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_revision_conflict"}) assert(H.command==nil && H.policy_detail==nil && H.ack=="original")',
    'success': 'H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.command==nil && H.policy_mode=="result" && H.policy_detail.executionPolicy.yolo && H.ack=="original")',
    'wrong_success': 'H.command=command A.retry_command() result.executionPolicy.yolo=false H.host_callback({is_ok:true data:result}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'save_failed': 'A.save=fn(){return false} H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.policy_mode=="result" && H.ack==nil)',
    'restart': 'A.recover_native({operation:"edit_execution_policy" commandId:"original" input:command.args}) H.callback({operation:"edit_execution_policy" data:result}) assert(H.policy_mode=="result" && H.ack=="original" && H.calls.len()==1)',
    'wrong_restart': 'H.command=command A.recover_native({operation:"edit_execution_policy" commandId:"original" input:command.args}) result.engagementId="other" H.callback({operation:"edit_execution_policy" data:result}) assert(H.command.args.commandId=="original" && H.ack==nil)',
}
records = []
for lang in ['en', 'zh']:
    base = PREFIX + helpers + retry + recover + fixture + ('A.t=fn(en,zh){return zh}\n' if lang == 'zh' else '')
    cases = {name: state + '\nlet root=View{' + pages + '}\ncollect(root)\n' + check for name, (state, check) in views.items()}
    cases.update(callbacks)
    for name, check in cases.items():
        script = a.output / (name + '-' + lang + '.splash')
        script.write_text(base + check + '\ntrue\n')
        r = subprocess.run([str(a.vm.resolve()), 'eval', str(script)], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        script.with_suffix('.log').write_text(r.stdout)
        records.append({'case': name + '-' + lang, 'passed': r.returncode == 0})
(a.output / 'report.json').write_text(json.dumps(records, indent=2) + '\n')
print(json.dumps({'passed': sum(x['passed'] for x in records), 'total': len(records), 'failed': [x['case'] for x in records if not x['passed']]}))
raise SystemExit(not all(x['passed'] for x in records))
