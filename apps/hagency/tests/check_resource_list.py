#!/usr/bin/env python3
"""Evaluate actual resource cards and exact-ID callbacks, not native pixels."""
import argparse
import json
from pathlib import Path
import subprocess

APP = Path(__file__).resolve().parents[1]
PREFIX = '''use mod.std.assert
let base={text:"" draw_icon:{} draw_bg:{} draw_text:{}}
let Fill=1
let Fit=2
let View=base
let Flow={Right:{}}
let Walk={}
mod.widgets={RINX_SELECTED:1 RINX_FIELD:2}
mod.rinx_icons={add:1 resources:2 connection:3}
'''
for widget in ['HagencySection','HagencyTitle','HagencyHint','HagencyActions','HagencyIconButton','HagencyCard','HagencyDetailCard','HagencySectionTitle','HagencyDetailRow','HagencyFieldLabel','HagencyCopy','RinxButton','HagencyButton','HagencyCardHeading','HagencyBadge','HagencyGlyph','RinxPrimaryButton','RinxLabel']:
    PREFIX += f'mod.widgets.{widget}=base\n'
PREFIX += '''let H={page:"resources" busy:false resources:[] resource_after:nil connections:[] session:{account:"@owner:test" device:"device"} capability:"coding" command:{id:"original"}}
let A={}
A.t=fn(en,zh){return en}
A.amount=fn(value){if value == nil {return A.t("Unknown","未知")} return ""+value}
A.capability_label=fn(value){return value}
A.refresh=fn(){H.refreshed=true}
A.new_resource=fn(){H.new_resource=true}
A.open_onboarding=fn(){H.opened_onboarding=true}
A.open_resource=fn(id){H.opened=id}
A.connect_existing=fn(row){H.connected=row.id}
A.more=fn(){H.more=true}
let texts=[]
let collect=fn(value){
    if !value.is_object(){return}
    let {text=""}=value
    if text != "" {texts.push(text)}
    for i key child in value {collect(child)}
}
let has=fn(text){for value in texts {if value == text {return true}} return false}
let find=fn(value,id){
    if !value.is_object(){return nil}
    for i key child in value {
        if key.to_string() == id {return child}
        let found=find(child,id)
        if found != nil {return found}
    }
    return nil
}
let resource=fn(id){return {id:id model:"Same model" framework:"codex" capabilities:["coding","documentation"] budget:{remainingTokens:123456 reserved:0 pool:{ceiling:123456 period:"monthly"}} usage:{spent:nil}}}
'''

def main():
    parser=argparse.ArgumentParser();parser.add_argument('vm',type=Path);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    source=(APP/'bundle/main.splash').read_text()
    start=source.index('        if H.page == "resources" {',source.index('A.render_page'))
    section=source[start:source.index('        if H.page == "onboarding"',start)]
    helpers=source[source.index('A.resource_period = fn('):source.index('A.redraw = fn(')]
    cases={
      'empty':('', 'assert(root.len() == 2)\nassert(has(A.t("No resources in this view","当前视图暂无资源")))'),
      'same_model_budget':('H.resources=[resource("res_aaaaaaaaaaaa11111111"),resource("res_bbbbbbbbbbbb22222222")]', '''assert(has(A.t("Resource ID · ","资源标识 · ")+"res_…11111111"))
assert(has(A.t("Resource ID · ","资源标识 · ")+"res_…22222222"))
assert(has(A.t("Unknown tokens","未知 token")))
assert(root.len() == 3)'''),
      'abbreviation_collision':('H.resources=[resource("res_aaaaaaaaaaaaxxxxxxxx"),resource("res_bbbbbbbbbbbbxxxxxxxx")]', '''assert(has(A.t("Resource ID · ","资源标识 · ")+H.resources[0].id))
assert(has(A.t("Resource ID · ","资源标识 · ")+H.resources[1].id))
assert(!has(A.t("Resource ID · ","资源标识 · ")+"res_…xxxxxxxx"))'''),
      'zero_and_unknown':('H.resources=[resource("short-id"),resource("other-id")] H.resources[0].budget.remainingTokens=0 H.resources[0].budget.pool.ceiling=0 H.resources[1].budget.remainingTokens=nil H.resources[1].budget.pool.ceiling=nil H.resources[1].budget.pool.period=nil','''assert(has("0 / 0"))
assert(has(A.t("Unknown / Unknown","未知 / 未知")))
assert(has(A.t("Not declared","未填写")))
assert(has(A.t("Resource ID · ","资源标识 · ")+"short-id"))'''),
      'busy_paging':('H.busy=true H.resources=[resource("res_aaaaaaaaaaaa11111111")] H.resource_after="cursor"','''assert(find(root,"all").enabled == false)
assert(find(root,"coding").enabled == false)
assert(find(root,"docs").enabled == false)
assert(find(root,"add").enabled == false)
assert(find(root,"more").enabled == false)
assert(find(root,"resource_detail").enabled == false)
assert(find(root,"offer").enabled == false)'''),
      'named_connection':('H.resources=[resource("resource-with-connection")] H.connections=[{resourceId:"resource-with-connection" ownerMxid:"@owner:test" executorDeviceId:"device" label:"Server label is separate"}]','''assert(has(A.t("View connection","查看连接")))
assert(!has("Server label is separate"))'''),
      'unicode_shortening':('H.resources=[resource("资源资源一二三四五六七八九十甲乙丙丁戊己庚辛壬癸")]', '''assert(A.resource_short_id(H.resources[0].id) == "资源资源…丙丁戊己庚辛壬癸")'''),
    }
    results=[]
    for language in ['en','zh']:
        for case,(state,assertions) in cases.items():
            script=PREFIX+helpers+'\n'
            if language=='zh':script+='A.t=fn(en,zh){return zh}\n'
            script+=state+'\nlet root=View{\n'+section+'}\ncollect(root)\n'+assertions+'\n'
            script+='''if !H.busy {
    for i row in H.resources {
        find(root[i+1],"resource_detail").on_click()
        assert(H.opened == row.id)
        find(root[i+1],"offer").on_click()
        assert(H.connected == row.id)
    }
}
assert(H.capability == "coding")
assert(H.command.id == "original")
true
'''
            path=args.output/f'{case}-{language}.splash';path.write_text(script)
            run=subprocess.run([str(args.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            (args.output/f'{case}-{language}.log').write_text(run.stdout)
            results.append({'case':f'{case}-{language}','passed':run.returncode==0})
    (args.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
    assert all(r['passed'] for r in results),[r for r in results if not r['passed']]
    print('14 actual resource list/identity checks passed (EN/CN)')

if __name__=='__main__':main()
