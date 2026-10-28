#!/usr/bin/env python3
"""Full-bundle VM: owner-local discovery, closed handoff and original recovery.
No pixels, live host, provider process or fabricated execution acceptance.
"""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
SETUP='''
let H=mod.hagency let A=mod.execution_actions
A.redraw=fn(){return nil}
H.busy=false H.ready=true H.page="device_execution" H.notice=""
let original={service:"original" args:{commandId:"do_not_replace"}}
H.command=original
let row={assignmentId:"assignment_a" requestId:"request_a" agentName:"Original agent" projectId:"project_a" predecessorCommandId:"previous_a" processAttempt:1 processState:"stopped" savedStart:true}
H.execution_page={supported:true items:[row] nextAfter:"assignment_a"}
let calls=[]
A.request=fn(service,args,done){calls.push({service:service args:args})}
'''
CASES={
'closed_selection_and_no_mutation_before_native_review': '''
A.execution_action("foreign",false) assert(calls.len()==0)
A.execution_action("assignment_a",false)
assert(calls.len()==1 && calls[0].service=="hagency.execution.review")
assert(calls[0].args.to_json()=={assignmentId:"assignment_a"}.to_json())
assert(H.command==original)
''',
'resume_requires_original_saved_intent': '''
row.savedStart=false A.execution_action("assignment_a",true) assert(calls.len()==0)
row.savedStart=true A.execution_action("assignment_a",true)
assert(calls.len()==1 && calls[0].service=="hagency.execution.resume" && calls[0].args.assignmentId=="assignment_a")
assert(H.command==original)
''',
'unsupported_and_empty_never_start': '''
H.execution_page={supported:false items:[row] nextAfter:nil reason:"shared_successors_disabled"}
A.execution_action("assignment_a",false) assert(calls.len()==0)
H.execution_page={supported:true items:[] nextAfter:nil}
A.execution_action("assignment_a",false) assert(calls.len()==0)
A.open_execution("assignment_a")
assert(calls.len()==1 && calls[0].service=="hagency.execution.assignments" && calls[0].args.after=="assignment_a")
assert(H.command==original)
''',
'results_distinguish_unknown_cancel_and_no_saved': '''
let closed=A.execution_result_text({state:"review_closed"})
let unknown=A.execution_result_text({state:"outcome_unknown"})
let none=A.execution_result_text({state:"recovered" result:{state:"no_saved_intent"}})
let running=A.execution_result_text({state:"observed" progress:{state:"running" command_id:"original"}})
assert(closed!=unknown && none!=running && closed!=none)
assert(H.command==original)
'''
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 source=a.bundle.read_text();start=source.index('        if H.page == "device_execution" {mod.widgets.HagencyDetailCard{');end=source.index('\n        if H.page ==',start);section=source[start:end]
 CASES['actual_page_has_distinct_assignment_controls']='''
let second={..row assignmentId:"assignment_b" savedStart:false}
H.execution_page.items=[row,second]
let starts=[] let resumes=[]
let visit=fn(node){if !node.is_object(){return} for i key child in node {if key.to_string()=="execution_review"{starts.push(child)} if key.to_string()=="execution_resume"{resumes.push(child)} visit(child)}}
let root=View{'''+section+'''} visit(root)
assert(starts.len()==2 && resumes.len()==1)
starts[0].on_click() starts[1].on_click() resumes[0].on_click()
assert(calls.len()==3 && calls[0].args.assignmentId=="assignment_a" && calls[1].args.assignmentId=="assignment_b" && calls[2].service=="hagency.execution.resume")
assert(H.command==original)
'''
 results=[]
 for lang in ['en','zh-CN']:
  for name,test in CASES.items():
   path=a.output/f'{lang}-{name}.splash';path.write_text(fixture(source+'\nmod.execution_actions=A\n',lang)+SETUP+test+'\ntrue\n')
   r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);path.with_suffix('.log').write_text(r.stdout);results.append({'language':lang,'case':name,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
