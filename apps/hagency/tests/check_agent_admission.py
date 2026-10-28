#!/usr/bin/env python3
"""Evaluate shipped agent-admission views and original-command handlers in pinned VM.

No native pixels, Matrix rooms, approvals or runtime execution are simulated as
accepted production effects. Inert widget objects expose exact callbacks/children.
"""
import argparse
import json
from pathlib import Path
import subprocess

APP=Path(__file__).resolve().parents[1]
PREFIX='''use mod.std.assert
use mod.math.*
let base={text:"" draw_icon:{} draw_bg:{} draw_text:{}}
let Fill=1
let Fit=2
let Down=3
let Flow={Right:{}}
let Walk={}
let View=base
mod.widgets={RINX_SELECTED:1 RINX_SURFACE:2 RINX_ACCENT:3 RINX_INK:4}
mod.rinx_icons={back:1 refresh:2 add:3 resources:4}
'''
for widget in ['HagencyCard','HagencyDetailCard','HagencySection','HagencyActions','HagencyTitle','HagencyCopy','HagencyHint','HagencyFieldLabel','HagencyDetailRow','HagencySectionTitle','HagencyDivider','HagencyStatus','HagencyEyebrow','HagencyInput','RinxButton','HagencyButton','HagencyCardHeading','HagencyBadge','HagencyGlyph','RinxPrimaryButton','RinxKeyPrimaryButton']:
    PREFIX+=f'mod.widgets.{widget}=base\n'
for widget in ['HagencyNumberInput','HagencyFormGroup','HagencyChoice','HagencyDisclosure','HagencyToggle','HagencyProgress','LoadingSpinner']:
    PREFIX+=f'mod.widgets.{widget}=base\n'
PREFIX+='mod.rinx_icons.expand=1 mod.rinx_icons.collapse=1\n'
PREFIX+='''let H={ux:{inboxReady:true agentInboxReady:true agentListReady:true inboxAgentReady:true inboxRequestReady:true agentCursor:nil inboxCursor:nil} needs_signin:false session:{account:"@manager:test" features:["agent-admission-v1"]} page:"agents" busy:false command:nil notice:"" agents:[] agent_after:nil agent_readiness:nil agent_selected:nil agent_id:"" agent_project_id:"project" agent_confirmation:"" agent_review:false agent_draft:{projectId:"" name:"" description:"" capability:"" tokens:"1000"} saved_commands:[] calls:[] inbox:[] inbox_after:nil needs_action:true}
let A={}
A.t=fn(en,zh){return en}
A.redraw=fn(){}
A.must_reopen=fn(){return false}
A.personal_config_service=fn(service){return false}
A.web_connection_service=fn(service){return false}
A.save=fn(){return true}
A.setup_reason=fn(code){return code}
A.error_text=fn(code){return code}
A.project_not_applied=fn(service,code){return false}
A.allocation_not_applied=fn(service,code){return false}
A.allocations_enabled=fn(){return false}
A.request=fn(service,args,done){H.calls.push({service:service args:args}) H.callback=done}
A.acknowledge=fn(id){H.ack=id}
A.submit=fn(service,args){H.calls.push({service:service args:args})}
A.amount=fn(value){if value == nil {return A.t("Unknown","未知")} return ""+value}
A.capability_label=fn(value){return value}
let host={}
host.request=fn(service,args,done){H.calls.push({service:service args:args}) H.host_callback=done}
let texts=[]
let buttons={}
let collect=fn(value){
    if !value.is_object(){return}
    let {text=""}=value
    if text != "" {texts.push(text)}
    for i key child in value {
        buttons[key.to_string()]=child
        collect(child)
    }
}
let has=fn(text){for value in texts {if value == text {return true}} return false}
let button=fn(id){let found=nil for key value in buttons {if key == id {found=value}} return found}
let readiness={projectId:"project" projectName:"Project Alpha" managerMxid:"@manager:test" administratorMxid:"@manager:test" canViewRequests:true canRequest:true blockingReason:nil requestContext:{setupId:"setup" setupRevision:3 setupBindingRevision:2 authorityRevision:4 allocationId:"allocation" allocationRevision:5 allocationReceiptDigest:"digest" requestedTokenLimit:700 nativeAvailableTokens:nil periodKey:"2026-10" periodStartMs:1 periodEndMs:2 capabilities:["coding","documentation"] toolModes:["none"]}}
let agent={id:"request" projectId:"project" projectName:"Project Alpha" managerMxid:"@manager:test" administratorMxid:"@manager:test" definition:{name:"Writer" capability:"documentation" description:"Draft articles" toolMode:"none"} requestedTokens:100 periodKey:"2026-10" state:"pending_approval" revision:6 authorityRevision:4 canApprove:false canReject:false canCancel:true blockingReason:nil executionAvailable:false}
'''

