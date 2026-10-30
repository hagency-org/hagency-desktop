#!/usr/bin/env python3
"""Exercise shipped workspace screens, review and exact-command recovery.

Uses the pinned VM with inert widgets; native window acceptance is separate.
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
helpers = source[source.index('A.workspace_valid=fn('):source.index('A.policy_label = fn(')]
retry = source[source.index('A.retry_command = fn('):source.index('A.enroll = fn(')]
recover = source[source.index('A.recover_native = fn('):source.index('A.saved_label = fn(')]
pages = source[source.index('        if H.page=="workspaces" {'):source.index('        if H.page=="execution_policies" {')]
fixture = '''
let Down=3 mod.widgets.RINX_SPACING=1 mod.rinx_icons.inbox=1
H.page="workspace" H.workspace_resource="resource" H.workspace_id="workspace" H.workspace_after="" H.workspace_list=nil H.workspace_detail=nil H.workspace_review=nil H.workspace_mode="detail" H.ack=nil H.needs_signin=false
A.is_task_operation=fn(operation){return false}
A.open_resource=fn(id){H.page="resource_detail" H.resource_id=id}
let row={resourceId:"resource" workspaceId:"workspace" dirty:true activeLeases:0 quarantined:false sharedResourceCount:1 releaseBlock:nil lastReleasedAt:nil revision:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
let page={resourceId:"resource" workspaces:[row] nextAfter:nil}
let result={workspace:row.to_json().parse_json() cleared:true}
result.workspace.dirty=false result.workspace.lastReleasedAt=12345 result.workspace.revision="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
let command={service:"hagency.workspaces.clear_dirty" args:{commandId:"original" resourceId:"resource" workspaceId:"workspace" expectedRevision:row.revision}}
'''
views = {
    'loading': ('H.busy=true', 'assert(has(A.t("Reading workspace status…","正在读取工作区状态…"))) assert(!button("workspace_refresh").enabled)'),
    'unavailable': ('', 'assert(has(A.t("Refresh to read current workspace status.","请刷新以读取工作区当前状态。"))) assert(button("workspace_review")==nil)'),
    'list_empty': ('H.page="workspaces" H.workspace_list=page page.workspaces=[]', 'assert(has(A.t("No recorded workspaces on this page","本页暂无工作区记录")))'),
    'list_unavailable': ('H.page="workspaces"', 'assert(has(A.t("Workspace list unavailable","工作区列表暂不可用"))) assert(!has(A.t("No recorded workspaces on this page","本页暂无工作区记录")))'),
    'list_open': ('H.page="workspaces" H.workspace_list=page', 'button("workspace_list_open").on_click() assert(H.calls.len()==1 && H.calls[0].service=="hagency.workspaces.get" && H.calls[0].args.workspaceId=="workspace")'),
    'list_next': ('H.page="workspaces" H.workspace_list=page page.nextAfter="cursor"', 'button("workspace_list_next").on_click() assert(H.calls[0].args.after=="cursor")'),
    'list_first': ('H.page="workspaces" H.workspace_list=page H.workspace_after="cursor"', 'button("workspace_list_first").on_click() assert(H.calls[0].args.after=="")'),
    'detail': ('H.workspace_detail=row', 'assert(has(A.t("Workspace needs review","工作区需要你检查"))) assert(button("workspace_confirm")==nil) button("workspace_review").on_click() assert(H.workspace_mode=="review" && H.calls.len()==0)'),
    'clean': ('H.workspace_detail=row row.dirty=false', 'assert(has(A.t("No recovery action is needed.","无需恢复操作。"))) assert(button("workspace_review")==nil) A.review_workspace() assert(H.workspace_mode=="detail")'),
    'quarantine': ('H.workspace_detail=row row.quarantined=true row.releaseBlock="inspection_required"', 'assert(button("workspace_review")==nil) assert(has(A.workspace_error("management_workspace_inspection_required"))) A.review_workspace() assert(H.workspace_review==nil)'),
    'active': ('H.workspace_detail=row row.activeLeases=1 row.releaseBlock="active_execution"', 'assert(button("workspace_review")==nil) assert(has(A.workspace_error("management_workspace_active"))) A.review_workspace() assert(H.workspace_review==nil)'),
    'quarantine_with_lease': ('H.workspace_detail=row row.activeLeases=1 row.quarantined=true row.releaseBlock="inspection_required"', 'assert(button("workspace_review")==nil) assert(has(A.workspace_error("management_workspace_inspection_required"))) assert(!has(A.workspace_error("management_workspace_active"))) assert(button("workspace_stopped")!=nil)'),
    'shared': ('H.workspace_detail=row row.sharedResourceCount=2 row.releaseBlock="shared_resource_authority"', 'assert(button("workspace_review")==nil) A.review_workspace() assert(H.workspace_review==nil)'),
    'review': ('H.workspace_detail=row A.review_workspace()', 'assert(has(A.t("Review workspace release","核对工作区释放"))) button("workspace_confirm").on_click() assert(H.calls.len()==1 && H.calls[0].service==command.service && H.calls[0].args.expectedRevision==row.revision)'),
    'cancel': ('H.workspace_detail=row A.review_workspace()', 'button("workspace_cancel").on_click() assert(H.workspace_mode=="detail" && H.workspace_review==nil && H.calls.len()==0)'),
    'pending': ('H.workspace_detail=row A.review_workspace() H.command=command', 'assert(!button("workspace_confirm").enabled) button("workspace_confirm").on_click() assert(H.calls.len()==0)'),
    'result': ('A.workspace_received(result)', 'assert(has(A.t("Workspace marker cleared","已解除工作区标记"))) assert(button("workspace_review")==nil) button("workspace_refresh").on_click() assert(H.workspace_detail==nil && H.workspace_mode=="detail" && H.calls[0].service=="hagency.workspaces.get")'),
}
callbacks = {
    'valid_detail': 'A.open_workspace("resource","workspace") H.callback(row) assert(H.workspace_detail.dirty)',
    'wrong_resource': 'A.open_workspace("resource","workspace") row.resourceId="other" H.callback(row) assert(H.workspace_detail==nil && H.notice!="")',
    'wrong_workspace': 'A.open_workspace("resource","workspace") row.workspaceId="other" H.callback(row) assert(H.workspace_detail==nil && H.notice!="")',
    'malformed': 'A.open_workspace("resource","workspace") H.callback({resourceId:"resource"}) assert(H.workspace_detail==nil && H.notice!="")',
    'negative_lease': 'A.open_workspace("resource","workspace") row.activeLeases=-1 H.callback(row) assert(H.workspace_detail==nil && H.notice!="")',
    'string_dirty': 'A.open_workspace("resource","workspace") row.dirty="false" H.callback(row) assert(H.workspace_detail==nil && H.notice!="")',
    'inconsistent_gate': 'A.open_workspace("resource","workspace") row.quarantined=true H.callback(row) assert(H.workspace_detail==nil && H.notice!="")',
    'late_page': 'A.open_workspace("resource","workspace") H.page="overview" H.callback(row) assert(H.workspace_detail==nil)',
    'late_workspace': 'A.open_workspace("resource","workspace") H.workspace_id="other" H.callback(row) assert(H.workspace_detail==nil)',
    'list_scope': 'A.open_workspaces("resource","") row.resourceId="other" H.callback(page) assert(H.workspace_list==nil && H.notice!="")',
    'list_malformed': 'A.open_workspaces("resource","") page.workspaces=[nil] H.callback(page) assert(H.workspace_list==nil && H.notice!="")',
    'stale_review': 'H.workspace_detail=row A.review_workspace() row.revision="changed" A.submit_workspace() assert(H.calls.len()==0)',
    'scope_changed': 'H.workspace_detail=row A.review_workspace() H.workspace_resource="other" A.submit_workspace() assert(H.calls.len()==0)',
    'became_active': 'H.workspace_detail=row A.review_workspace() row.activeLeases=1 A.submit_workspace() assert(H.calls.len()==0)',
    'lost_reply': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'authority_loss': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_access_required"}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'conflict': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_revision_conflict"}) assert(H.command==nil && H.workspace_detail==nil && H.ack=="original")',
    'inspection_refusal': 'H.command=command A.retry_command() H.host_callback({is_ok:false error:"management_workspace_inspection_required"}) assert(H.command==nil && H.workspace_detail==nil && H.ack=="original")',
    'success': 'H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.command==nil && H.workspace_mode=="result" && !H.workspace_detail.dirty && H.ack=="original")',
    'wrong_success': 'H.command=command A.retry_command() result.workspace.dirty=true H.host_callback({is_ok:true data:result}) assert(H.command.args.commandId=="original" && H.ack==nil)',
    'save_failed': 'A.save=fn(){return false} H.command=command A.retry_command() H.host_callback({is_ok:true data:result}) assert(H.workspace_mode=="result" && H.ack==nil)',
    'restart': 'A.recover_native({operation:"clear_workspace_dirty" commandId:"original" input:command.args}) H.callback({operation:"clear_workspace_dirty" data:result}) assert(H.workspace_mode=="result" && H.ack=="original" && H.calls.len()==1)',
    'wrong_restart': 'H.command=command A.recover_native({operation:"clear_workspace_dirty" commandId:"original" input:command.args}) result.workspace.workspaceId="other" H.callback({operation:"clear_workspace_dirty" data:result}) assert(H.command.args.commandId=="original" && H.ack==nil)',
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
