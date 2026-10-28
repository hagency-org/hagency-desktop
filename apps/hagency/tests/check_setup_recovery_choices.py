#!/usr/bin/env python3
"""Keep every original setup command reachable with distinct parent widgets.

Evaluates the full shipped source and the exact saved-request section, then
invokes each original callback. These are VM checks, not native pixels.
"""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP

CHECK='''
let H=mod.hagency
let A=mod.setup_choice_actions
let p={pendingCommands:IDS readiness:{projectId:"original-project"}}
H.command={service:"original_pending" args:{commandId:"unchanged"}}
H.busy=false
H.session={features:["project-setup-v1"]}
H.project_selected={id:"original-project"}
H.setup_progress=p
H.setup_confirmation=nil
H.confirm=""
A.redraw=fn(){return nil}
let calls=[]
A.request=fn(service,args,done){calls.push({service:service args:args})}
let root=View{SECTION}
let choices=[]
let texts=[]
let visit=fn(node){
    if !node.is_object(){return}
    let {text=""}=node
    if text != "" {texts.push(text)}
    let found=false
    for i key child in node {
        if key.to_string() == "setup_recover" {
            assert(!found)
            found=true
            choices.push(child)
        }
        visit(child)
    }
}
visit(root)
assert(choices.len() == p.pendingCommands.len())
for i button in choices {
    let visible=false
    for text in texts {if text == p.pendingCommands[i] {visible=true}}
    assert(visible)
    button.on_click()
    assert(H.confirm == "project_setup")
    assert(H.setup_confirmation.service == "hagency.projects.setup.retry")
    assert(H.setup_confirmation.args.commandId == p.pendingCommands[i])
}
assert(H.command.service == "original_pending" && H.command.args.commandId == "unchanged")
assert(calls.len() == 0)
let previous=H.setup_confirmation
A.setup_prepare("retry", "never-saved")
assert(H.setup_confirmation == previous)
assert(calls.len() == 0)
true
'''
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 src=a.bundle.read_text();start=src.index('                if p.pendingCommands.len() > 0');section=src[start:src.index('                if p.failedCommands.len()',start)]
 results=[]
 for lang in ['en','zh-CN']:
  for count in [0,1,3]:
   ids=['original-begin','original-confirm','original-cancel'][:count]
   script=fixture(src+'\nmod.setup_choice_actions=A\n',lang)+CHECK.replace('IDS',json.dumps(ids)).replace('SECTION',section)
   path=a.output/f'{lang}-{count}.splash';path.write_text(script)
   r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);path.with_suffix('.log').write_text(r.stdout);results.append({'language':lang,'count':count,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(v['passed'] for v in results),'total':len(results)}));raise SystemExit(not all(v['passed'] for v in results))
if __name__=='__main__':main()
