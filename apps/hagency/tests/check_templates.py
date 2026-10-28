#!/usr/bin/env python3
"""Exercise shipped resource-template UX through real Splash VM callbacks.

The complete bundle is evaluated in both languages with inert widgets. Native
CAS/persistence and pixels require the separate native tests and UI capture.
"""
import argparse
import json
import subprocess
from pathlib import Path
from check_full_bundle import APP, fixture

SETUP = r'''
let H=mod.hagency let A=mod.template_actions let t=H.templates
A.redraw=fn(){return nil} H.busy=false H.ready=true H.notice="" H.command=nil H.saved_commands=[]
let calls=[] let replies=[]
host.request=fn(service,args,done){calls.push({service:service args:args}) replies.push(done)}
let revision="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
let original_id="templatecmd_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
let definition={id:"rad_original" name:"project-writer" role:"documentation" enabled:true createdAt:1 status:"defined" activeEngagements:0}
let list={resourceId:"resource_original" resourceName:"My Codex resource" revision:revision items:[definition] nextAfter:"rad_original" roles:["documentation","architect"] canCreate:true}
let detail={resourceId:"resource_original" revision:revision definition:definition roles:["documentation","architect"] canEditIdentity:true canToggle:true canDelete:true}
let prepared={commandId:original_id resourceId:"resource_original" definitionId:"rad_original" operation:"template_create" expectedRevision:revision change:{operation:"create" name:"project-writer" role:"documentation" enabled:true} result:nil error:nil}
let receipt={commandId:original_id resourceId:"resource_original" definitionId:"rad_original" deleted:false revision:"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}
let set_list=fn(){H.page="templates" t.resourceId="resource_original" t.list=list}
let set_detail=fn(){H.page="template" t.resourceId="resource_original" t.id="rad_original" t.detail=detail}
let start=fn(){set_list() A.template_begin(true) t.draft.name="project-writer" A.template_role("documentation")}
let widgets=[] let texts=[]
let visit=fn(n){if !n.is_object(){return} for index key child in n{widgets.push({name:key.to_string() value:child}) if key.to_string()=="text" && child.is_string(){texts.push(child)} visit(child)}}
let find=fn(name){for row in widgets{if row.name==name{return row.value}} return nil}
let count=fn(name){let total=0 for row in widgets{if row.name==name{total=total+1}} return total}
let has=fn(text){for value in texts{if value==text{return true}} return false}
'''

