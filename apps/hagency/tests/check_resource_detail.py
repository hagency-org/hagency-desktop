#!/usr/bin/env python3
"""Check actual resource-detail and navigation subtrees in the pinned script VM.

These are script-object/route checks, not native screenshots or budget authority.
"""
import argparse
import json
from pathlib import Path
import subprocess

APP = Path(__file__).resolve().parents[1]

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('vm',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args(); args.output.mkdir(parents=True,exist_ok=True)
    source=(APP/'bundle/main.splash').read_text()
    start=source.index('        if H.page == "resource_detail" {mod.widgets.HagencyDetailCard{')
    section=source[start:source.index('        if H.page == "projects"',start)]
    helpers=source[source.index('A.local_name = fn('):source.index('A.redraw = fn(')]
    helpers+=source[source.index('A.navigate = fn('):source.index('A.load_catalog = fn(')]
    helpers+=source[source.index('A.reasoning_label = fn('):source.index('A.render_page = fn(')]
    nav=source[source.index('navigation := View{'):source.index('notice := mod.widgets.HagencyHint')]
    nav=nav[nav.index('    if H.ready {'):].rsplit('}}',1)[0]
    prefix='''use mod.std.assert
let base={text:"" draw_icon:{} draw_bg:{} draw_text:{}}
let Fill=1
let Fit=2
let Down=1
let Flow={Down:1 Right:{}}
let View=base
let Walk={}
mod.widgets={RINX_SPACING:1 RINX_SELECTED:2 RINX_SURFACE:3 RINX_DISABLED:4 RINX_INK:5 RINX_ACCENT:6 RINX_ON_ACCENT:7 RINX_PAGE:8 RINX_ACCENT_HOVER:9 RINX_HOVER:10}
mod.rinx_icons={back:1 refresh:2 connection:3 resources:4 home:5 inbox:6 projects:7}
mod.widgets.HagencyDetailCard=base
mod.widgets.HagencySection=base
mod.widgets.HagencyActions=base
mod.widgets.HagencyTitle=base
mod.widgets.HagencySectionTitle=base
mod.widgets.HagencyCopy=base
mod.widgets.HagencyHint=base
mod.widgets.HagencyDetailRow=base
mod.widgets.HagencyFieldLabel=base
mod.widgets.RinxButton=base
mod.widgets.HagencyButton=base
mod.widgets.RinxHint=base
mod.widgets.HagencyInput=base
mod.widgets.RinxPrimaryButton=base
let H={resource_name_mode:"" resource_name_draft:nil command:nil ready:true page:"resource_detail" busy:false project_features:true resource_detail:nil resource_id:"resource_1" capability:"coding" selected:nil connection_project:nil confirm:"" notice:""}
let A={}
A.t=fn(en,zh){return en}
A.amount=fn(value){if value == nil {return A.t("Unknown","未知")} return ""+value}
A.capability_label=fn(value){return value}
A.projects_enabled=fn(){return H.project_features}
A.error_text=fn(code){return code}
A.redraw=fn(){}
A.refresh=fn(){H.refreshed=true}
A.request=fn(service,args,done){H.service=service H.args=args H.callback=done}
let texts=[]
let collect=fn(value){
    if !value.is_object() {return}
    let {text=""}=value
    if text != "" {texts.push(text)}
    for i key child in value {collect(child)}
}
let has=fn(text){for value in texts {if value == text {return true}} return false}
'''
    observed='''H.resource_detail={protocol:"hagency-management-v1" observedAtMs:1000 editing:{allowed:false reason:"contribution_history_not_proven"} resource:{id:"resource_1" framework:"codex" model:"Model with a long descriptive name" reasoning:"high" capabilities:["coding","documentation"] published:false budget:{remainingTokens:nil pool:{ceiling:5000 period:"monthly" committed:300}} usage:{spent:nil spend_period_key:nil}}}'''
    scenarios={
      'loading':('H.busy=true', 'assert(has(A.t("Reading this device’s resource…","正在读取此设备上的资源…")))'),
      'unavailable':('', 'assert(has(A.t("No current resource snapshot","暂无当前资源快照")))'),
      'unmeasured':(observed, 'assert(has(A.t("Unknown / 5000 tokens","未知 / 5000 token")))\nassert(has(A.t("No measurement yet","暂无观测记录")))'),
      'measured':(observed+'\nH.resource_detail.resource.budget.remainingTokens=4700\nH.resource_detail.resource.usage.spent=120\nH.resource_detail.resource.usage.spend_period_key="2026-10"\nH.resource_detail.resource.published=true', 'assert(has("2026-10"))\nassert(has(A.t("4700 / 5000 tokens","4700 / 5000 token")))\nassert(has(A.t("120 tokens","120 token")))\nassert(has(A.t("Listed locally","已登记到本机目录")))'),
      'zero':(observed+'\nH.resource_detail.resource.budget.pool.ceiling=0\nH.resource_detail.resource.budget.remainingTokens=0', 'assert(has(A.t("0 / 0 tokens","0 / 0 token")))'),
      'unknown_limit':(observed+'\nH.resource_detail.resource.budget.pool.ceiling=nil\nH.resource_detail.resource.budget.pool.period=nil', 'assert(has(A.t("Unknown / Unknown tokens","未知 / 未知 token")))\nassert(has(A.t("Not declared","未声明")))'),
    }
    results=[]
    for language in ['en','zh']:
      for case,(state,checks) in scenarios.items():
        script=prefix+helpers+'\n'
        if language=='zh':script+='A.t=fn(en,zh){return zh}\n'
        script+=state+'\nlet root=View{\n'+section+'}\ncollect(root)\n'+checks+'\n'
        if case not in ['loading','unavailable']:
          script+='assert(has(A.t("Configuration editing is not available","暂不支持编辑配置")))\nassert(has(A.t("High","高")))\nassert(has("resource_1"))\n'
        script+='''let back=false
for i key button in root[0][0] {
    if key.to_string() == "resource_back" {back=true if !H.busy {button.on_click()}}
    assert(key.to_string() != "resource_edit")
}
assert(back)
assert(H.capability == "coding")
if !H.busy {assert(H.page == "resources")}
H.busy=false
A.open_resource("resource_2")
assert(H.page == "resource_detail" && H.resource_id == "resource_2")
assert(H.resource_detail == nil)
assert(H.service == "hagency.resources.get" && H.args.id == "resource_2")
H.callback({protocol:"hagency-management-v1" resource:{id:"wrong"}})
assert(H.resource_detail == nil && H.notice != "")
H.callback({protocol:"hagency-management-v1" resource:{id:"resource_2"}})
assert(H.resource_detail.resource.id == "resource_2")
H.busy=true
let nav=View{'''+nav+'''}
assert(nav.len() == 5)
for i key button in nav {
    let {enabled=nil}=button
    if enabled != nil {assert(enabled == false)} else {assert(button[0].enabled == false)}
}
H.project_features=false
let old_nav=View{'''+nav+'''}
assert(old_nav.len() == 4)
for i key button in old_nav {assert(button.enabled == false)}
true
'''
        path=args.output/f'{case}-{language}.splash';path.write_text(script)
        run=subprocess.run([str(args.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        (args.output/f'{case}-{language}.log').write_text(run.stdout)
        results.append({'case':f'{case}-{language}','passed':run.returncode==0})
    (args.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
    failures=[r['case'] for r in results if not r['passed']]
    assert not failures,failures
    print('12 actual resource-detail subtree/route checks passed (EN/CN)')

if __name__=='__main__':main()
