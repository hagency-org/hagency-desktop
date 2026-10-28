#!/usr/bin/env python3
"""Run real bundle callbacks in the pinned VM; HTTP and pixels are separate gates."""
import argparse
import json
import subprocess
from pathlib import Path
from check_full_bundle import APP, fixture

SETUP = '''
let H=mod.hagency let A=mod.readmission_actions let host=mod.readmission_host
A.redraw=fn(){} A.save=fn(){return true}
H.busy=false H.command=nil H.ready=true H.page="action"
H.session={account:"@owner:test" device:"original_device" deviceGeneration:3 administrator:"@admin:test" features:["resource-readmission-v1"]}
let admission={contribution:{id:"original_connection" requestId:"old_request" revision:4 resourceId:"original_resource" ownerMxid:"@owner:test" executorDeviceId:"original_device" executorGeneration:2 state:"revoked" label:"Original resource" coordinatorMxid:"@coordinator:test" verification:nil} request:{id:"old_request" contributionId:"original_connection" state:"revoked" canDecide:false}}
H.selected=admission
let detail={protocol:"hagency-management-v1" resource:{id:"original_resource" revision:"local_revision" framework:"codex" model:"test-model" reasoning:"medium" capabilities:["coding"]}}
let response={admission:admission observedAtMs:1000}
let old={offer:{requestId:"old_request" contributionId:"original_connection" label:"Original resource" resourceRevision:"old_revision" runtime:"codex" capabilities:["coding"] coordinatorMxid:"@coordinator:test" origin:"legacy_current"} request:{id:"old_request" contributionId:"original_connection" state:"revoked" approverMxid:"@admin:test"} current:false}
let sent=[] let callback=nil
host.request=fn(service,args,done){sent.push({service:service args:args}) callback=done}
let original_submit=A.submit
A.submit=fn(service,args){sent.push({service:service args:args})}
let load=fn(){A.begin_reoffer() callback({is_ok:true data:response}) callback({is_ok:true data:detail}) sent=[]}
let collect=fn(value){let texts=[] let walk=fn(v){if !v.is_object(){return} let {text=""}=v if text!=""{texts.push(text)} for i key child in v {walk(child)}} walk(value) return texts}
let contains=fn(values,text){for value in values {if value==text{return true}}return false}
'''

