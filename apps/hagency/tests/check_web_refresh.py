#!/usr/bin/env python3
"""Exercise retained management refresh in the real Splash VM, in EN/CN.

The full shipped bundle is evaluated. Native replies and visibility delivery are
controlled fixtures; this does not claim an OS focus event or a live server E2E.
"""
import argparse
import json
import subprocess
from pathlib import Path
from check_full_bundle import APP, fixture

SETUP = r'''
let H=mod.hagency let A=mod.refresh_actions let w=H.web let c=H.webConnection let r=H.webRefresh
H.ready=true H.busy=false H.needs_signin=false H.command=nil H.confirm="" H.notice="" H.saved_commands=[]
A.redraw=fn(){return nil}
let calls=[] let replies=[] let delays=[]
host.request=fn(service,args,done){calls.push({service:service args:args.to_json().parse_json()}) replies.push(done)}
start_timeout=fn(delay,done){delays.push({delay:delay done:done})}
let old={items:[] marker:"old" nextAfter:nil}
let fresh={items:[] marker:"fresh" nextAfter:nil}
let view=fn(kind){H.page="web_overview" w.kind=kind w.after=nil w.readKind=kind w.readQuery={after:nil} w.data=old}
let settings=fn(){H.page="web_catalog" w.resourceId="resource_a" w.section="settings" w.after=nil w.catalog={resourceId:"resource_a" revision:"original" rateCapPerDay:100} w.rate="100"}
let connection=fn(){H.page="web_connection" c.id="server.example" c.detail={id:c.id allocatedTokens:100 revision:1} c.tokens="100"}
let answer=fn(index,value){replies[index]({is_ok:true data:value})}
'''
CASES = {
    'timer_observes_without_clearing_data_or_notice': 'view("usage") H.notice="Keep this notice" A.web_auto_refresh() assert(calls.len()==1 && w.data==old && H.busy && r.inFlight && delays.len()==1 && delays[0].delay==15) answer(0,fresh) assert(w.data==fresh && H.notice=="Keep this notice" && !H.busy && !r.inFlight)',
    'timer_waits_while_hidden': 'view("usage") mod.refresh_visibility("hidden") A.web_auto_refresh() assert(calls.len()==0 && delays.len()==1 && !r.visible)',
    'focus_return_reads_immediately_without_an_extra_timer': 'view("workforce") mod.refresh_visibility("hidden") mod.refresh_visibility("visible") assert(calls.len()==1 && calls[0].service=="hagency.web.workforce" && delays.len()==0)',
    'inflight_read_is_not_overlapped': 'view("usage") A.web_refresh_now() A.web_refresh_now() mod.refresh_visibility("visible") assert(calls.len()==1) answer(0,fresh) A.web_refresh_now() assert(calls.len()==2)',
    'hidden_reply_is_not_applied_and_return_retries': 'view("usage") A.web_refresh_now() mod.refresh_visibility("hidden") answer(0,fresh) assert(w.data==old && !H.busy) mod.refresh_visibility("visible") assert(calls.len()==2)',
    'hide_then_show_invalidates_old_generation': 'view("usage") A.web_refresh_now() mod.refresh_visibility("hidden") mod.refresh_visibility("visible") answer(0,fresh) assert(w.data==old && !H.busy) A.web_refresh_now() assert(calls.len()==2)',
    'not_ready_or_needing_signin_does_not_start_authentication': 'view("usage") H.ready=false A.web_refresh_now() H.ready=true H.needs_signin=true A.web_refresh_now() assert(calls.len()==0)',
    'explicit_native_request_stays_exclusive': 'view("usage") H.busy=true A.web_refresh_now() assert(calls.len()==0 && H.busy)',
    'original_command_is_preserved': 'view("usage") let command={service:"hagency.projects.decide" args:{commandId:"original"}} H.command=command A.web_refresh_now() assert(calls.len()==0 && H.command==command)',
    'saved_review_and_unknown_outcome_are_preserved': 'view("usage") for phase in ["saving","saved","submitting","unknown"]{w.phase=phase A.web_refresh_now()} w.phase="" w.pending={commandId:"original"} A.web_refresh_now() assert(calls.len()==0 && w.pending.commandId=="original")',
    'confirmation_is_never_replaced': 'view("usage") H.confirm="approve" A.web_refresh_now() assert(calls.len()==0 && H.confirm=="approve")',
    'account_resource_cursor_survives_automatic_and_explicit_refresh': 'A.open_web_account("account_a","resource_previous") answer(0,old) A.web_refresh_now() assert(calls[1].args.to_json()=={id:"account_a" resourceAfter:"resource_previous"}.to_json()) answer(1,fresh) A.refresh_page() assert(calls[2].args.to_json()==calls[1].args.to_json())',
    'overview_cursor_survives_refresh': 'A.open_web("usage","resource_previous") answer(0,old) A.web_refresh_now() assert(calls[1].args.after=="resource_previous")',
    'alerts_keep_applied_filters_and_cursor': 'w.status="open" w.severity="critical" w.agent="resource_a" A.open_web("alerts","alert_previous") answer(0,old) A.web_refresh_now() let q=calls[1].args assert(q.after=="alert_previous" && q.status=="open" && q.severity=="critical" && q.agent=="resource_a")',
    'unsaved_alert_filter_is_not_submitted': 'w.status="open" A.open_web("alerts",nil) answer(0,old) w.agent="typing" A.web_refresh_now() assert(calls.len()==1 && w.agent=="typing" && w.data==old)',
    'typing_alert_filter_while_reading_discards_reply': 'w.status="open" A.open_web("alerts",nil) answer(0,old) A.web_refresh_now() w.agent="typing" answer(1,fresh) assert(w.data==old && w.agent=="typing" && !H.busy)',
    'page_return_issues_fresh_read': 'view("usage") H.page="overview" A.open_web("usage",nil) assert(calls.len()==1 && calls[0].service=="hagency.web.usage") answer(0,fresh) assert(w.data==fresh)',
    'reply_cannot_redirect_changed_page': 'view("usage") A.web_refresh_now() H.page="resource_edit" let draft={model:"typed"} H.resource_edit_draft=draft answer(0,fresh) assert(H.page=="resource_edit" && H.resource_edit_draft==draft && w.data==old)',
    'reply_cannot_replace_new_account_selection': 'A.open_web_account("account_a","one") answer(0,old) A.web_refresh_now() w.readQuery={id:"account_b" resourceAfter:"two"} answer(1,fresh) assert(w.data==old)',
    'catalog_rate_draft_survives_focus_and_manual_refresh': 'settings() w.rate="250" mod.refresh_visibility("visible") A.refresh_page() assert(calls.len()==0 && w.rate=="250" && w.catalog.revision=="original")',
    'catalog_offer_draft_survives': 'settings() w.section="offers" w.draft={role:"coding" count:"5"} A.web_refresh_now() assert(calls.len()==0 && w.draft.count=="5")',
    'trust_draft_survives': 'settings() w.section="trust" w.trustRoom="!draft:example.org" A.web_refresh_now() w.trustRoom="" w.trustName="draft" A.web_refresh_now() assert(calls.len()==0)',
    'clean_catalog_updates_without_navigation': 'settings() A.web_refresh_now() assert(calls[0].service=="hagency.web_catalog.get") answer(0,{resourceId:"resource_a" revision:"new" rateCapPerDay:200}) assert(w.rate=="200" && w.catalog.revision=="new" && H.page=="web_catalog")',
    'rate_typing_during_read_preserves_original_revision': 'settings() A.web_refresh_now() w.rate="321" answer(0,{resourceId:"resource_a" revision:"new" rateCapPerDay:200}) assert(w.rate=="321" && w.catalog.revision=="original")',
    'connection_allowance_draft_is_not_reset': 'connection() c.tokens="321" mod.refresh_visibility("visible") A.refresh_page() assert(calls.len()==0 && c.tokens=="321" && c.detail.revision==1)',
    'clean_connection_update_preserves_identity': 'connection() A.web_refresh_now() answer(0,{id:c.id allocatedTokens:200 revision:2}) assert(c.tokens=="200" && c.detail.revision==2 && H.page=="web_connection")',
    'connection_wrong_identity_is_ignored': 'connection() A.web_refresh_now() answer(0,{id:"other.example" allocatedTokens:200 revision:2}) assert(c.tokens=="100" && c.detail.revision==1)',
    'connection_list_preserves_cursor': 'H.page="web_connections" c.after="previous.example" A.web_refresh_now() assert(calls[0].service=="hagency.web_connections.list" && calls[0].args.after=="previous.example")',
    'wizard_and_native_configuration_forms_are_not_refreshed': 'for page in ["wizard","project_wizard","agent_wizard","resource_edit","resource_create","personal_agent","web_connection_new","web_catalog_command","provider_connect","account_prepare"]{H.page=page A.web_refresh_now()} assert(calls.len()==0)',
    'resource_name_editor_is_preserved': 'H.page="resource_detail" H.resource_id="resource_a" H.resource_name_mode="edit" H.resource_name_draft={name:"typing"} A.web_refresh_now() assert(calls.len()==0 && H.resource_name_draft.name=="typing")',
    'expanded_resource_list_is_not_truncated': 'H.page="resources" for i in 0..17{H.resources.push({id:"resource_"+i})} A.web_refresh_now() assert(calls.len()==0 && H.resources.len()==17)',
    'resource_detail_refresh_keeps_selected_id': 'H.page="resource_detail" H.resource_id="resource_a" A.web_refresh_now() assert(calls[0].service=="hagency.resources.get") answer(0,{protocol:"hagency-management-v1" resource:{id:"resource_a"}}) assert(H.resource_detail.resource.id=="resource_a")',
    'alert_note_draft_is_preserved': 'H.page="resource_alert" H.alert_draft={note:"typing"} A.web_refresh_now() assert(calls.len()==0 && H.alert_draft.note=="typing")',
    'provider_account_member_cursor_is_preserved': 'H.page="provider_account" H.account_id="account_a" H.account_resource_after="resource_previous" H.account_models={selected:"keep"} A.web_refresh_now() assert(calls[0].args.id=="account_a" && calls[0].args.after=="resource_previous") answer(0,{protocol:"hagency-management-v1" account:{id:"account_a"}}) assert(H.account_models.selected=="keep")',
    'error_keeps_last_data_and_allows_next_refresh': 'view("usage") A.web_refresh_now() replies[0]({is_ok:false error:"unavailable"}) assert(w.data==old && !H.busy && !r.inFlight) A.web_refresh_now() assert(calls.len()==2)',
    'auth_error_requires_explicit_login': 'view("usage") A.web_refresh_now() replies[0]({is_ok:false error:"native_resume_revoked"}) A.web_refresh_now() assert(H.needs_signin && calls.len()==1 && w.data==old)',
    'all_overview_kinds_use_only_closed_read_routes': 'for kind in ["workforce","capability","usage","accounts","alerts","terminal"]{view(kind) if kind=="alerts"{w.readQuery={after:nil status:nil severity:nil agent:nil}} if kind=="terminal"{w.readQuery={id:"agent_a"}} A.web_refresh_now() assert(calls[calls.len()-1].service=="hagency.web."+kind) answer(replies.len()-1,fresh)} assert(calls.len()==6)',
    'terminal_wrong_identity_keeps_original_capture': 'view("terminal") w.readQuery={id:"agent_a"} A.web_refresh_now() answer(0,{id:"agent_b" text:"wrong"}) assert(w.data==old)',
    'inbox_refresh_is_read_only_and_preserves_saved_ids': 'H.page="inbox" H.saved_commands=[{commandId:"saved_original" operation:"web_catalog" hasReply:false}] A.web_refresh_now() assert(calls[0].service=="hagency.inbox.list") answer(0,{items:[] nextAfter:nil}) assert(H.saved_commands[0].commandId=="saved_original")',
    'agent_list_keeps_current_project': 'H.page="agents" H.agent_project_id="project_a" A.web_refresh_now() assert(calls[0].service=="hagency.agents.list" && calls[0].args.projectId=="project_a" && !calls[0].args.needsMyAction)',
    'admission_refresh_keeps_current_request': 'H.page="action" H.selected={contribution:{id:"admission_a" verification:nil}} A.web_refresh_now() assert(calls[0].service=="hagency.connections.get" && calls[0].args.id=="admission_a") answer(0,{admission:{contribution:{id:"admission_a" verification:nil}} observedAtMs:1}) assert(H.selected.contribution.id=="admission_a" && H.page=="action")',
    'unknown_overview_kind_never_constructs_a_mutating_route': 'view("catalog.command.apply") A.web_refresh_now() assert(calls.len()==0)',
}

def main():
    p=argparse.ArgumentParser()
    p.add_argument('vm',type=Path)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
    source=(APP/'bundle/main.splash').read_text()
    results=[]
    for language in ['en','zh-CN']:
        prefix=fixture(source+'\nmod.refresh_actions=A\nmod.refresh_visibility=on_app_visibility_changed\n',language)
        for name,case in CASES.items():
            script=a.output/f'{language}-{name}.splash'
            script.write_text(prefix+SETUP+case+'\ntrue\n')
            run=subprocess.run([str(a.vm.resolve()),'eval',str(script)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
            script.with_suffix('.log').write_text(run.stdout)
            results.append({'case':name,'language':language,'passed':run.returncode==0})
    (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n')
    print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results),'failed':[r for r in results if not r['passed']]}))
    raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
