#!/usr/bin/env python3
"""Evaluate actual onboarding views and closed native handoffs; no consent mocks."""
import argparse
import json
from pathlib import Path
import subprocess
from check_agent_admission import PREFIX, APP


def main():
    p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True)
    args=p.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    source=(APP/'bundle/main.splash').read_text()
    helpers=source[source.index('A.onboarding_reason = fn('):source.index('A.project_not_applied = fn(')]
    start=source.index('        if H.page == "onboarding" {')
    page=source[start:source.index('        if H.page == "provider_accounts" ||',start)]
    fixture='''
mod.rinx_icons.connection=5
H.page="onboarding" H.session.account="@owner:test" H.onboarding=nil H.onboarding_connections=[] H.onboarding_after=nil H.onboarding_result=nil
A.connection_status=fn(row){return row.state}
A.observe_verification=fn(row,observed){H.observed=observed}
A.navigate=fn(page){H.page=page}
let native={state:"legacy" canReview:true grantReviewAvailable:false grantIntents:[]}
let original={intentId:"original-intent" contributionId:"own" resourceId:"resource-one" state:"saved"}
let connection={id:"own" label:"Owner's admitted resource" ownerMxid:"@owner:test" state:"admitted"}
'''
    cases={
      'loading':('H.busy=true','assert(button("onboarding_review") == nil)\nassert(!button("onboarding_refresh").enabled)'),
      'failed_read':('', 'assert(has(A.t("Refresh to read the current native requirements.","请刷新以读取当前原生条件。")))'),
      'legacy':('H.onboarding=native','''assert(button("onboarding_review") != nil)
button("onboarding_review").on_click()
assert(H.calls.len()==1 && H.calls[0].service=="hagency.onboarding.review" && H.calls[0].args.to_json()=={}.to_json())
H.callback({state:"review_closed"})
assert(H.calls[1].service=="hagency.onboarding.status")
assert(H.onboarding==nil && H.onboarding_result.state=="review_closed")'''),
      'empty_device':('H.onboarding=native native.setupKind="empty_device"','assert(button("onboarding_review").text==A.t("Set up this device","设置此设备"))'),
      'empty_recovery':('H.onboarding=native native.setupKind="empty_device" native.state="recovery_required"','assert(button("onboarding_review").text==A.t("Resume device setup","继续设备设置"))'),
      'empty_active_accounts':('H.onboarding=native native.setupKind="empty_device" native.state="active" native.canReview=false A.open_accounts=fn(after){H.page="provider_accounts"}','button("onboarding_accounts").on_click()\nassert(H.page=="provider_accounts")\nH.page="onboarding"'),
      'empty_history_blocked':('H.onboarding=native native.canReview=false native.reason="empty_device_history_not_empty"','assert(button("onboarding_review")==nil && has(A.onboarding_reason("empty_device_history_not_empty")))'),
      'original_recovery':('H.onboarding=native native.state="recovery_required"','assert(button("onboarding_review").text==A.t("Recover native migration","恢复原生迁移"))'),
      'unsupported_build':('H.onboarding=native native.state="unavailable" native.canReview=false native.reason="onboarding_unavailable"','''assert(button("onboarding_review")==nil)
A.review_onboarding() assert(H.calls.len()==0)
assert(has(A.onboarding_reason("onboarding_unavailable")))'''),
      'unsupported_history':('H.onboarding=native native.canReview=false native.reason="source_adoption_inventory_unsupported"','assert(button("onboarding_review")==nil && has(A.onboarding_reason("source_adoption_inventory_unsupported")))'),
      'replacement_device':('H.onboarding=native native.state="active" native.canReview=false','''assert(button("onboarding_grant_review")==nil && button("onboarding_grant_recover")==nil)
A.review_grant({intentId:"forged"}) assert(H.calls.len()==0)
assert(has(A.t("Capacity review requires this resource owner's original device and qualified native history. Inventory access on another device does not grant permission to reserve capacity.","容量核对需要资源主人的原始设备和符合条件的原生历史。在其他设备上查看清单不代表有权预留容量。")))'''),
      'original_grant':('H.onboarding=native native.state="active" native.canReview=false native.grantReviewAvailable=true native.grantIntents=[original]','''assert(has("original-intent"))
button("onboarding_grant_recover").on_click()
assert(H.calls[0].service=="hagency.grants.review" && H.calls[0].args.to_json()=={intentId:"original-intent"}.to_json())'''),
      'admitted_owner_only':('''H.onboarding=native native.state="active" native.canReview=false native.grantReviewAvailable=true
H.onboarding_connections=[connection,{id:"other" label:"Foreign" ownerMxid:"@other:test" state:"admitted"},{id:"pending" label:"Pending" ownerMxid:"@owner:test" state:"pending_admission"}]''','''assert(has("Owner's admitted resource") && !has("Foreign") && !has("Pending"))
button("onboarding_grant_review").on_click()
assert(H.calls[0].service=="hagency.grants.review" && H.calls[0].args.to_json()=={contributionId:"own"}.to_json())'''),
      'next_owner_connections':('H.onboarding=native native.state="active" native.canReview=false native.grantReviewAvailable=true H.onboarding_after="page2"','''button("onboarding_connections_more").on_click()
assert(H.calls[0].service=="hagency.connections.list" && H.calls[0].args.to_json()=={after:"page2"}.to_json())
assert(H.onboarding_connections.len()==0 && H.onboarding_after==nil)
H.callback({items:[connection] nextAfter:nil observedAtMs:99})
assert(H.onboarding_connections[0].id=="own" && H.onboarding_after==nil && H.observed==99)
A.review_grant({contributionId:H.onboarding_connections[0].id})
assert(H.calls[1].service=="hagency.grants.review" && H.calls[1].args.to_json()=={contributionId:"own"}.to_json())'''),
      'uncertain_review':('H.onboarding=native native.state="recovery_required" H.onboarding_result={state:"outcome_unknown"}','assert(has(A.t("Result unknown · recover original","结果未知 · 请恢复原始操作")))'),
      'closed_hold_not_publication':('H.onboarding=native native.state="active" native.canReview=false H.onboarding_result={state:"recorded" closure:{retainedTokens:200 knownConsumedTokens:40 unresolvedTokens:160 observedAtMs:1792340000000}}','assert(has(A.t("Previous grant closed. Retained amounts remain committed; a new grant requires a separately approved admission.","旧的发布已关闭，保留的额度仍被占用。新的发布需要重新通过加入审批。")))\nassert(!has(A.t("The local reservation is recorded. Reopen its original grant to review server publication.","本地预留已记录。请重新打开原始授权，核对服务器发布。")))'),
      'local_hold_not_publication':('H.onboarding=native native.state="active" native.canReview=false H.onboarding_result={state:"recorded"}','assert(has(A.t("The local reservation is recorded. Reopen its original grant to review server publication.","本地预留已记录。请重新打开原始授权，核对服务器发布。")))'),
    }
    result=[]
    for lang in ['en','zh']:
        for name,(setup,checks) in cases.items():
            script=PREFIX+helpers+fixture
            if lang=='zh':script+='A.t=fn(en,zh){return zh}\n'
            script+=setup+'\nlet root=View{'+page+'}\ncollect(root)\n'+checks+'\n'
            script+='''if H.onboarding != nil {
    let migration=A.t("Confirming migration stops existing local workers and preserves original identities and history. Closing a new review does not start migration; closing recovery preserves earlier progress. Interrupted confirmation must recover its original result.","确认迁移会停止原有本地工作进程，并保留原始标识与历史。关闭新核对不会开始迁移，关闭恢复核对会保留此前进度。确认中断后须恢复原始结果。")
    if H.onboarding.state == "active" {
        assert(!has(migration))
        assert(has(A.t("Device setup is complete. Register resources, then review server admission and a separate capacity grant.","设备设置已完成。请登记资源，再核对服务器准入与独立容量授权。")))
        assert(has(A.t("1 · Device resources","1 · 设备资源")))
    } else if A.onboarding_empty(H.onboarding) {
        assert(!has(migration))
        assert(has(A.t("Set up local management for your signed-in Matrix account. No provider account or shared quota is created in this step.","为当前登录的 Matrix 账号设置本机管理。此步骤不会创建模型服务账号或共享额度。")))
    } else {assert(has(migration))}
}
'''
            script+='''for id in ["migration_confirm","tokens","provider","grant_confirm","publish","execute"] {assert(button(id)==nil)}
H.busy=false H.calls=[] H.command={commandId:"keep-original"}
button("onboarding_refresh").on_click()
assert(H.calls[0].service=="hagency.onboarding.status" && H.calls[0].args.to_json()=={}.to_json() && H.onboarding==nil)
H.callback({state:"untrusted" canReview:true}) assert(H.onboarding==nil && H.notice!="")
H.callback({state:"active" canReview:false})
assert(!H.onboarding.grantReviewAvailable && H.onboarding.grantIntents.len()==0)
assert(H.command.commandId=="keep-original")
button("onboarding_back").on_click() assert(H.page=="resources")
true
'''
            case=f'{name}-{lang}';path=args.output/f'{case}.splash';path.write_text(script)
            run=subprocess.run([str(args.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
            (args.output/f'{case}.log').write_text(run.stdout);result.append({'case':case,'passed':run.returncode==0})
    historical=APP/'tests/fixtures/destructure-widget-scope.splash'
    reduced=subprocess.run([str(args.vm.resolve()),'eval',str(historical)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (args.output/'historical-destructure.log').write_text(reduced.stdout)
    (args.output/'historical-result.json').write_text(json.dumps({'fixture':str(historical),'exitCode':reduced.returncode,'expectedChildren':['legacy'],'acceptanceUsesOutsideConstructorHelper':True},indent=2)+'\n')
    (args.output/'results.json').write_text(json.dumps(result,indent=2)+'\n')
    assert all(row['passed'] for row in result),[row for row in result if not row['passed']]
    print(f'{len(result)} actual onboarding subtree/closed-handoff checks passed (EN/CN)')


if __name__=='__main__':main()
