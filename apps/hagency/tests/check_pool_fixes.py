#!/usr/bin/env python3
"""A pool whose running Agent holds the whole monthly maximum.

Reproduces what an owner hit on a published pool (9 M a month, one running Agent holding all
9 M): the per-request floor came from an estimate that missed the Agent, so the page offered
1 M - 9 M; the maximum's lock reason hid the amendment's own; and nothing said the month was
all given out.
"""
import argparse, json, subprocess
from pathlib import Path
from check_full_bundle import APP, fixture
SETUP='''
A.redraw=fn(){} A.save=fn(){return true}
// No device storage here: the per-pool drafts start empty and stay in memory.
let fs={exists:fn(path){return false} read:fn(path){return "{}"} write:fn(path,data){return true}}
H.pool_drafts={account:"@owner:test" quota:[]}
H.ready=true H.busy=false H.command=nil H.calls=[] H.callbacks=[] H.entry_role="owner" H.page="resource_detail"
H.session={account:"@owner:test" device:"device" canAdmitResources:false features:["resource-pool-agents-v1","pool-grant-succession-v1"]}
host.request=fn(service,args,done){H.calls.push({service:service args:args}) H.callbacks.push(done)}
let reply=fn(index,data){H.callbacks[index]({is_ok:true data:data})}
let find=fn(service){let at={i:-1} for index call in H.calls {if call.service==service && at.i<0 {at.i=index}} return at.i}
let now=1793334588000
let pool=fn(id,name,ceiling,remaining,committed,spent,quota){return {id:id model:"model-"+id framework:"codex" reasoning:"medium" published:false capabilities:["coding"] localName:{name:name revision:1} budget:{remainingTokens:remaining reserved:committed pool:{ceiling:ceiling committed:committed period:"monthly"} seat:{quota:quota}} usage:{spent:spent spend_period_key:"2026-10"}}}
let main=pool("resource_main","Research",9000000,0,9000000,350000,50000000)
let spare=pool("resource_spare","Spare",20000000,20000000,0,0,50000000)
let conn={id:"con_main" label:"Research" resourceId:"resource_main" ownerMxid:"@owner:test" coordinatorMxid:"@owner:test" executorDeviceId:"device" state:"admitted"}
let spare_conn={id:"con_spare" label:"Spare" resourceId:"resource_spare" ownerMxid:"@owner:test" coordinatorMxid:"@owner:test" executorDeviceId:"device" state:"admitted"}
let grant={id:"con_main" ownerMxid:"@owner:test" publishedTokens:9000000 grantRevision:1 currentPeriodEndMs:1793491200000}
let detail=fn(row,amend){return {protocol:"hagency-management-v1" resource:row editing:{allowed:false reason:"previously_offered"} choices:[] ceiling:{tokens:row.budget.pool.ceiling period:"monthly"} ceilingAmendment:amend observedAtMs:now}}
let amendment={allowed:true reason:nil sequence:1 minimumTokens:9000000 perRequestTokens:9000000}
let open=fn(row){
    H.resources=[main,spare]
    H.resource_detail=detail(row,amendment)
    H.pool_info={rid:row.id conn:conn connState:"ready" grant:grant grantState:"ready" board:{grantReviewAvailable:true} boardState:"ready" requests:0 alerts:0 tools:0 stopped:0 workspaces:0 agents:1 inuse:1 now:now using:[] floor:nil}
    A.pool_edit_sync()
}
let floor_reply={contributionId:"con_main" heldTokens:9000000 heldAgents:1 perRequestTokens:9000000 mostTokens:9000000 periodEndMs:1793491200000}
'''
CASES={
'the_floor_is_the_windows_figure_not_an_estimate': '''
open(main)
let v=A.pool_view(H.resource_detail)
assert(v.quotaMode=="amend" && v.publishedTokens==9000000)
// The running Agent isn't among the requests the page counts: the estimate knows no floor,
// which is how the page offered 1 M - 9 M.
assert(A.pool_quota_floor(v)==nil && A.pool_quota_min(v)==1 && A.pool_quota_fixed(v)==nil)
// The window's own record says what running Agents hold.
H.pool_info.floor=A.pool_floor_read({is_ok:true data:floor_reply},"con_main")
assert(A.pool_quota_floor(v)==9000000 && A.pool_quota_min(v)==9000000)
assert(A.pool_floor_text(v)==A.t("No less than the 9 M running Agents hold","不能低于正在运行的 Agent 已拿到的 9 M"))
// A reply for another pool, or a refusal, is no floor.
assert(A.pool_floor_read({is_ok:true data:{contributionId:"con_other" heldTokens:1}},"con_main")==nil)
assert(A.pool_floor_read({is_ok:false error:"management_grant_succession_required"},"con_main")==nil)
''',
'the_page_reads_the_floor_of_a_published_month': '''
H.resources=[main] H.resource_detail=detail(main,amendment) H.pool_info=nil
A.pool_info_load("resource_main")
assert(H.calls[0].service=="hagency.connections.list")
reply(0,{items:[conn] nextAfter:nil observedAtMs:now})
assert(H.calls[1].service=="hagency.pools.list")
reply(1,{items:[grant] nextAfter:nil observedAtMs:now})
let guard={n:0}
while find("hagency.grants.floor")<0 && guard.n<8 {reply(H.calls.len()-1,{items:[] nextAfter:nil}) guard.n=guard.n+1}
let at=find("hagency.grants.floor")
assert(at>=0 && H.calls[at].args.contributionId=="con_main")
reply(at,floor_reply)
assert(H.pool_info.floor.held==9000000 && H.pool_info.floor.most==9000000 && H.pool_info.floor.agents==1)
''',
'one_possible_amount_is_shown_fixed_with_why': '''
open(main)
H.pool_info.floor=A.pool_floor_read({is_ok:true data:floor_reply},"con_main")
let v=A.pool_view(H.resource_detail)
assert(A.pool_quota_fixed(v)==9000000)
let line=A.pool_quota_fixed_line(v)
assert(line.search("9 M")>=0 && line.search(A.t("Raise the monthly maximum","调高上面的每月上限"))>=0)
// With room above the floor the slider stays.
H.pool_info.floor=A.pool_floor_read({is_ok:true data:{contributionId:"con_main" heldTokens:9000000 heldAgents:1 mostTokens:20000000}},"con_main")
assert(A.pool_quota_fixed(v)==nil && A.pool_quota_min(v)==9000000)
''',
'a_month_all_given_out_says_so_and_how_to_give_more': '''
open(main)
H.pool_meta={epoch:0 conns:[conn,spare_conn] connsError:false grants:[grant,{id:"con_spare" ownerMxid:"@owner:test" publishedTokens:nil grantRevision:nil}] grantsError:false requests:[] counts:[] now:now board:nil boardState:"ready"}
H.pool_info.floor=A.pool_floor_read({is_ok:true data:floor_reply},"con_main")
let v=A.pool_view(H.resource_detail)
assert(A.pool_full(v) && v.status.key=="full")
assert(A.pool_full_title(v)==A.t("October’s 9 M is all given out","10 月的 9 M 已全部分出"))
assert(A.pool_full_why(v).search("9 M")>=0)
// Ways forward: a higher maximum here, or Spare, admitted and not published this month.
assert(A.pool_can_raise(v) && A.pool_full_no_raise(v)=="")
let others=A.pool_full_others(v)
assert(others.len()==1 && others[0].rid=="resource_spare" && others[0].kind=="publish")
assert(A.pool_full_other_label(others[0])==A.t("Publish ","发布 ")+"Spare")
// Raising preselects room for one more request at this month's limit.
H.pool_raise={rid:"resource_main" drawn:0 tokens:nil name:nil asked:9000000 why:"full"}
A.pool_raise_apply()
assert(H.pool_raise.drawn==18000000 && A.pool_whole(H.pool_edit.max)>=18000000 && A.pool_raising("resource_main"))
assert(A.pool_raise_line("resource_main").search("18 M")>=0)
// At the account limit the maximum can't go up, and the card says so instead of offering it.
let capped=pool("resource_main","Research",9000000,0,9000000,350000,9000000)
open(capped)
H.pool_info.floor=A.pool_floor_read({is_ok:true data:floor_reply},"con_main")
let w=A.pool_view(H.resource_detail)
assert(!A.pool_can_raise(w) && A.pool_full_no_raise(w).search("9 M")>=0)
''',
'the_maximums_lock_reason_is_the_amendments_own': '''
// Given-out capacity never locks the maximum: an offered pool amends it. The amendment's reason wins.
let refused={allowed:false reason:"monthly_ceiling_required" sequence:0 minimumTokens:nil perRequestTokens:nil}
assert(A.pool_max_reason("retained_commitments",refused)==A.pool_max_reason(nil,refused))
assert(A.pool_max_reason("retained_commitments",refused).search(A.t("without a monthly maximum","没有设每月上限"))>=0)
// Without the amendment (an older Hagency) the update is the way forward.
assert(A.pool_max_reason("retained_commitments",nil)==A.pool_lock_rule())
// An allowed amendment of a pool with given-out capacity can go up.
open(main)
let v=A.pool_view(H.resource_detail)
assert(v.maxMode=="amend" && v.maxMin==9000000 && v.maxMax==50000000)
''',
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 source=(APP/'bundle/main.splash').read_text();results=[]
 for lang in ('en','zh-CN'):
  for name,case in CASES.items():
   path=a.output/(lang+'-'+name+'.splash');path.write_text(fixture(source+SETUP+case,lang))
   r=subprocess.run([str(a.vm.resolve()),'root',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
   path.with_suffix('.log').write_text(r.stdout);results.append(dict(case=name,language=lang,passed=r.returncode==0))
 (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps(dict(passed=sum(r['passed'] for r in results),total=len(results))));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