CASES = {
    'list_uses_exact_resource': '''A.open_templates("resource_original",nil) assert(calls[0].service=="hagency.templates.list" && calls[0].args.resourceId=="resource_original") replies[0]({is_ok:true data:list}) assert(!H.busy && t.list==list)''',
    'list_opens_exact_template': '''set_list() RENDER find("template_open").on_click() assert(calls[0].service=="hagency.templates.get" && calls[0].args.id=="rad_original" && calls[0].args.resourceId=="resource_original") replies[0]({is_ok:true data:detail}) assert(t.detail==detail)''',
    'pagination_uses_native_cursor': '''set_list() list.items=[] RENDER assert(has(A.t("No templates on this page.","本页暂无模板。"))) find("templates_next").on_click() assert(calls[0].args.after=="rad_original")''',
    'first_page_clears_cursor': '''set_list() t.after="rad_previous" RENDER find("templates_first").on_click() assert(calls[0].args.after==nil)''',
    'detail_back_keeps_page': '''set_detail() t.after="rad_previous" RENDER find("template_back").on_click() assert(calls[0].service=="hagency.templates.list" && calls[0].args.after=="rad_previous")''',
    'stale_resource_is_ignored': '''A.open_templates("resource_original",nil) t.resourceId="resource_other" replies[0]({is_ok:true data:list}) assert(t.list==nil)''',
    'stale_page_is_ignored': '''A.open_template("resource_original","rad_original") H.page="overview" replies[0]({is_ok:true data:detail}) assert(H.page=="overview" && t.detail==nil && !H.busy)''',
    'stale_epoch_does_not_unlock_new_read': '''A.open_template("resource_original","rad_original") t.epoch=t.epoch+1 H.busy=false A.open_templates("resource_original",nil) replies[0]({is_ok:true data:detail}) assert(H.busy && t.detail==nil) replies[1]({is_ok:true data:list}) assert(!H.busy && t.list==list)''',
    'wrong_detail_identity_refused': '''A.open_template("resource_original","rad_original") definition.id="rad_other" replies[0]({is_ok:true data:detail}) assert(t.detail==nil && t.error!="")''',
    'oversize_page_refused': '''A.open_templates("resource_original",nil) list.items=[] for i in 0..17{list.items.push(definition)} replies[0]({is_ok:true data:list}) assert(t.list==nil && t.error!="")''',
    'oversize_role_choices_refused': '''A.open_templates("resource_original",nil) list.roles=[] for i in 0..17{list.roles.push("documentation")} replies[0]({is_ok:true data:list}) assert(t.list==nil)''',
    'bounded_list_renders_each_definition': '''set_list() list.items=[] for i in 0..16{list.items.push({id:"rad_"+i.to_string() name:"agent-"+i.to_string() role:"documentation" enabled:true status:"defined"})} RENDER assert(count("template_open")==16)''',
    'no_native_create_authority_disables_entry': '''set_list() list.canCreate=false RENDER assert(!find("template_new").enabled) A.template_begin(true) assert(t.draft==nil && calls.len()==0)''',
    'new_editor_uses_only_supported_roles': '''set_list() RENDER find("template_new").on_click() assert(t.draft.expectedRevision==revision && t.draft.role=="") A.template_role("fabricated") assert(t.draft.role=="") A.template_role("architect") assert(t.draft.role=="architect")''',
    'editor_callbacks_capture_name_role_and_enabled': '''start() RENDER assert(count("template_role")==2 && has(A.template_notice())) find("template_name").on_change("reader") find("template_enabled").on_click() assert(t.draft.name=="reader" && !t.draft.enabled)''',
    'blank_name_prevents_prepare': '''start() t.draft.name=" " A.template_save_draft() assert(calls.len()==0 && t.phase=="editing" && t.error!="")''',
    'missing_role_prevents_prepare': '''start() t.draft.role="" A.template_save_draft() assert(calls.len()==0 && t.error!="")''',
    'unsupported_role_prevents_prepare': '''start() t.draft.role="invented" A.template_save_draft() assert(calls.len()==0 && t.error==A.template_reason("management_model_unqualified"))''',
    'create_prepares_without_client_identity_or_apply': '''start() A.template_save_draft() assert(calls.len()==1 && calls[0].service=="hagency.templates.create") assert(calls[0].args.to_json()=={resourceId:"resource_original" expectedRevision:revision name:"project-writer" role:"documentation" enabled:true}.to_json()) replies[0]({is_ok:true data:prepared}) assert(t.phase=="saved" && t.pending==prepared && calls.len()==1)''',
    'in_use_identity_is_locked_but_toggle_available': '''set_detail() detail.canEditIdentity=false detail.canDelete=false definition.status="provisioned" definition.activeEngagements=2 A.template_begin(false) RENDER assert(find("template_name").is_read_only && count("template_role")==0) find("template_name").on_change("replacement") A.template_role("architect") find("template_enabled").on_click() assert(t.draft.name=="project-writer" && t.draft.role=="documentation" && !t.draft.enabled)''',
    'disable_unqualified_existing_role_is_available': 'set_detail() detail.roles=[] detail.canEditIdentity=false detail.canDelete=false A.template_begin(false) t.draft.enabled=false A.template_save_draft() assert(calls.len()==1 && calls[0].service=="hagency.templates.update" && calls[0].args.role=="documentation" && !calls[0].args.enabled)',
    'reenable_unqualified_role_is_refused': 'set_detail() detail.roles=[] definition.enabled=false A.template_begin(false) t.draft.enabled=true A.template_save_draft() assert(calls.len()==0 && t.error!="")',
    'in_use_delete_is_absent_and_refused': '''set_detail() detail.canDelete=false RENDER assert(find("template_delete")==nil) A.template_delete() assert(calls.len()==0)''',
    'readonly_detail_never_editable': '''set_detail() detail.canEditIdentity=false detail.canToggle=false detail.canDelete=false RENDER assert(!find("template_edit").enabled) A.template_begin(false) assert(t.draft==nil)''',
    'update_retains_original_identity_and_revision': '''set_detail() A.template_begin(false) t.draft.enabled=false A.template_save_draft() assert(calls[0].service=="hagency.templates.update" && calls[0].args.id=="rad_original" && calls[0].args.expectedRevision==revision && !calls[0].args.enabled)''',
    'delete_prepares_exact_record_only': '''set_detail() RENDER find("template_delete").on_click() assert(calls.len()==1 && calls[0].service=="hagency.templates.delete" && calls[0].args.to_json()=={resourceId:"resource_original" id:"rad_original" expectedRevision:revision}.to_json()) prepared.operation="template_delete" prepared.change.operation="delete" replies[0]({is_ok:true data:prepared}) assert(t.phase=="saved" && calls.len()==1)''',
    'definite_prepare_refusal_keeps_draft': '''start() A.template_save_draft() replies[0]({is_ok:false error:"management_revision_conflict"}) assert(H.page=="template_edit" && t.phase=="editing" && t.draft.name=="project-writer" && t.pending==nil)''',
    'in_use_prepare_refusal_is_localized': '''set_detail() A.template_delete() replies[0]({is_ok:false error:"management_resource_in_use"}) assert(H.page=="template" && t.phase=="" && t.error==A.template_reason("management_resource_in_use"))''',
    'unqualified_prepare_refusal_keeps_editor': '''start() A.template_save_draft() replies[0]({is_ok:false error:"management_model_unqualified"}) assert(H.page=="template_edit" && t.phase=="editing" && t.error!= "management_model_unqualified")''',
    'unknown_prepare_blocks_replacement': '''start() A.template_save_draft() replies[0]({is_ok:false error:"management_outcome_unknown"}) A.template_save_draft() A.template_begin(true) A.template_apply() assert(calls.len()==1 && t.phase=="unknown" && t.pending==nil)''',
    'wrong_prepare_resource_is_not_applied': '''start() A.template_save_draft() prepared.resourceId="resource_other" replies[0]({is_ok:true data:prepared}) assert(t.phase=="unknown" && t.pending==nil)''',
    'wrong_prepare_revision_is_not_applied': '''start() A.template_save_draft() prepared.expectedRevision="changed" replies[0]({is_ok:true data:prepared}) assert(t.phase=="unknown" && t.pending==nil)''',
    'saved_restart_routes_original_command': '''A.recover_native({commandId:original_id operation:"template_create"}) assert(calls[0].service=="hagency.templates.command.get" && calls[0].args.commandId==original_id) replies[0]({is_ok:true data:prepared}) assert(t.pending==prepared && t.phase=="saved" && calls.len()==1)''',
    'wrong_saved_command_is_refused': '''A.open_saved_template({commandId:"templatecmd_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}) replies[0]({is_ok:true data:prepared}) assert(t.pending==nil && t.phase=="unknown")''',
    'saved_variant_mismatch_is_refused': '''prepared.operation="template_delete" A.template_prepared(prepared) assert(t.pending==nil && t.phase=="unknown")''',
    'exact_saved_fields_reviewed': '''A.template_prepared(prepared) RENDER assert(has("project-writer") && has(A.capability_label("documentation")) && has(A.template_enabled(true)) && has(A.template_notice()))''',
    'apply_sends_only_original_command_id_once': '''A.template_prepared(prepared) RENDER find("template_command_apply").on_click() A.template_apply() assert(calls.len()==1 && calls[0].service=="hagency.templates.command.apply" && calls[0].args.to_json()=={commandId:original_id}.to_json()) replies[0]({is_ok:true data:receipt}) assert(t.phase=="recorded") A.template_apply() assert(calls.len()==1)''',
    'unknown_apply_removes_apply_and_checks_original': '''A.template_prepared(prepared) A.template_apply() replies[0]({is_ok:false error:"management_outcome_unknown"}) RENDER assert(find("template_command_apply")==nil && t.pending.commandId==original_id) find("template_command_check").on_click() assert(calls[1].service=="hagency.templates.command.get" && calls[1].args.commandId==original_id)''',
    'failed_original_read_keeps_id_for_retry': '''A.open_saved_template({commandId:original_id}) replies[0]({is_ok:false error:"unavailable"}) A.template_check() assert(calls.len()==2 && calls[1].service=="hagency.templates.command.get" && calls[1].args.commandId==original_id && t.pending==nil)''',
    'recovered_saved_command_does_not_autoapply': '''A.template_prepared(prepared) A.template_apply() replies[0]({is_ok:false error:"management_outcome_unknown"}) A.template_check() replies[1]({is_ok:true data:prepared}) assert(t.phase=="saved" && calls.len()==2)''',
    'mismatched_receipt_remains_unknown': '''A.template_prepared(prepared) A.template_apply() receipt.definitionId="rad_other" replies[0]({is_ok:true data:receipt}) assert(t.phase=="unknown" && prepared.result==nil)''',
    'late_apply_reply_keeps_new_view': '''A.template_prepared(prepared) A.template_apply() H.page="overview" replies[0]({is_ok:true data:receipt}) assert(H.page=="overview" && prepared.result==nil)''',
    'saved_receipt_mismatch_is_not_acknowledged': '''prepared.result=receipt receipt.resourceId="other" A.template_prepared(prepared) A.template_finish() assert(t.pending==nil && t.phase=="unknown" && calls.len()==0)''',
    'recorded_change_acknowledges_only_original': '''prepared.result=receipt A.template_prepared(prepared) H.saved_commands=[{commandId:original_id operation:"template_create"},{commandId:"keep" operation:"task_comment"}] A.template_finish() assert(calls[0].service=="hagency.intent.acknowledge" && calls[0].args.commandId==original_id) replies[0]({is_ok:true data:{}}) assert(t.pending==nil && t.commandId=="" && H.saved_commands.len()==1 && H.saved_commands[0].commandId=="keep" && calls[1].service=="hagency.templates.list")''',
    'failed_ack_preserves_receipt': '''prepared.result=receipt A.template_prepared(prepared) A.template_finish() replies[0]({is_ok:false error:"unavailable"}) assert(t.pending.result==receipt && t.phase=="recorded" && calls.len()==1)''',
    'failed_saved_command_can_close_but_not_apply': '''prepared.error="management_resource_in_use" A.template_prepared(prepared) RENDER assert(t.phase=="failed" && find("template_command_apply")==nil && find("template_command_close")!=nil)''',
    'pending_saved_command_blocks_new_editor': '''set_list() H.saved_commands=[{commandId:original_id operation:"template_create" hasReply:false}] A.template_begin(true) assert(t.draft==nil && calls.len()==0)''',
    'other_active_command_is_preserved': '''set_list() let other={service:"other" args:{id:"preserve"}} H.command=other A.template_begin(true) assert(t.draft==nil && H.command==other)''',
    'busy_blocks_reads_writes_and_edits': '''set_list() H.busy=true A.open_templates("other",nil) A.template_begin(true) A.open_template("other","other") A.template_delete() assert(calls.len()==0 && t.draft==nil && t.resourceId=="resource_original")''',
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('vm', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    source = (APP / 'bundle/main.splash').read_text()
    start = source.index('        if H.page=="templates"{')
    end = source.index('        if H.page == "resource_detail" {', start)
    render = '\nlet tree=View{\n' + source[start:end] + '\n}\nvisit(tree)\n'
    results = []
    for language in ['en', 'zh-CN']:
        prefix = fixture(source + '\nmod.template_actions=A\n', language)
        for name, body in CASES.items():
            script = args.output / f'{language}-{name}.splash'
            script.write_text(prefix + SETUP + body.replace('RENDER', render) + '\ntrue\n')
            run = subprocess.run([str(args.vm.resolve()), 'eval', str(script)], text=True,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            script.with_suffix('.log').write_text(run.stdout)
            results.append({'case': name, 'language': language, 'passed': run.returncode == 0})
    (args.output / 'report.json').write_text(json.dumps({'checks': results}, indent=2) + '\n')
    print(json.dumps({'passed': sum(x['passed'] for x in results), 'total': len(results)}))
    raise SystemExit(not all(x['passed'] for x in results))


if __name__ == '__main__':
    main()
