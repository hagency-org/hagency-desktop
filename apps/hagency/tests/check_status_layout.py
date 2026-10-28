#!/usr/bin/env python3
"""Check actual setup/connection trees after layout edits; no native rendering."""
import argparse
import json
from pathlib import Path
import subprocess

APP=Path(__file__).resolve().parents[1]
PREFIX='''use mod.std.assert
let base={text:"" draw_icon:{} draw_bg:{} draw_text:{}}
let Fill=1
let Fit=2
let View=base
let Icon=base
let Flow={Right:{}}
let Walk={}
let Align={}
mod.widgets={RINX_ACCENT:1}
mod.rinx_icons={back:1 check:2 connection:3 refresh:4}
'''
for name in ['HagencyDetailCard','HagencySection','HagencyTitle','HagencySectionTitle','HagencyDetailRow','HagencyFieldLabel','HagencyCopy','HagencyHint','HagencyActions','HagencyStatus','HagencyInput','HagencyDivider','RinxButton','HagencyButton','HagencyCardHeading','HagencyBadge','HagencyGlyph','RinxPrimaryButton','RinxKeyPrimaryButton','HagencyEyebrow','RinxLabel']:
    PREFIX+=f'mod.widgets.{name}=base\n'
PREFIX+='''let H={page:"project_setup" busy:false command:nil confirm:"" session:{account:"@manager:test" device:"device" deviceGeneration:2 canAdmitResources:false features:["project-requests-v1","project-setup-v1"]} connection_project:nil project_selected:{id:"project" managerMxid:"@manager:test" name:"Project with a deliberately long descriptive name for context"} setup_confirmation:nil setup_recover_hint:false setup_admin:"" setup_space:"" setup_room:""}
H.setup_progress={readiness:{projectId:"project" setup:nil administrator:{mxid:"@admin:test"} eligibleAllocation:nil matrixReady:false capacityReady:false canAssignAdministrator:false canBegin:false canConfirm:false canCancel:false canRebase:false blockingReason:"live_reservation_required"} localHistory:false space:nil room:nil pendingCommands:[] failedCommands:[] blocker:nil}
H.selected={contribution:{id:"connection" label:"Named server contribution" state:"admitted" verification:{status:"expired"} ownerMxid:"@owner:test" coordinatorMxid:"@owner:test" executorDeviceId:"device" executorGeneration:2 runtime:"codex" capabilities:["coding","documentation"]} request:{canDecide:false approverMxid:"@admin:test"}}
let A={}
A.t=fn(en,zh){return en}
A.error_text=fn(code){return code}
A.capability_label=fn(value){return value}
A.open_project=fn(id){H.opened=id}
A.open_project_setup=fn(id){H.setup_opened=id}
A.setup_prepare=fn(action,id){H.prepared=action H.prepared_id=id}
A.setup_apply=fn(){H.applied=true}
A.redraw=fn(){}
A.readmission_enabled=fn(){return false}
A.resume=fn(row){H.resumed=row.id}
A.verify=fn(row){H.verified=row.id}
A.revoke=fn(){H.revoked=true}
A.decide=fn(approve){H.decided=approve}
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
'''

def main():
    p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);p.add_argument('--source',type=Path,default=APP/'bundle/main.splash')
    args=p.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    src=args.source.read_text()
    helpers=src[src.index('A.state_text = fn('):src.index('A.error_text = fn(')]
    helpers+=src[src.index('A.setup_enabled = fn('):src.index('A.setup_received = fn(')]
    start=src.index('        if H.page == "project_setup" &&')
    setup=src[start:src.index('        if H.page == "connections"',start)]
    start=src.index('        if H.page == "action" &&')
    connection=src[start:src.index('\n    }\n}\n\nmod.widgets.HagencyCard',start)]
    cases={
      'setup_readonly':('', '''assert(has(A.t("Project setup","项目设置")))
assert(has(H.project_selected.name))
assert(has(A.t("Room creation has not begun","尚未开始创建房间")))
assert(find(root,"setup_assign") == nil && find(root,"setup_begin") == nil)
find(root,"setup_back").on_click()
assert(H.opened == "project")'''),
      'legacy_assignment_is_hidden':('H.setup_progress.readiness.canAssignAdministrator=true','''assert(find(root,"setup_assign") == nil)
assert(find(root,"setup_admin_input") == nil)
assert(find(root,"setup_begin") == nil)'''),
      'setup_confirmation':('H.confirm="project_setup" H.setup_confirmation={title:"Update project requirements" detail:"@manager:test"}','''assert(has("Update project requirements"))
assert(find(root,"setup_assign") == nil)
find(root,"setup_confirmation_cancel").on_click()
assert(H.confirm == "" && H.setup_confirmation == nil)'''),
      'setup_unknown_saved':('H.setup_progress.pendingCommands=["original-command"]','''find(root,"setup_recover").on_click()
assert(H.prepared == "retry" && H.prepared_id == "original-command")
assert(find(root,"setup_begin") == nil)'''),
      'connection_context':('H.page="action" H.connection_project="project"','''assert(has(H.project_selected.name))
assert(has("Named server contribution"))
assert(has(A.t("Connection needs a fresh check","连接需要重新检查")))
assert(has(A.t("Waiting for the resource owner to verify from their enrolled device.","等待资源主人使用登记的设备验证连接。")))
assert(find(root,"verify") == nil && find(root,"revoke") == nil && find(root,"approve") == nil)
find(root,"connection_project_back").on_click()
assert(H.opened == "project")'''),
      'connection_direct':('H.page="action"','''assert(find(root,"connection_project_back") == nil)
assert(!has(H.project_selected.name))
assert(has("Named server contribution"))
assert(has(A.t("People and resource","人员与资源")))'''),
      'connection_owner':('H.page="action" H.session.account="@owner:test"','''find(root,"verify").on_click()
assert(H.verified == "connection")
assert(find(root,"approve") == nil)
find(root,"revoke").on_click()
assert(H.confirm == "revoke")'''),
      'connection_decision':('H.page="action" H.selected.request.canDecide=true H.selected.contribution.state="pending_admission"','''assert(find(root,"verify") == nil)
find(root,"approve").on_click()
assert(H.confirm == "approve")'''),
    }
    results=[]
    for language in ['en','zh']:
        for name,(state,checks) in cases.items():
            script=PREFIX+helpers+'\n'
            if language=='zh':script+='A.t=fn(en,zh){return zh}\n'
            page = setup if name.startswith('setup_') else connection
            script+=state+'\nlet root=View{'+page+'}\ncollect(root)\n'+checks+'\ntrue\n'
            path=args.output/f'{name}-{language}.splash';path.write_text(script)
            result=subprocess.run([str(args.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            (args.output/f'{name}-{language}.log').write_text(result.stdout)
            results.append({'case':f'{name}-{language}','passed':result.returncode==0})
    (args.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
    assert all(r['passed'] for r in results),[r for r in results if not r['passed']]
    print('16 actual status layout/role/navigation checks passed (EN/CN)')

if __name__=='__main__':main()