def main():
    p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);args=p.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    src=(APP/'bundle/main.splash').read_text()
    helpers=src[src.index('A.cleanup_enabled = fn('):src.index('A.setup_enabled = fn(')]
    helpers+='\n'+'\n'.join(line for line in src.splitlines() if line.startswith(('A.name_not_applied = fn(', 'A.is_task_operation=fn(')))+'\n'
    retry=src[src.index('A.retry_command = fn('):src.index('A.enroll = fn(')]
    # Actual production handler ends immediately before A.enroll.
    pages=src[src.index('        if H.page == "agents" && A.agents_enabled()'):src.index('        if H.page == "project_wizard" && A.projects_enabled()')]
    inbox_start=src.index('        if H.page == "inbox" {')
    pages+=src[inbox_start:src.index('        if H.page == "wizard" {',inbox_start)]
    cases={
      'missing_feature':('H.session={} H.agent_readiness=readiness','assert(root.len() == 0)\nassert(!A.agents_enabled())\nA.open_project_agents("project")\nassert(H.calls.len() == 0)'),
      'manager_ready':('H.agent_readiness=readiness','''assert(has("Project Alpha"))
assert(button("agent_new") != nil)
button("agent_new").on_click()
assert(H.page == "agent_wizard")
assert(H.calls.len() == 0)
assert(H.agent_draft.projectId == "project")'''),
      'missing_capacity':('H.agent_readiness=readiness readiness.requestContext=nil readiness.canRequest=false readiness.blockingReason="live_reservation_required"','''assert(button("agent_new") == nil)
assert(has("live_reservation_required"))
A.begin_agent()
assert(H.page == "agents" && H.calls.len() == 0)'''),
      'coordinator':('H.agent_readiness=readiness readiness.canViewRequests=false readiness.canRequest=false readiness.requestContext=nil readiness.blockingReason="project_manager_required"','''assert(button("agent_new") == nil)
A.open_project_agents("project")
H.callback({readiness:readiness})
assert(H.calls.len() == 1 && H.calls[0].service == "hagency.agents.readiness")
assert(!has(A.t("No agent requests on this page","此页暂无 Agent 申请")))
assert(has(A.t("Project information only","仅查看项目信息")))
assert(has(A.t("Project authority","项目权限")))
assert(has(A.t("The project manager manages agent requests and approvals. Return to the project to view resource allocation and responsibilities.","项目经理管理 Agent 申请与审批。可返回项目查看资源分配及各方职责。")))'''),
      'admin_list':('H.agent_readiness=readiness readiness.canRequest=false readiness.blockingReason="project_manager_required" H.agents=[agent]','''assert(button("agent_new") == nil)
assert(has("Writer"))
button("agent_open").on_click()
assert(H.calls.len() == 1 && H.calls[0].service == "hagency.agents.get" && H.calls[0].args.id == "request")
H.callback({agentRequest:agent})
assert(H.agent_selected.id == "request")'''),
      'wizard_review':('H.page="agent_wizard" H.agent_readiness=readiness H.agent_draft={projectId:"project" name:"Writer" description:"Draft articles" capability:"documentation" tokens:"100"}','''assert(button("agent_review") != nil)
assert(button("agent_submit") == nil)
button("agent_review").on_click()
assert(H.agent_review && H.calls.len() == 0)
A.submit_agent()
assert(H.calls.len() == 1)
let call=H.calls[0]
assert(call.service == "hagency.agents.request" && call.args.projectId == "project")
assert(call.args.request.expectedSetupRevision == 3 && call.args.request.expectedSetupBindingRevision == 2 && call.args.request.expectedAuthorityRevision == 4 && call.args.request.expectedAllocationRevision == 5)
assert(call.args.request.definition.toolMode == "none" && call.args.request.requestedTokens == 100)'''),
      'wizard_confirm':('H.page="agent_wizard" H.agent_readiness=readiness H.agent_review=true H.agent_draft={projectId:"project" name:"Writer" description:"Draft articles" capability:"documentation" tokens:"100"}','''assert(button("agent_submit") != nil && button("agent_review") == nil)
assert(!has("@admin:test"))
button("agent_edit").on_click()
assert(!H.agent_review && H.calls.len() == 0)'''),
      'manager_detail':('H.page="agent_action" H.agent_selected=agent H.agent_id="request"','''assert(button("agent_approve") == nil && button("agent_reject") == nil && button("agent_cancel") != nil)
button("agent_cancel").on_click()
assert(H.agent_confirmation == "cancel" && H.calls.len() == 0)
A.decide_agent(true)
assert(H.calls.len() == 0)'''),
      'admin_detail':('H.page="agent_action" H.agent_selected=agent H.agent_id="request" agent.canApprove=true agent.canReject=true agent.canCancel=false','''assert(button("agent_approve") != nil && button("agent_reject") != nil && button("agent_cancel") == nil)
button("agent_approve").on_click()
assert(H.agent_confirmation == "approve" && H.calls.len() == 0)'''),
      'admin_confirm':('H.page="agent_action" H.agent_selected=agent H.agent_id="request" agent.canApprove=true agent.canReject=true agent.canCancel=false H.agent_confirmation="approve"','''assert(button("agent_confirm") != nil)
button("agent_confirm").on_click()
assert(H.calls.len() == 1 && H.calls[0].service == "hagency.agents.decide")
assert(H.calls[0].args.requestId == "request" && H.calls[0].args.expectedRevision == 6 && H.calls[0].args.expectedAuthorityRevision == 4 && H.calls[0].args.approve)'''),
      'approved_pending':('H.page="agent_action" H.agent_selected=agent H.agent_id="request" agent.state="approved_waiting_executor" agent.canCancel=false','''assert(has(A.t("Approved · executor setup pending","已批准 · 等待执行设备设置")))
assert(button("agent_approve") == nil && button("agent_reject") == nil && button("agent_cancel") == nil)'''),
      'expired_legacy_name':('H.page="agent_action" agent={id:"request" projectId:"project" managerMxid:"@manager:test" administratorMxid:"@manager:test" definition:agent.definition requestedTokens:100 periodKey:"2026-10" state:"expired" revision:7 authorityRevision:4 canApprove:false canReject:false canCancel:false blockingReason:"agent_request_expired" executionAvailable:false} H.agent_selected=agent','''assert(has(A.t("Agent request expired","Agent 申请已过期")))
assert(button("agent_approve") == nil && button("agent_cancel") == nil)'''),
      'draft_validation':('H.agent_readiness=readiness','''assert(!A.restore_agent_draft({}))
assert(!A.restore_agent_draft({projectId:"project" name:"x" description:"x" capability:"coding" tokens:100}))
assert(A.restore_agent_draft({projectId:"project" name:"Writer" description:"Draft articles" capability:"documentation" tokens:"701"}))
A.review_agent()
assert(!H.agent_review && H.notice != "")
H.agent_draft.tokens="100.5" A.review_agent() assert(!H.agent_review)
H.agent_draft.tokens="100" A.review_agent() assert(H.agent_review)
A.clear_agent_draft(agent)
assert(H.agent_draft.projectId == "")'''),
      'unknown_retains_original':('H.agent_selected=agent H.agent_readiness=readiness H.agent_id="request" H.command={service:"hagency.agents.decide" args:{commandId:"original" requestId:"request" expectedRevision:6 expectedAuthorityRevision:4 approve:true}}','''A.retry_command()
assert(H.calls.len() == 1)
H.host_callback({is_ok:false error:"management_outcome_unknown"})
assert(H.command.args.commandId == "original")
assert(H.agent_selected == nil && H.agent_readiness == nil && H.agent_id == "request")
let {ack=nil}=H assert(ack == nil)
A.retry_command()
assert(H.calls.len() == 2 && H.calls[1].args.commandId == "original")'''),
      'definitive_rejection':('H.agent_selected=agent H.agent_readiness=readiness H.command={service:"hagency.agents.request" args:{commandId:"original"}}','''A.retry_command()
H.host_callback({is_ok:false error:"agent_offer_unavailable"})
assert(H.command == nil && H.ack == "original")
assert(H.agent_readiness == nil && H.agent_selected == nil)'''),
      'inbox_pending':('H.page="inbox" H.agents=[agent]','''assert(has("Writer") && has("Project Alpha"))
assert(!has(A.t("You’re up to date","暂无待处理事项")))
button("agent_inbox_open").on_click()
assert(H.calls.len() == 1 && H.calls[0].service == "hagency.agents.get")'''),
      'inbox_legacy':('H.page="inbox" H.session={} H.agents=[]','''assert(button("agent_inbox_open") == nil)
assert(has(A.t("You’re up to date","暂无待处理事项")))'''),
      'latest_result':('H.command={service:"hagency.agents.decide" args:{commandId:"original"}} agent.state="cancelled" agent.canCancel=false','''A.retry_command()
H.host_callback({is_ok:true data:{agentRequest:agent}})
assert(H.command == nil && H.ack == "original")
assert(H.agent_selected.state == "cancelled" && H.agent_id == "request")'''),
    }
    results=[]
    for language in ['en','zh']:
      for case,(state,checks) in cases.items():
        script=PREFIX+helpers+'\n'+retry+'\n'
        if language=='zh':script+='A.t=fn(en,zh){return zh}\n'
        script+=state+'\nlet root=View{\n'+pages+'}\ncollect(root)\n'+checks+'\ntrue\n'
        path=args.output/f'{case}-{language}.splash';path.write_text(script)
        r=subprocess.run([str(args.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        (args.output/f'{case}-{language}.log').write_text(r.stdout)
        results.append({'case':f'{case}-{language}','passed':r.returncode==0})
    (args.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
    failures=[r['case'] for r in results if not r['passed']]
    assert not failures,failures
    print(f'{len(results)} actual agent-admission subtree/handler checks passed (EN/CN)')

if __name__=='__main__':main()
