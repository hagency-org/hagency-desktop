#!/usr/bin/env python3
"""Evaluate shipped read-only agent status tree and exact-target navigation."""
import argparse
import json
from pathlib import Path
import subprocess
from check_agent_admission import PREFIX, APP


def main():
    p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True)
    args=p.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    src=(APP/'bundle/main.splash').read_text()
    helpers=src[src.index('A.cleanup_enabled = fn('):src.index('A.setup_enabled = fn(')]
    setup_feature=src[src.index('A.setup_enabled = fn('):src.index('A.setup_allocation = fn(')]
    start=src.index('        if H.page == "agent_status" &&')
    page=src[start:src.index('        if H.page == "project_wizard"',start)]
    fixture='''
let Down=3
mod.widgets.HagencyDivider=base
mod.widgets.HagencyStatus=base
H.page="agent_status" H.agent_status=nil H.agent_id="request" H.agent_project_id="project"
let status={protocol:"hagency-management-v1" requestObservedAtMs:1000 agentRequest:agent assignment:{supported:true observedAtMs:1100 record:nil} membership:{supported:true observedAtMs:1200 record:nil} runtime:{supported:true observedAtMs:1300 record:nil}}
let assignment={id:"assignment" requestId:"request" agentId:"real-agent" state:"pending_native" generation:1 executionAvailable:false}
let membership={assignmentId:"assignment" requestId:"request" agentId:"real-agent" puppetMxid:"@_hagency_agent:test" roomId:"!project:test" state:"matrix_verified" verifiedAtMs:1200 cleanupObservedAtMs:nil blockingReason:nil workAvailable:false}
let execution={supported:true observedAtMs:1400 observedAtUtc:"1970-01-01 00:00:01 UTC" record:{assignmentId:"assignment" state:"qualified" policy:"codex_text_v1" activityExpiresAtMs:5000 activityExpiresAtUtc:"1970-01-01 00:00:05 UTC" workAvailable:false capacityReleased:false}}
let runtime={assignmentId:"assignment" state:"qualified" processState:"live" provisionExpiresAtMs:4000 workAvailable:false capacityReleased:false}
'''
    cases={
      'loading':('H.busy=true', 'assert(has(A.t("Reading current server records…","正在读取当前服务器记录…")))\nassert(button("agent_status_refresh").enabled == false)'),
      'read_failed':('', 'assert(has(A.t("Status could not be read. Refresh to try again.","未能读取状态，请刷新后重试。")))'),
      'no_record':('H.agent_status=status','assert(has(A.t("No stage result recorded","尚无此阶段的结果记录")))'),
      'policy_disabled':('H.agent_status=status status.assignment.supported=false status.membership.supported=false status.runtime.supported=false','assert(has(A.t("Not enabled on this server","此服务器尚未启用")))'),
      'qualified_not_running':('H.agent_status=status status.assignment.record=assignment status.membership.record=membership status.runtime.record=runtime','''assert(has("real-agent"))
assert(has("@_hagency_agent:test"))
assert(has("!project:test"))
assert(has(A.t("Runtime qualification recorded","已记录运行环境资格检查")))
assert(has(A.t("Qualification checks the provider process. It does not prove that the agent is currently online or performing work.","资格检查用于验证模型服务进程，并不证明 Agent 当前在线或正在执行任务。")))'''),
      'execution_legacy_absent':('H.agent_status=status','assert(has(A.t("No shared text-execution observation in this response","此响应暂无共享文本执行观测")))'),
      'execution_disabled':('H.agent_status=status status.execution=execution execution.supported=false execution.observedAtUtc=nil execution.record=nil','assert(has(A.t("Not enabled on this server","此服务器尚未启用")))\nassert(!has("1970-01-01 00:00:01 UTC"))'),
      'execution_no_record':('H.agent_status=status status.execution=execution execution.record=nil','assert(has(A.t("No stage result recorded","尚无此阶段的结果记录")))\nassert(has("1970-01-01 00:00:01 UTC"))'),
      'execution_pending':('H.agent_status=status status.execution=execution execution.record.state="pending"','assert(has(A.t("Waiting for a text-execution process","等待文本执行进程")))'),
      'execution_issued':('H.agent_status=status status.execution=execution execution.record.state="issued"','assert(has(A.t("Process start issued","已发出进程启动请求")))'),
      'execution_qualified_not_available':('H.agent_status=status status.execution=execution','assert(has(A.t("Text-execution process qualified at observation","观测时文本执行进程已通过检查")))\nassert(has(A.t("Not available when checked","检查时不可用")))\nassert(has(A.t("Codex · text only","Codex · 仅文本")))\nassert(has("1970-01-01 00:00:05 UTC"))'),
      'execution_available_observation':('H.agent_status=status status.execution=execution execution.record.workAvailable=true','assert(has(A.t("Available when checked","检查时可用")))\nassert(has(A.t("Readiness is a finite server observation and can change. It does not mean a task is running; refresh before relying on it.","就绪状态是有时限的服务器观测，可能变化，并不代表有任务正在运行，请刷新后再判断。")))'),
      'execution_unknown':('H.agent_status=status status.execution=execution execution.record.state="outcome_unknown" execution.record.activityExpiresAtUtc=nil','assert(has(A.t("Result unknown · recovery required","结果未知 · 需要恢复")))\nassert(has(A.t("No current expiry observation","暂无当前有效期观测")))'),
      'execution_stopped':('H.agent_status=status status.execution=execution execution.record.state="stopped"','assert(has(A.t("Stop or cleanup recorded","已记录停止或清理结果")))'),
      'execution_cleanup':('H.agent_status=status status.execution=execution execution.record.state="cleanup_pending"','assert(has(A.t("New work blocked · cleanup pending","已阻止新任务 · 等待清理")))'),
      'cleanup_link':('H.agent_status=status H.session.features=["agent-admission-v1","agent-cleanup-v1"]','assert(button("agent_status_removal")!=nil)\nbutton("agent_status_removal").on_click()\nassert(H.calls[0].service=="hagency.agents.removal.get" && H.calls[0].args.id=="request")\nassert(H.page=="agent_removal")\nH.calls=[]'),
      'cleanup_disabled':('H.agent_status=status','assert(button("agent_status_removal")==nil)\nassert(has(A.t("Removal status is not enabled on this server.","此服务器尚未启用移除状态查询。")))'),
      'stale':('H.agent_status=status status.membership.record=membership status.runtime.record=runtime membership.state="membership_recheck_required" runtime.state="qualification_stale"','''assert(has(A.t("Runtime qualification is stale","运行环境资格检查已失效")))
assert(has(A.t("Matrix membership needs a fresh check","Matrix 成员关系需要重新检查")))'''),
      'unknown':('H.agent_status=status status.runtime.record=runtime runtime.state="outcome_unknown"','assert(has(A.t("Result unknown · recovery required","结果未知 · 需要恢复")))'),
      'cleanup':('H.agent_status=status status.runtime.record=runtime runtime.state="cleanup_pending"','assert(has(A.t("New work blocked · cleanup pending","已阻止新任务 · 等待清理")))'),
    }
    results=[]
    for language in ['en','zh']:
        for name,(state,checks) in cases.items():
            script=PREFIX+helpers+fixture
            if language=='zh':script+='A.t=fn(en,zh){return zh}\n'
            script+=state+'\nlet root=View{'+page+'}\ncollect(root)\n'+checks+'\n'
            if name not in ['loading','read_failed']:
                script+='''assert(has(A.t("Shared-agent work records are not available in this interface yet.","此界面暂不提供共享 Agent 的任务记录。")))
assert(has(A.t("No history is not zero usage. Requested tokens are not measured consumption or remaining capacity.","暂无历史不等于零用量。申请额度不是已测量的消耗量或剩余额度。")))
'''
            script+='''for id in ["agent_submit","agent_confirm","agent_approve","agent_cancel","execute","top_up","remove"] {assert(button(id) == nil)}
H.busy=false
H.command={commandId:"original"}
button("agent_status_refresh").on_click()
assert(H.calls.len() == 1 && H.calls[0].service == "hagency.agents.status" && H.calls[0].args.id == "request")
assert(H.agent_status == nil)
H.callback({protocol:"hagency-management-v1" agentRequest:{id:"foreign"}})
assert(H.agent_status == nil && H.notice != "")
H.callback(status)
assert(H.agent_status.agentRequest.id == "request")
assert(H.command.commandId == "original")
button("agent_status_back").on_click()
assert(H.calls[1].service == "hagency.agents.get" && H.calls[1].args.id == "request")
true
'''
            path=args.output/f'{name}-{language}.splash';path.write_text(script)
            result=subprocess.run([str(args.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
            (args.output/f'{name}-{language}.log').write_text(result.stdout)
            results.append({'case':f'{name}-{language}','passed':result.returncode==0})
    feature_cases=[
      ([], "This server has not enabled project setup or agent requests in this mini app.", "此服务器尚未在小程序中启用项目设置或 Agent 申请。"),
      (["project-setup-v1"], "Open Project setup to review current prerequisites. Agent requests are not enabled on this server.", "请打开“项目设置”查看当前要求。此服务器尚未启用 Agent 申请。"),
      (["agent-admission-v1"], "Open Project agents to review current request prerequisites. Project setup is not enabled on this server.", "请打开“项目 Agent”查看当前申请要求。此服务器尚未启用项目设置。"),
      (["project-setup-v1", "agent-admission-v1"], "Open Project setup and Project agents to review this server's current prerequisites. Capacity must be confirmed separately.", "请打开“项目设置”和“项目 Agent”查看服务器当前要求。容量需另行确认。"),
    ]
    for language in ['en','zh']:
        for i,(features,en,zh) in enumerate(feature_cases):
            script=PREFIX+helpers+setup_feature
            if language=='zh':script+='A.t=fn(en,zh){return zh}\n'
            script+=f'H.session={{features:{json.dumps(features)}}}\nassert(A.project_next_steps() == {json.dumps(zh if language=="zh" else en,ensure_ascii=False)})\ntrue\n'
            name=f'project-feature-copy-{i}-{language}'
            path=args.output/f'{name}.splash';path.write_text(script)
            result=subprocess.run([str(args.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
            (args.output/f'{name}.log').write_text(result.stdout)
            results.append({'case':name,'passed':result.returncode==0})
    assert 'Allocation and agent setup are not yet available in this mini app.' not in src
    (args.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
    assert all(r['passed'] for r in results),[r for r in results if not r['passed']]
    print(str(len(results))+' agent-status subtree/navigation and project-feature copy checks passed (EN/CN)')

if __name__=='__main__':main()