CASES = {
    'new_resource_coordinator_is_always_owner': 'H.wizard={label:" Resource " coordinator:"@someone-else:test" resource:{resourceId:"original_resource" revision:"local_revision"}} A.contribute() assert(sent.len()==1 && sent[0].args.coordinatorMxid==H.session.account && sent[0].args.label=="Resource")',
    'older_server_has_no_new_action': '''H.session.features=[] assert(!A.can_reoffer(H.selected)) A.begin_reoffer() A.open_connection_history("original_connection",nil,1) assert(sent.len()==0 && H.notice!="")''',
    'only_original_owner_device': '''H.session.account="@admin:test" assert(!A.can_reoffer(H.selected)) H.session.account="@owner:test" H.session.device="other_device" assert(!A.can_reoffer(H.selected)) H.session.device="original_device" H.session.deviceGeneration=1 assert(!A.can_reoffer(H.selected))''',
    'closed_and_expired_only': '''for state in ["admitted","pending_admission"]{admission.contribution.state=state admission.request.state="pending" assert(!A.can_reoffer(admission))} admission.request.state="expired" assert(A.can_reoffer(admission)) for state in ["revoked","rejected"] {admission.contribution.state=state assert(A.can_reoffer(admission))}''',
    'fresh_remote_then_local_read': '''A.begin_reoffer() assert(sent.len()==1 && sent[0].service=="hagency.connections.get" && sent[0].args.id=="original_connection" && H.reoffer_draft==nil) callback({is_ok:true data:response}) assert(sent.len()==2 && sent[1].service=="hagency.resources.get" && sent[1].args.id=="original_resource") callback({is_ok:true data:detail}) assert(!H.busy && H.reoffer_draft.expectedContributionRevision==4 && H.reoffer_draft.expectedRevision=="local_revision" && !H.reoffer_draft.review)''',
    'missing_admission_denied': '''A.begin_reoffer() callback({is_ok:true data:{observedAtMs:1000}}) assert(!H.busy && H.reoffer_draft==nil && sent.len()==1 && H.notice!="")''',
    'wrong_admission_denied': '''A.begin_reoffer() admission.contribution.id="other_connection" callback({is_ok:true data:response}) assert(!H.busy && H.reoffer_draft==nil && sent.len()==1 && H.notice!="")''',
    'mismatched_admission_request_denied': '''A.begin_reoffer() admission.request.id="unrelated_request" callback({is_ok:true data:response}) assert(!H.busy && H.reoffer_draft==nil && sent.len()==1 && H.notice!="")''',
    'remote_reopened_during_load': '''A.begin_reoffer() admission.contribution.state="admitted" callback({is_ok:true data:response}) assert(sent.len()==1 && H.reoffer_draft==nil && H.page=="action" && H.notice!="")''',
    'wrong_local_resource_denied': '''A.begin_reoffer() callback({is_ok:true data:response}) detail.resource.id="other_resource" callback({is_ok:true data:detail}) assert(H.reoffer_draft==nil && H.notice!="")''',
    'busy_or_pending_cannot_replace': '''H.busy=true A.begin_reoffer() H.busy=false H.command={service:"original_request"} A.begin_reoffer() assert(sent.len()==0 && H.reoffer_draft==nil)''',
    'review_required_before_submit': '''load() A.submit_reoffer() assert(sent.len()==0) A.reoffer_field("label"," Updated resource ") A.reoffer_field("coordinator"," @next:test ") A.review_reoffer() assert(sent.len()==0 && H.reoffer_draft.review) A.submit_reoffer() assert(sent.len()==1 && sent[0].service=="hagency.connections.reoffer" && sent[0].args.contributionId=="original_connection" && sent[0].args.expectedContributionRevision==4 && sent[0].args.resourceId=="original_resource" && sent[0].args.expectedRevision=="local_revision" && sent[0].args.label=="Updated resource" && sent[0].args.coordinatorMxid=="@owner:test")''',
    'blank_input_stays_editable': '''load() A.reoffer_field("label","  ") A.review_reoffer() assert(!H.reoffer_draft.review && sent.len()==0 && H.notice!="")''',
    'review_is_frozen': '''load() A.review_reoffer() A.reoffer_field("label","unreviewed") assert(H.reoffer_draft.label=="Original resource")''',
    'changed_connection_requires_review': '''load() A.review_reoffer() H.selected.contribution.revision=5 A.submit_reoffer() assert(sent.len()==0 && !H.reoffer_draft.review && H.notice!="")''',
    'changed_admission_requires_review': '''load() A.review_reoffer() H.selected.contribution.requestId="another_request" A.submit_reoffer() assert(sent.len()==0 && !H.reoffer_draft.review)''',
    'one_original_command_survives_unknown': '''load() A.review_reoffer() A.submit=original_submit A.submit_reoffer() assert(sent.len()==1 && sent[0].service=="hagency.intent.new") callback({is_ok:true data:{commandId:"one_original"}}) assert(sent.len()==2 && sent[1].args.commandId=="one_original") let command=H.command callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command==command && !H.busy) A.submit_reoffer() assert(sent.len()==2) A.retry_command() assert(sent.len()==3 && sent[2].args.commandId=="one_original" && sent[2].args.label=="Original resource")''',
    'known_refusal_keeps_draft_and_acknowledges_original': '''load() A.review_reoffer() H.command={service:"hagency.connections.reoffer" args:{commandId:"refused_original"}} A.retry_command() callback({is_ok:false error:"contribution_native_fence_required"}) assert(H.command==nil && H.reoffer_draft!=nil && !H.reoffer_draft.review && sent.len()==2 && sent[1].service=="hagency.intent.acknowledge" && sent[1].args.commandId=="refused_original")''',
    'success_opens_same_connection': '''load() let opened="" A.open_connection=fn(id){opened=id} A.acknowledge=fn(id){} H.command={service:"hagency.connections.reoffer" args:{commandId:"original"}} A.retry_command() callback({is_ok:true data:response}) assert(H.command==nil && H.reoffer_draft==nil && opened=="original_connection")''',
    'native_restart_recovery_opens_original': '''let opened="" A.open_connection=fn(id){opened=id} A.acknowledge=fn(id){} H.reoffer_draft={} A.recover_native({commandId:"original" operation:"reoffer"}) assert(sent.len()==1 && sent[0].service=="hagency.intent.retry" && sent[0].args.commandId=="original") callback({is_ok:true data:{operation:"reoffer" data:response}}) assert(H.reoffer_draft==nil && opened=="original_connection")''',
    'history_exact_identity_and_cursor': '''A.open_connection_history("original_connection","old_cursor",2) assert(sent[0].service=="hagency.connections.history" && sent[0].args.id=="original_connection" && sent[0].args.after=="old_cursor") callback({is_ok:true data:{protocol:"hagency-management-v1" items:[old] nextAfter:"next_cursor"}}) assert(H.connection_history.items.len()==1 && H.connection_history.items[0].request.id=="old_request" && H.history_page==2)''',
    'history_rejects_wrong_connection': '''A.open_connection_history("original_connection",nil,1) old.offer.contributionId="other" callback({is_ok:true data:{protocol:"hagency-management-v1" items:[old] nextAfter:nil}}) assert(H.connection_history==nil && H.notice!="")''',
    'history_rejects_mismatched_request': '''A.open_connection_history("original_connection",nil,1) old.request.id="unrelated" callback({is_ok:true data:{protocol:"hagency-management-v1" items:[old] nextAfter:nil}}) assert(H.connection_history==nil)''',
    'history_rejects_cursor_loop': '''A.open_connection_history("original_connection","same",2) callback({is_ok:true data:{protocol:"hagency-management-v1" items:[old] nextAfter:"same"}}) assert(H.connection_history==nil && H.notice!="")''',
    'history_empty_is_not_a_fence_proof': '''A.open_connection_history("original_connection",nil,1) callback({is_ok:true data:{protocol:"hagency-management-v1" items:[] nextAfter:nil}}) assert(H.connection_history.items.len()==0 && sent.len()==1 && H.reoffer_draft==nil)''',
    'render_retained_legacy_history': '''H.page="connection_history" H.connection_history={items:[old] nextAfter:nil} let page=View{@@PAGES@@} let texts=collect(page) assert(contains(texts,A.t("Admission history","准入历史")) && contains(texts,A.t("Captured during upgrade. This is the definition available then, not a reconstructed original submission.","此定义在升级时保存，反映当时可读取的配置，未重建最初提交内容。")) && contains(texts,A.t("Request","申请")+" · old_request"))''',
    'render_history_page_controls': '''for cursor in [nil,"older_page"] {for next in [nil,"next_page"] {H.page="connection_history" H.history_after=cursor H.connection_history={items:[old] nextAfter:next} let page=View{@@PAGES@@} let texts=collect(page) let visible=cursor!=nil || next!=nil assert(contains(texts,A.t("First page","第一页"))==visible && contains(texts,A.t("Next page","下一页"))==visible) let controls={first:false more:false} let scan=fn(v){if !v.is_object(){return} let {text="",enabled=false}=v if text==A.t("First page","第一页"){controls.first=enabled} if text==A.t("Next page","下一页"){controls.more=enabled} for i key child in v {scan(child)}} scan(page) assert(controls.first==(cursor!=nil) && controls.more==(next!=nil))}}''',
    'form_is_read_only_during_pending_work': '''load() for busy in [false,true] {for command in [nil,{service:"original_request"}] {H.busy=busy H.command=command let page=View{@@PAGES@@} let fields=[] let scan=fn(v){if !v.is_object(){return} let {is_read_only=nil text=""}=v if is_read_only!=nil{fields.push(v)} for i key child in v{scan(child)}} scan(page) assert(fields.len()==1) for field in fields{assert(field.is_read_only==(busy || command!=nil))} let before=H.reoffer_draft.label A.reoffer_field("label","edited") assert(H.reoffer_draft.label==if busy || command!=nil{before}else{"edited"})}}''',
    'render_review_and_new_approval': '''load() A.review_reoffer() let page=View{@@PAGES@@} let texts=collect(page) assert(contains(texts,A.t("2 / 2 · Review a new request","2 / 2 · 核对新申请")) && contains(texts,A.t("Submit new admission request","提交新准入申请")) && contains(texts,"Original resource") && contains(texts,"@owner:test"))''',
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('vm', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    source = (APP / 'bundle/main.splash').read_text()
    pages = source[source.index('        if H.page=="connection_history" {mod.widgets.HagencyDetailCard{'):source.index('        if H.page == "action" && H.selected != nil {mod.widgets.HagencyDetailCard{')]
    results = []
    for language in ['en', 'zh-CN']:
        for name, code in CASES.items():
            path = args.output / f'{language}-{name}.splash'
            path.write_text(fixture(source + '\nmod.readmission_actions=A\nmod.readmission_host=host\n', language) + SETUP + code.replace('@@PAGES@@', pages) + '\ntrue\n')
            result = subprocess.run([str(args.vm.resolve()), 'eval', str(path)], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            path.with_suffix('.log').write_text(result.stdout)
            results.append({'language': language, 'case': name, 'passed': result.returncode == 0})
    (args.output / 'report.json').write_text(json.dumps(results, indent=2) + '\n')
    print(json.dumps({'passed': sum(row['passed'] for row in results), 'total': len(results), 'failed': [row for row in results if not row['passed']]}))
    raise SystemExit(not all(row['passed'] for row in results))


if __name__ == '__main__':
    main()
