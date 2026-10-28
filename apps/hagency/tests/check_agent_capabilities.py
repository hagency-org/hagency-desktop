#!/usr/bin/env python3
"""Whole shipped VM source + real capability callbacks; native test separately
checks View child de-duplication. No screenshots or server mutations are made.
"""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP

STATE='''
let H=mod.hagency
let A=mod.capability_fixture_actions
H.ready=true H.busy=false H.page="agent_wizard" H.command=nil H.agent_review=false
H.session={account:"@manager:test" features:["agent-admission-v1"]}
H.agent_project_id="project"
H.agent_draft={projectId:"project" name:"Writer" description:"Draft" capability:"testing" tokens:"100"}
H.agent_readiness={projectId:"project" projectName:"Original project" administratorMxid:"@admin:test" canRequest:true canViewRequests:true blockingReason:nil requestContext:{capabilities:["architect","coding","documentation","integration","review","testing"] periodKey:"2026-10" requestedTokenLimit:700}}
A.save=fn(){return true}
A.redraw=fn(){}
let rendered=View{A.render_page()}
let choices=[]
let visit=fn(node){
    if !node.is_object(){return}
    let sibling_names=[]
    for i key child in node {
        if key.to_string() == "agent_capability" {
            // The actual View reconciles named children by their parent/key.
            assert(sibling_names.len() == 0)
            sibling_names.push(key)
            choices.push(child)
        }
        visit(child)
    }
}
visit(rendered)
assert(choices.len() == 6)
let expected=["architect","coding","documentation","integration","review","testing"]
for i button in choices {
    assert(button.text == A.capability_label(expected[i]))
    button.on_click()
    assert(H.agent_draft.capability == expected[i])
}
choices[1].on_click()
assert(H.agent_draft.capability == "coding")
assert(H.command == nil && H.agent_review == false)
true
'''
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 source=a.bundle.read_text();results=[]
 for lang in ['en','zh-CN']:
  start=source.index('        if H.page == "agent_wizard" && A.agents_enabled()')
  body=source[start:source.index('        if H.page == "agent_action"', start)]
  script=fixture(source+'\nmod.capability_fixture_actions=A\n',lang)+STATE.replace('let rendered=View{A.render_page()}', 'let rendered=View{'+body+'}')
  path=a.output/(lang+'.splash');path.write_text(script)
  r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);(a.output/(lang+'.log')).write_text(r.stdout);results.append({'language':lang,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(v['passed'] for v in results),'total':len(results)}));raise SystemExit(not all(v['passed'] for v in results))
if __name__=='__main__':main()
