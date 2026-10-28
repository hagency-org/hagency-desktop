#!/usr/bin/env python3
"""A language notification redraws the shipped bundle without changing pending work."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP

def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 source=a.bundle.read_text();results=[]
 for language in ['en','zh-CN']:
  test=fixture(source+'\nmod.locale_actions=A\nmod.locale_hook=on_app_locale_changed\n',language)+'''
let H=mod.hagency let A=mod.locale_actions
let calls=[] let redraws=0
host.request=fn(service,args,done){calls.push(service)}
A.redraw=fn(){redraws+=1}
H.page="operator_task_edit" H.busy=true
H.command={commandId:"original-control" service:"hagency.agents.control.set"}
H.task_draft={operation:"edit" input:{title:"English 中文 {count}" expectedRevision:7}}
H.task_pending={commandId:"original-task" taskId:"original-id"}
H.task_phase="unknown"
let before=H.to_json()
mod.locale_hook("zh-CN") mod.locale_hook("en")
assert(redraws==2 && calls.len()==0 && H.to_json()==before)
true
'''
  path=a.output/(language+'.splash');path.write_text(test);r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);path.with_suffix('.log').write_text(r.stdout);results.append({'language':language,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
