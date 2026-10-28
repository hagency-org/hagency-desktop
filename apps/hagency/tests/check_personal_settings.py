#!/usr/bin/env python3
"""Pinned VM tests of shipped personal settings and original-command callbacks.

Uses inert widget objects; this is source qualification, not native pixels or
execution-device policy acceptance. The full shipped-root check is separate.
"""
import argparse,json,subprocess
from pathlib import Path
from check_agent_admission import APP,PREFIX

def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 s=(APP/'bundle/main.splash').read_text()
 helpers=s[s.index('A.personal_reason = fn('):s.index('A.resource_period = fn(')]
 # Shared recovery keeps the shipped task/name classifiers, rather than a fake
 # implementation that could erase another operation's saved command.
 helpers+='\n'+'\n'.join(line for line in s.splitlines() if line.startswith(('A.name_not_applied = fn(', 'A.is_task_operation=fn(', 'A.topup_service=fn(', 'A.topup_operation=fn(', 'A.topup_not_applied=fn(')))+'\n'
 pages=s[s.index('        if H.page == "personal_agents" {'):s.index('        if H.page == "projects" && A.projects_enabled() {')]
 fixture='''let Down=3
mod.rinx_icons.device=5
H.personal_config={choices:nil draft:nil offset:0 epoch:0 result:nil}
A.is_template_operation=fn(operation){return false}
A.is_graph_operation=fn(operation){return false}
H.personal_list=nil H.personal_after=nil H.personal_page=1 H.personal_detail=nil H.personal_id="agent_a" H.personal_mode="detail" H.personal_draft=nil H.personal_result=nil H.personal_observed=nil H.page="personal_agent"
A.navigate=fn(page){H.page=page}
A.reasoning_label=fn(value){return value}
A.cleanup_enabled=fn(){return false}
A.agent_not_applied=fn(service,code){return false}
let row={id:"agent_a" displayName:"Personal A" state:"suspended" generation:3 executionDeviceId:"device_a"}
let observation={policyRevision:4 budget:{kind:"tokens" tokens:1000 period:"utc_month"} requests:"ask_owner" highRiskTools:"rules" toolRuleCount:2 directoryRuleCount:1 knownConsumedTokens:17 heldTokens:50 model:{model:"codex-model" reasoningEffort:"high" webSearch:false workspaceConfigured:true}}
let detail={protocol:"hagency-personal-settings-v1" agent:row settings:{supported:true online:true cached:true observedAtUtc:"2026-10-16 00:00:00 UTC" observation:observation}}
let listing={protocol:"hagency-personal-settings-v1" items:[row] nextAfter:nil}
let saved={protocol:"hagency-personal-settings-v1" agentId:"agent_a" commandId:"original" policyRevision:5 saved:true}
'''
 cases={
  'empty':('H.page="personal_agents" H.personal_list=listing listing.items=[]','assert(has(A.t("No personal agents on this page","此页暂无个人 Agent")))'),
  'list':('H.page="personal_agents" H.personal_list=listing','assert(has("Personal A") && has("agent_a"))\nbutton("personal_open").on_click()\nassert(H.calls[0].service=="hagency.personal_agents.get" && H.calls[0].args.id=="agent_a")'),
  'paging':('H.page="personal_agents" H.personal_list=listing listing.nextAfter="agent_a"','button("personal_next").on_click()\nassert(H.personal_list==nil && H.personal_page==2 && H.calls[0].args.after=="agent_a" && H.calls[0].args.limit==16)'),
  'unsupported':('H.personal_detail=detail detail.settings.supported=false detail.settings.observation=nil','assert(button("personal_edit")==nil)\nassert(has(A.personal_reason("configuration_unsupported")))\nA.edit_personal_policy()\nassert(H.personal_draft==nil && H.calls.len()==0)'),
  'no_observation':('H.personal_detail=detail detail.settings.observation=nil','assert(button("personal_edit")==nil)\nassert(has(A.t("The execution device has not reported its policy yet.","执行设备尚未上报策略。")))\nassert(!has("0 token"))'),
  'offline_cache':('H.personal_detail=detail detail.settings.online=false','assert(has("17 / 50 token"))\nassert(has(A.t("Cached observation; this is not a live agent status.","缓存观测，不代表 Agent 的实时运行状态。")))\nassert(button("personal_edit")==nil)'),
  'model_safe':('H.personal_detail=detail','assert(has("codex-model"))\nassert(has(A.t("Configured on execution device","已在执行设备配置")))\nassert(has(A.t("Custom tool / directory rules: ","自定义工具 / 目录规则：")+"2 / 1"))'),
  'rules_unchanged':('H.personal_detail=detail','button("personal_edit").on_click()\nassert(H.personal_mode=="edit" && H.personal_draft.highRiskTools=="rules")\nassert(!A.personal_valid())\nA.review_personal_policy()\nassert(H.personal_mode=="edit" && H.calls.len()==0 && observation.highRiskTools=="rules")'),
  'edit_cancel':('H.personal_detail=detail A.edit_personal_policy()','assert(button("personal_tokens")!=nil && button("personal_review")!=nil && button("personal_submit")==nil)\nbutton("personal_edit_cancel").on_click()\nassert(H.personal_mode=="detail" && H.personal_draft==nil && H.calls.len()==0 && observation.highRiskTools=="rules")'),
  'review_replacement':('H.personal_detail=detail A.edit_personal_policy() H.personal_draft.highRiskTools="deny" H.personal_draft.tokens="250" A.review_personal_policy()','assert(has(A.t("This replaces the existing custom tool and directory rules.","此修改将替换现有的自定义工具与目录规则。")))\nassert(button("personal_submit")!=nil && H.calls.len()==0)\nbutton("personal_submit").on_click()\nlet call=H.calls[0]\nassert(call.service=="hagency.personal_agents.policy.save" && call.args.agentId=="agent_a" && call.args.expectedGeneration==3 && call.args.expectedRevision==4)\nassert(call.args.policy.budget.tokens==250 && call.args.policy.budget.period=="utc_month" && call.args.policy.highRiskTools=="deny")'),
  'review_cancel':('H.personal_detail=detail A.edit_personal_policy() H.personal_draft.highRiskTools="ask_owner" A.review_personal_policy()','button("personal_edit_cancel").on_click()\nassert(H.calls.len()==0 && H.personal_mode=="detail" && observation.highRiskTools=="rules")'),
  'changed_generation':('H.personal_detail=detail A.edit_personal_policy() H.personal_draft.highRiskTools="deny" A.review_personal_policy() row.generation=4','A.submit_personal_policy()\nassert(H.calls.len()==0)'),
  'pending_original':('H.personal_detail=detail H.command={args:{commandId:"original"}}','assert(button("personal_edit")==nil)\nA.edit_personal_policy()\nassert(H.personal_draft==nil && H.command.args.commandId=="original" && H.calls.len()==0)'),
  'budget_validation':('H.personal_detail=detail A.edit_personal_policy() H.personal_draft.highRiskTools="deny"','for value in ["","bad","-1","0.5","9007199254740992"] {H.personal_draft.tokens=value assert(!A.personal_valid())}\nH.personal_draft.tokens="0" assert(A.personal_valid())\nH.personal_draft.tokens="9007199254740991" assert(A.personal_valid())'),
  'refresh':('H.personal_detail=detail','button("personal_reload").on_click()\nassert(H.personal_detail==nil && H.personal_draft==nil && H.calls[0].args.id=="agent_a")\nH.callback({protocol:"hagency-personal-settings-v1" agent:{id:"foreign"}})\nassert(H.personal_detail==nil)\nH.callback(detail)\nassert(H.personal_detail.agent.id=="agent_a")'),
  'observe_review_cancel':('H.personal_detail=detail detail.settings.observation=nil','button("personal_observe").on_click()\nassert(H.personal_mode=="observe_review" && H.calls.len()==0)\nH.personal_mode="detail" H.personal_draft=nil\nassert(H.calls.len()==0)'),
  'observe_review':('H.personal_detail=detail detail.settings.observation=nil A.review_personal_observation()','assert(button("personal_observe_submit")!=nil && button("personal_submit")==nil && H.calls.len()==0)\nbutton("personal_observe_submit").on_click()\nassert(H.calls[0].service=="hagency.personal_agents.observe" && H.calls[0].args.agentId=="agent_a" && H.calls[0].args.expectedGeneration==3)'),
  'observe_cancel':('H.personal_detail=detail A.review_personal_observation()','button("personal_observe_cancel").on_click()\nassert(H.personal_mode=="detail" && H.personal_draft==nil && H.calls.len()==0)'),
  'observe_generation_changed':('H.personal_detail=detail A.review_personal_observation() row.generation=4','A.submit_personal_observation()\nassert(H.calls.len()==0)'),
  'observe_offline':('H.personal_detail=detail detail.settings.online=false','assert(button("personal_observe")==nil)\nA.review_personal_observation()\nassert(H.calls.len()==0 && H.personal_draft==nil)'),
  'observe_recorded':('H.personal_detail=detail H.personal_observed={agentId:"agent_a" observed:true}','assert(has(A.t("Device observation recorded","设备观测已记录")))'),
  'saved_policy_invalidates_old_observation_notice':('H.personal_observed={agentId:"agent_a" observed:true} A.personal_saved(saved) H.personal_detail=detail detail.settings.observation=nil','assert(!has(A.t("Device observation recorded","设备观测已记录")))\nassert(has(A.t("Saved revision: ","已保存版本：")+"5"))\nassert(has(A.t("The execution device has not reported its policy yet.","执行设备尚未上报策略。")))'),
  'historical_result':('H.personal_result=saved H.personal_detail=detail','assert(has(A.t("Saved revision: ","已保存版本：")+"5"))\nassert(has(A.t("This confirms the original request. Use Read current settings to obtain the device’s updated policy.","这里确认原始请求已完成。请使用“读取当前设置”获取设备更新后的策略。")))'),
 }
 results=[]
 def run(name,script):
  path=a.output/(name+'.splash');path.write_text(script+'\ntrue\n');r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(a.output/(name+'.log')).write_text(r.stdout);results.append({'case':name,'passed':r.returncode==0})
 for lang in ['en','zh']:
  for name,(state,checks) in cases.items():run(name+'-'+lang,PREFIX+helpers+fixture+('A.t=fn(en,zh){return zh}\n' if lang=='zh' else '')+state+'\nlet root=View{'+pages+'}\ncollect(root)\n'+checks)
 retry=s[s.index('A.retry_command = fn('):s.index('A.enroll = fn(')]
 recover=s[s.index('A.recover_native = fn('):s.index('A.saved_label = fn(')]
 for outcome in ['success','unknown','unsupported_after_send','conflict']:
  script=PREFIX+helpers+fixture+retry+'''H.personal_detail=detail H.personal_draft={tokens:"250"}
H.command={service:"hagency.personal_agents.policy.save" args:{commandId:"original" agentId:"agent_a" expectedGeneration:3 expectedRevision:4}}
A.retry_command()
assert(H.calls[0].args.commandId=="original")
'''
  script+= {'success':'H.host_callback({is_ok:true data:saved})\nassert(H.command==nil && H.ack=="original" && H.personal_result.policyRevision==5 && H.calls[1].service=="hagency.personal_agents.get")','unsupported_after_send':'H.host_callback({is_ok:false error:"configuration_unsupported"})\nassert(H.command.args.commandId=="original" && H.personal_detail==nil)', 'unknown':'H.host_callback({is_ok:false error:"management_outcome_unknown"})\nassert(H.command.args.commandId=="original" && H.personal_detail==nil && H.personal_draft==nil)','conflict':'H.host_callback({is_ok:false error:"local_revision_conflict"})\nassert(H.command==nil && H.ack=="original" && H.personal_detail==nil)'}[outcome]
  run('saved-'+outcome,script)
 run('native-recovery',PREFIX+helpers+fixture+recover+'''A.recover_native({commandId:"original" operation:"personal_agent_policy"})
assert(H.calls[0].service=="hagency.intent.retry" && H.calls[0].args.commandId=="original")
H.callback({operation:"personal_agent_policy" data:saved})
assert(H.personal_result.policyRevision==5 && H.page=="personal_agent" && H.calls[1].service=="hagency.personal_agents.get" && H.ack=="original")''')
 run('observation-unknown-classification',PREFIX+helpers+fixture+retry+'''H.personal_detail=detail H.personal_draft={agentId:"agent_a"}
H.command={service:"hagency.personal_agents.observe" args:{commandId:"original_observation" agentId:"agent_a" expectedGeneration:3}}
A.retry_command()
H.host_callback({is_ok:false error:"local_revision_conflict"})
assert(H.command.args.commandId=="original_observation" && H.personal_detail==nil)
assert(!A.personal_not_applied("hagency.personal_agents.observe","configuration_unsupported"))
assert(A.personal_not_applied("hagency.personal_agents.policy.save","local_revision_conflict"))''')
 run('native-observation-recovery',PREFIX+helpers+fixture+recover+'''A.recover_native({commandId:"original_observation" operation:"personal_agent_observation"})
assert(H.calls[0].service=="hagency.intent.retry" && H.calls[0].args.commandId=="original_observation")
H.callback({operation:"personal_agent_observation" data:{agentId:"agent_a" observed:true}})
assert(H.personal_observed.observed && H.page=="personal_agent" && H.calls[1].service=="hagency.personal_agents.get" && H.ack=="original_observation")''')
 run('personal-unknown-does-not-inherit-resource-failures',PREFIX+helpers+fixture+retry+'''for code in ["invalid_management_arguments","management_revision_changed","management_resource_name_changed"] {
H.command={service:"hagency.personal_agents.policy.save" args:{commandId:"original" agentId:"agent_a" expectedGeneration:3 expectedRevision:4}}
A.retry_command()
H.host_callback({is_ok:false error:code})
assert(H.command.args.commandId=="original")
}''')
 (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
