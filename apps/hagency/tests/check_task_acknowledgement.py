#!/usr/bin/env python3
"""Full shipped task handlers: terminal acknowledgement, bounded reuse, lost replies.

The host model mirrors the native eight-unacknowledged-intent bound. It proves
script sequencing only; actual SQLite/host authority has separate native tests.
"""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
from check_operator_task_writers import SETUP,section
SETUP=SETUP.replace('A.request=fn(service,args,done){calls.push({service:service args:args}) replies.push(done)}\n','')+'\nH.saved_commands=[]\n'
SAVED='''let original={commandId:"taskcmd_original" taskId:"task_original" operation:"task_comment" change:{operation:"comment" input:{id:"task_original" expectedRevision:7 text:"Saved comment"}} result:{commandId:"taskcmd_original" taskId:"task_original" revision:8 deleted:false} error:nil}
'''
CASES={
 'twelve_successful_saves_do_not_fill_eight_intent_slots': '''
let count=0 let unacked=0 let maximum=0 let applied=0 let acknowledged=0
host.request=fn(service,args,done){
 calls.push({service:service args:args})
 if service=="hagency.tasks.create"{assert(unacked<8) count+=1 unacked+=1 maximum=max(maximum,unacked) let id="taskcmd_"+count.to_string() done({is_ok:true data:{commandId:id taskId:"task_"+count.to_string() operation:"task_create" change:{operation:"create" input:args} result:nil error:nil}})}
 else if service=="hagency.tasks.command.apply"{applied+=1 done({is_ok:true data:{commandId:args.commandId taskId:"task_"+count.to_string() revision:1 deleted:false}})}
 else if service=="hagency.intent.acknowledge"{assert(args.commandId=="taskcmd_"+count.to_string()) unacked-=1 acknowledged+=1 done({is_ok:true data:{acknowledged:true}})}
 else if service=="hagency.tasks.get"{done({is_ok:true data:{supported:true writeSupported:true writeOperations:["edit","comment"] revision:1 task:{id:args.id title:"Saved"}}})}
 else{assert(false)}
}
for n in 12{A.task_begin("create","") A.task_set("title","Task "+n.to_string()) A.task_prepare() assert(H.task_phase=="recorded" && unacked==1) A.task_current() assert(H.task_pending==nil && !H.busy && unacked==0)}
assert(count==12 && applied==12 && acknowledged==12 && maximum==1)
''',
 'lost_ack_keeps_original_and_retry_never_prepares_replacement': SAVED+'''
A.task_prepared(original) A.task_current()
assert(calls.len()==1 && calls[0].service=="hagency.intent.acknowledge" && calls[0].args.commandId=="taskcmd_original" && H.busy)
A.task_current() A.task_begin("create","") assert(calls.len()==1)
replies[0]({is_ok:false error:"management_outcome_unknown"})
assert(!H.busy && H.task_pending.commandId=="taskcmd_original" && H.task_phase=="recorded" && H.page=="operator_task_command")
A.task_current() assert(calls.len()==2 && calls[1].args.commandId=="taskcmd_original")
replies[1]({is_ok:true data:{acknowledged:true}})
assert(calls.len()==3 && calls[2].service=="hagency.tasks.get" && calls[2].args.id=="task_original" && H.task_pending==nil)
''',
 'new_edit_waits_for_previous_terminal_ack_then_preserves_current_revision': SAVED+'''
A.task_prepared(original)
H.task_detail.revision=11 H.task_detail.task.id="different_task"
A.task_begin("comment","")
assert(calls.len()==1 && calls[0].service=="hagency.intent.acknowledge" && H.task_pending.commandId=="taskcmd_original")
replies[0]({is_ok:true data:{acknowledged:true}})
assert(calls.len()==1 && H.task_pending==nil && H.task_phase=="editing" && H.task_draft.input.id=="different_task" && H.task_draft.input.expectedRevision==11)
A.task_set("text","Next update") A.task_prepare()
assert(calls.len()==2 && calls[1].service=="hagency.tasks.comment" && calls[1].args.id=="different_task" && calls[1].args.expectedRevision==11)
''',
 'failed_ack_from_new_edit_routes_back_to_exact_result': SAVED+'''
A.task_prepared(original) H.page="operator_tasks" A.task_begin("create","")
replies[0]({is_ok:false error:"native_resume_missing"})
assert(H.needs_signin && H.page=="operator_task_command" && H.task_pending.commandId=="taskcmd_original" && H.task_phase=="recorded" && calls.len()==1)
''',
 'unknown_saved_and_submitting_never_acknowledge': SAVED+'''
for phase in ["unknown","saved","submitting","saving","loading"]{H.task_pending=original H.task_phase=phase H.busy=false A.task_current() A.task_finish(fn(){assert(false)}) A.task_begin("create","") assert(calls.len()==0 && H.task_pending.commandId=="taskcmd_original")}
''',
 'restart_read_and_terminal_refresh_do_not_mutate': SAVED+'''
A.open_saved_task({commandId:"taskcmd_original"})
assert(calls.len()==1 && calls[0].service=="hagency.tasks.command.get")
replies[0]({is_ok:true data:original})
assert(calls.len()==1 && H.task_phase=="recorded")
A.task_saved_refresh() assert(calls.len()==2 && calls[1].service=="hagency.tasks.command.get")
replies[1]({is_ok:true data:original}) assert(calls.len()==2)
A.task_current() assert(calls.len()==3 && calls[2].service=="hagency.intent.acknowledge")
''',
 'deleted_result_acknowledges_before_back_to_list': SAVED+'''
original.result.deleted=true A.task_prepared(original) A.task_current()
assert(calls.len()==1 && calls[0].service=="hagency.intent.acknowledge" && H.page=="operator_task_command")
replies[0]({is_ok:true data:{acknowledged:true}})
assert(calls.len()==2 && calls[1].service=="hagency.tasks.list" && H.page=="operator_tasks" && H.task_pending==nil)
''',
 'known_failure_keeps_exact_request_until_acknowledged': SAVED+'''
original.result=nil original.error="management_task_revision_changed" A.task_prepared(original) A.task_current()
assert(calls.len()==1 && calls[0].args.commandId=="taskcmd_original")
replies[0]({is_ok:false error:"management_outcome_unknown"})
assert(H.task_phase=="failed" && H.task_pending.error=="management_task_revision_changed")
A.task_current() replies[1]({is_ok:true data:{acknowledged:true}})
assert(calls.len()==3 && calls[2].service=="hagency.tasks.get" && H.task_pending==nil)
'''
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);p.add_argument('--case');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);source=a.bundle.read_text();results=[]
 CASES['details_dismiss_waits_and_retains_receipt_on_failure']=SAVED+'''
A.task_prepared(original) H.task_result_details=true
let tree=View{'''+section(source,'operator_task_command')+'''} visit(tree)
find("task_result_dismiss").on_click()
assert(calls.len()==1 && calls[0].service=="hagency.intent.acknowledge" && H.task_pending.commandId=="taskcmd_original" && H.page=="operator_task_command")
replies[0]({is_ok:false error:"management_outcome_unknown"})
assert(H.task_pending.commandId=="taskcmd_original" && H.task_phase=="recorded" && !H.busy)
find("task_result_dismiss").on_click() assert(calls.len()==2 && calls[1].args.commandId=="taskcmd_original")
replies[1]({is_ok:true data:{acknowledged:true}})
assert(calls.len()==3 && calls[2].service=="hagency.tasks.list" && H.task_pending==nil)
'''
 for lang in ['en','zh-CN']:
  for name,test in CASES.items():
   if a.case and name!=a.case:continue
   path=a.output/f'{lang}-{name}.splash';path.write_text(fixture(source+'\nmod.task_writer_actions=A\n',lang)+SETUP+test+'\ntrue\n');r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);path.with_suffix('.log').write_text(r.stdout);results.append({'language':lang,'case':name,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
