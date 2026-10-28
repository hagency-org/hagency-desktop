#!/usr/bin/env python3
"""Exercise the shipped console controls in the actual Splash VM, in EN/CN.

Widgets and native replies are fixtures; native persistence and pixels have
separate integration checks. These checks exercise rendered callbacks, original
command recovery, stale views and nullable measurements, not source substrings.
"""
import argparse
import json
import subprocess
from pathlib import Path
from check_full_bundle import APP, fixture

SETUP = r'''
let H=mod.hagency let A=mod.web_actions let w=H.web
A.redraw=fn(){return nil} H.busy=false H.ready=true H.command=nil H.saved_commands=[]
let calls=[] let replies=[]
host.request=fn(service,args,done){calls.push({service:service args:args}) replies.push(done)}
let revision="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
let id="webcatalog_0123456789abcdef0123456789abcdef"
let catalog={resourceId:"resource_a" resourceName:"Writer" revision:revision rateCapPerDay:nil rateEnforced:false defaultYolo:false canRetire:true retireBlockedReason:nil agents:[{id:"eng_original" name:"Unconfigured agent" resourceId:""}] nextAfter:nil}
let change={operation:"rate_cap" rateCapPerDay:5000}
let prepared={commandId:id resourceId:"resource_a" expectedRevision:revision change:change result:nil error:nil}
let receipt={commandId:id resourceId:"resource_a" revision:revision operation:"rate_cap" retired:false serverAutoApproval:false}
let start=fn(){H.page="web_catalog" w.resourceId="resource_a" w.section="settings" w.catalog=catalog w.rate="5000"}
let widgets=[] let texts=[]
let visit=fn(n){if !n.is_object(){return} for index key child in n{widgets.push({name:key.to_string() value:child}) if key.to_string()=="text" && child.is_string(){texts.push(child)} visit(child)}}
let find=fn(name){for row in widgets{if row.name==name{return row.value}} return nil}
let has=fn(text){for t in texts{if t==text{return true}} return false}
let count=fn(name){let n=0 for row in widgets{if row.name==name{n+=1}} return n}
let fleet=fn(canOpen){H.page="web_overview" w.kind="workforce" w.data={items:[{id:"request_a" name:"Writer" state:"approved" projectId:"project_a" projectName:nil managerMxid:"@owner:example.org" capability:"documentation" requestedTokens:1000 periodKey:"2026-10" account:"account_a" canOpenAgent:canOpen localBinding:{resourceId:"resource_a" knownConsumedTokens:12 retainedTokens:20 allocatedTokens:1000 usageComplete:false processState:"idle"}}] nextAfter:nil}}
let usage=fn(){H.page="web_overview" w.kind="usage" w.data={items:[] totals:{scope:"authorized_resources" complete:true byPeriod:[]} unavailable:[] nextAfter:nil managedUsage:{supported:true complete:true items:[] byProject:[]}}}
let managedRow=fn(id){return {requestId:"request_"+id agentId:"agent_"+id name:"Agent "+id resourceId:"resource_"+id projectId:"project_a" periodKey:"2026-10" knownConsumedTokens:12 retainedTokens:20 allocatedTokens:1000 usageComplete:false callCount:3}}
'''
CASES = {
    'legacy_capability_retains_terms_read_only': 'H.page="web_overview" w.kind="capability" w.data={items:[{resource:{id:"resource_a" framework:"codex" model:"model" reasoning:nil} modelTier:nil family:nil offersEditable:false roles:[{role:"documentation" qualified:true published:true count:2 budgetCapPerEngagement:500 rateCap:100}]}] nextAfter:nil} RENDER assert(find("web_capability_offer")==nil) assert(has(A.t("Saved offer: published","已保存的报价：已发布"))) assert(has(A.t("Count / tokens per engagement / daily tokens","数量 / 每次合作 token / 每日 token")+" · 2 / 500 / 100"))',
    'adopted_capability_retains_edit': 'H.page="web_overview" w.kind="capability" w.data={items:[{resource:{id:"resource_a" framework:"codex" model:"model" reasoning:nil} modelTier:nil family:nil offersEditable:true roles:[]}] nextAfter:nil} RENDER find("web_capability_offer").on_click() assert(calls[0].service=="hagency.web_catalog.offers" && calls[0].args.resourceId=="resource_a")',
    'catalog_identity': 'A.open_web_catalog("resource_a","settings",nil) assert(calls[0].args.resourceId=="resource_a") replies[0]({is_ok:true data:catalog}) assert(w.catalog==catalog)',
    'active_publication_refusal_is_recoverable': 'let acks=[] A.save=fn(){return true} A.acknowledge=fn(id){acks.push(id)} H.command={service:"hagency.resources.publish" args:{commandId:"original" resourceId:"resource_a" expectedRevision:revision published:false}} A.retry_command() replies[0]({is_ok:false error:"management_resource_in_use"}) assert(H.command==nil && !H.busy && acks.len()==1 && acks[0]=="original")',
    'catalog_stale_page': 'A.open_web_catalog("resource_a","settings",nil) H.page="overview" replies[0]({is_ok:true data:catalog}) assert(w.catalog==nil && H.page=="overview" && !H.busy)',
    'catalog_mismatch': 'A.open_web_catalog("resource_b","settings",nil) replies[0]({is_ok:true data:catalog}) assert(w.catalog==nil)',
    'rate_review_from_button': 'start() RENDER find("web_daily_review").on_click() assert(calls[0].service=="hagency.web_catalog.prepare" && calls[0].args.change.rateCapPerDay==5000 && calls[0].args.expectedRevision==revision)',
    'rate_validation': 'start() for bad in ["0","-1","1.2","9007199254740992","hello"]{w.rate=bad A.web_rate()} assert(calls.len()==0)',
    'rate_clear': 'start() w.rate="" A.web_rate() assert(calls[0].args.change.rateCapPerDay==nil)',
    'preset_original_identity': 'start() RENDER find("web_assign_preset").on_click() assert(calls[0].args.change.operation=="assign_preset" && calls[0].args.change.engagementId=="eng_original")',
    'retirement_review': 'start() RENDER find("web_resource_remove").on_click() assert(calls[0].args.change.operation=="retire")',
    'in_use_not_removable': 'start() catalog.canRetire=false RENDER assert(find("web_resource_remove")==nil)',
    'default_permissions_are_explicit': 'start() RENDER find("web_default_standard").on_click() assert(calls[0].args.change.operation=="default_policy" && !calls[0].args.change.yolo)',
    'prepare_wrong_identity': 'start() A.web_rate() prepared.resourceId="resource_b" replies[0]({is_ok:true data:prepared}) assert(w.pending==nil && w.phase=="unknown")',
    'prepare_wrong_change': 'start() A.web_rate() prepared.change.rateCapPerDay=999 replies[0]({is_ok:true data:prepared}) assert(w.pending==nil && w.phase=="unknown")',
    'object_key_order_is_not_identity': 'assert(A.web_same_change({operation:"offer" role:"coding" published:true count:1},{count:1 published:true role:"coding" operation:"offer"}))',
    'unknown_prepare_no_second_submit': 'start() A.web_rate() replies[0]({is_ok:false error:"transport_unavailable"}) A.web_rate() assert(calls.len()==1 && w.phase=="unknown")',
    'revision_change_returns_to_editor': 'start() A.web_rate() replies[0]({is_ok:false error:"management_revision_conflict"}) assert(H.page=="web_catalog" && w.pending==nil && w.phase=="")',
    'saved_id_recovered_without_apply': 'A.recover_native({operation:"web_catalog" commandId:id}) assert(calls[0].service=="hagency.web_catalog.command.get") replies[0]({is_ok:true data:prepared}) assert(w.phase=="saved" && calls.len()==1)',
    'apply_original_id_only_once': 'A.web_prepared(prepared) RENDER find("web_command_apply").on_click() A.web_apply() assert(calls.len()==1 && calls[0].args.to_json()=={commandId:id}.to_json()) replies[0]({is_ok:true data:receipt}) assert(w.phase=="recorded") A.web_apply() assert(calls.len()==1)',
    'unknown_apply_keeps_id': 'A.web_prepared(prepared) A.web_apply() replies[0]({is_ok:false error:"management_outcome_unknown"}) RENDER assert(find("web_command_apply")==nil && w.commandId==id) find("web_command_check").on_click() assert(calls[1].service=="hagency.web_catalog.command.get" && calls[1].args.commandId==id)',
    'cancel_uses_original_id': 'A.web_prepared(prepared) RENDER find("web_command_cancel").on_click() A.web_cancel() assert(calls.len()==1 && calls[0].service=="hagency.web_catalog.command.cancel" && calls[0].args.to_json()=={commandId:id}.to_json()) prepared.error="management_change_cancelled" replies[0]({is_ok:true data:prepared}) assert(w.phase=="failed" && prepared.result==nil)',
    'cancel_recovers_already_applied_result': 'A.web_prepared(prepared) A.web_cancel() prepared.result=receipt replies[0]({is_ok:true data:prepared}) assert(w.phase=="recorded" && w.pending.result==receipt)',
    'cancel_transport_failure_keeps_original': 'A.web_prepared(prepared) A.web_cancel() replies[0]({is_ok:false error:"management_outcome_unknown"}) RENDER assert(w.phase=="unknown" && w.pending.commandId==id && find("web_command_cancel")==nil) find("web_command_check").on_click() assert(calls[1].args.commandId==id)',
    'cancel_wrong_reply_is_not_accepted': 'A.web_prepared(prepared) A.web_cancel() let wrong={commandId:id resourceId:"another" expectedRevision:revision change:change result:nil error:"management_change_cancelled"} replies[0]({is_ok:true data:wrong}) assert(w.phase=="unknown" && w.pending.resourceId=="resource_a")',
    'wrong_receipt_not_done': 'A.web_prepared(prepared) A.web_apply() receipt.operation="retire" replies[0]({is_ok:true data:receipt}) assert(w.phase=="unknown" && prepared.result==nil)',
    'saved_wrong_receipt_refused': 'prepared.result=receipt receipt.resourceId="resource_b" A.web_prepared(prepared) A.web_finish() assert(w.pending==nil && w.phase=="unknown" && calls.len()==0)',
    'finish_acknowledges_original': 'prepared.result=receipt A.web_prepared(prepared) H.saved_commands=[{commandId:id},{commandId:"keep"}] A.web_finish() assert(calls[0].args.commandId==id) replies[0]({is_ok:true data:{}}) assert(H.saved_commands.len()==1 && H.saved_commands[0].commandId=="keep" && w.pending==nil && calls[1].service=="hagency.web_catalog.get")',
    'failed_ack_keeps_result': 'prepared.result=receipt A.web_prepared(prepared) A.web_finish() replies[0]({is_ok:false error:"unavailable"}) assert(w.pending.result==receipt && w.phase=="recorded")',
    'saved_pending_blocks_new_change': 'start() H.saved_commands=[{operation:"web_catalog" hasReply:false commandId:id}] A.web_rate() assert(calls.len()==0)',
    'offer_all_terms_saved': 'start() w.section="offers" w.draft={role:"coding" count:"2" budget:"1000" rate:"100" published:true qualified:true} A.web_offer_save() let c=calls[0].args.change assert(c.operation=="offer" && c.role=="coding" && c.count==2 && c.budgetCapPerEngagement==1000 && c.rateCap==100 && c.published)',
    'automatic_catalogue_offer_can_be_withdrawn': 'start() w.section="offers" A.web_offer_edit({role:"coding" count:nil budgetCapPerEngagement:nil rateCap:nil published:false catalogPublished:true qualified:true}) assert(w.draft.published) w.catalog={items:[] revision:revision} RENDER find("web_offer_publish").on_click() assert(!w.draft.published) find("web_offer_review").on_click() assert(!calls[0].args.change.published)',
    'trust_does_not_approve_project': 'start() w.trustRoom="!project:example.org" w.trustName="Team" A.web_trust_add() let c=calls[0] assert(c.service=="hagency.web_catalog.prepare" && c.args.change.operation=="trust_add" && c.args.change.projectRoomId=="!project:example.org")',
    'alerts_filter_and_paging': 'w.status="open" w.severity="critical" w.agent="resource_a" A.open_web("alerts","alert_previous") let q=calls[0].args assert(q.status=="open" && q.severity=="critical" && q.agent=="resource_a" && q.after=="alert_previous")',
    'account_resource_paging': 'A.open_web_account("account_a","resource_previous") assert(calls[0].args.to_json()=={id:"account_a" resourceAfter:"resource_previous"}.to_json())',
    'unknown_account_capacity_is_visible': 'H.page="web_overview" w.kind="accounts" w.data={items:[{account:{runtime:"codex" id:"account_a" resourceCount:0} capacity:nil resources:[] nextResourceAfter:nil declaredByPeriod:{} oversubscribedTokens:nil aggregateComplete:true}] nextAfter:nil} RENDER assert(find("web_account_open")!=nil)',
    'nullable_usage_renders': 'H.page="web_overview" w.kind="usage" w.data={items:[{resourceId:"resource_a" label:"Writer" model:"model" periodKey:"2026-10" ceilingTokens:nil drawnTokens:0 knownFreshTokens:nil cacheReadTokens:nil reservedTokens:0}] totals:{scope:"authorized_resources" complete:true byPeriod:[]} unavailable:["historical_series"] nextAfter:nil} RENDER assert(find("web_usage_resource")!=nil)',
    'terminal_has_honest_unavailable_state': 'H.page="web_overview" w.kind="terminal" w.data={id:"agent_a" supported:false text:nil} RENDER assert(has(A.t("This agent has no attached terminal pane. Its work records remain available from the agent page.","此 Agent 没有关联的终端窗格，可在 Agent 页面查看任务记录。")))',
    'empty_fleet_navigation': 'H.page="web_overview" w.kind="workforce" w.data={items:[] nextAfter:nil} RENDER assert(has(A.t("No agents in this view.","此视图暂无 Agent。")))',
    'owner_rows_hide_server_controls': 'fleet(false) RENDER assert(find("web_agent_open")==nil && find("web_agent_project")==nil && find("web_workforce_resource")!=nil && find("web_agent_terminal")!=nil) assert(has("project_a · "+A.agent_state_text("approved")))',
    'manager_rows_keep_server_controls': 'fleet(true) RENDER let selected=[] A.open_agent=fn(id){selected.push(id)} find("web_agent_open").on_click() assert(selected[0]=="request_a" && find("web_agent_project")!=nil)',
    'managed_usage_stays_separate': 'usage() w.data.managedUsage.items=[managedRow("a")] let totals=w.data.totals RENDER assert(find("web_managed_resource")!=nil && w.data.totals==totals && totals.complete && totals.byPeriod.len()==0) assert(has(A.t("Recorded / reserved / allocated tokens","已记录 / 已预留 / 已分配 token")+" · 12 / 20 / 1000")) assert(has(A.t("Accounting is incomplete; the recorded amount is a lower bound.","核算尚未完成，已记录用量仅为当前下限。")))',
    'managed_usage_page_bounded': 'usage() for i in 0..17{w.data.managedUsage.items.push(managedRow(""+i))} RENDER assert(count("web_managed_resource")==16 && find("web_managed_previous")==nil) find("web_managed_next").on_click() assert(w.managedPage==16)',
    'managed_usage_second_page': 'usage() for i in 0..17{w.data.managedUsage.items.push(managedRow(""+i))} w.managedPage=16 RENDER assert(count("web_managed_resource")==1 && find("web_managed_next")==nil) find("web_managed_previous").on_click() assert(w.managedPage==0)',
    'managed_usage_stale_page_ignored': 'usage() for i in 0..17{w.data.managedUsage.items.push(managedRow(""+i))} w.managedPage=0 RENDER w.data=nil find("web_managed_next").on_click() assert(w.managedPage==0)',
    'managed_usage_unavailable': 'usage() w.data.managedUsage.supported=false RENDER assert(has(A.t("Agent call accounting is unavailable for this connection.","此连接暂无 Agent 调用核算记录。")) && find("web_managed_resource")==nil)',
    'alerts_rendered_filter': 'H.page="web_overview" w.kind="alerts" w.data={counts:{total:0 byStatus:{}} filteredCount:0 alerts:[] nextAfter:nil} RENDER find("web_alert_agent").on_change("resource_a") find("web_alert_filter").on_click() assert(calls[0].args.agent=="resource_a")',
    'empty_capability_renders': 'H.page="web_overview" w.kind="capability" w.data={items:[] nextAfter:nil} RENDER assert(has(A.t("No resource configurations yet.","暂无资源配置。")))',
    'command_unknown_recovery_render': 'H.page="web_catalog_command" w.pending=nil w.commandId=id RENDER find("web_command_recover").on_click() assert(calls[0].service=="hagency.web_catalog.command.get" && calls[0].args.commandId==id)',
    'command_done_callback_render': 'prepared.result=receipt A.web_prepared(prepared) RENDER assert(find("web_command_apply")==nil) find("web_command_finish").on_click() assert(calls[0].args.commandId==id)',
}

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('vm', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args=parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    source=(APP/'bundle/main.splash').read_text()
    start=source.index('        if H.page=="web_overview"{')
    end=source.index('        if H.page=="templates"{',start)
    render='\nlet tree=View{\n'+source[start:end]+'\n}\nvisit(tree)\n'
    results=[]
    for language in ['en','zh-CN']:
        prefix=fixture(source+'\nmod.web_actions=A\n',language)
        for name,case in CASES.items():
            script=args.output/f'{language}-{name}.splash'
            script.write_text(prefix+SETUP+case.replace('RENDER',render)+'\ntrue\n')
            run=subprocess.run([str(args.vm.resolve()),'eval',str(script)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
            script.with_suffix('.log').write_text(run.stdout)
            results.append({'case':name,'language':language,'passed':run.returncode==0})
    (args.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n')
    print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results),'failed':[r for r in results if not r['passed']]}))
    raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__': main()
