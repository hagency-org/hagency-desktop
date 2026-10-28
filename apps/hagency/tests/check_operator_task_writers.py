#!/usr/bin/env python3
"""Closed task writer controls from the full shipped VM source; no live writes."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
SETUP='''
let H=mod.hagency let A=mod.task_writer_actions
A.redraw=fn(){return nil} H.busy=false H.ready=true H.notice="" H.command=nil
let calls=[] let replies=[]
host.request=fn(service,args,done){calls.push({service:service args:args}) replies.push(done)}
A.request=fn(service,args,done){calls.push({service:service args:args}) replies.push(done)}
H.task_page={supported:true writeSupported:true writeOperations:["create"] tasks:[] nextOffset:nil}
H.task_id="task_original"
H.task_detail={supported:true writeSupported:true writeOperations:["edit","delete","accept","transition","comment"] revision:7 next:["in_progress"] commentCount:0 nextCommentOffset:nil task:{id:"task_original" title:"Original title" description:"Original description" priority:"p1" granularity:"task" assignee:"Agent A" parent_id:nil labels:["one","two"] status:"blocked" created_by:"@owner:test" created_at:"then" updated_at:"now" started_at:nil completed_at:nil heartbeat_at:nil waiting_reason:"review" waiting_until:"tomorrow" comments:[]}}
let widgets=[] let visit=fn(node){if !node.is_object(){return} for index key child in node{if key.is_string() || key.is_id(){widgets.push({name:key.to_string() value:child})} visit(child)}}
let find=fn(name){for row in widgets{if row.name==name{return row.value}} return nil}
'''
CASES={
'prepare_apply_lost_reply_retains_exact_native_ids': '''
A.task_begin("create","") A.task_set("title","First original") A.task_prepare()
assert(calls.len()==1 && calls[0].service=="hagency.tasks.create" && calls[0].args.title=="First original")
let pending={commandId:"native_original" taskId:"task_native" operation:"task_create" change:{operation:"create" input:calls[0].args} result:nil error:nil}
replies[0]({is_ok:true data:pending})
assert(H.task_phase=="submitting" && H.task_pending.commandId=="native_original" && H.command==nil)
A.task_apply() A.task_apply()
assert(calls.len()==2 && calls[1].service=="hagency.tasks.command.apply" && calls[1].args.commandId=="native_original")
replies[1]({is_ok:false error:"management_outcome_unknown"})
assert(H.task_phase=="unknown" && H.task_pending.taskId=="task_native")
A.task_apply()
assert(calls.len()==3 && calls[2].args.commandId=="native_original")
replies[2]({is_ok:true data:{commandId:"native_original" taskId:"task_native" revision:1 deleted:false status:"created" updatedAt:"now" commentCount:0}})
assert(H.task_phase=="recorded") A.task_apply() assert(calls.len()==3)
''',
'cancel_edit_has_no_writer_and_cas_is_never_rebased': '''
let original_detail=H.task_detail
A.task_begin("edit","") A.task_set("title","Changed")
assert(H.task_draft.input.expectedRevision==7 && H.task_detail.task.title=="Original title")
A.task_cancel_edit() assert(calls.len()==1 && calls[0].service=="hagency.tasks.get")
replies[0](original_detail)
A.task_begin("edit","") A.task_set("assignee","") A.task_prepare()
assert(calls[1].args.expectedRevision==7 && calls[1].args.id=="task_original" && calls[1].args.assignee=="")
replies[1]({is_ok:true data:{commandId:"saved_edit" taskId:"task_original" operation:"task_edit" change:{operation:"edit" input:calls[1].args} result:nil error:nil}})
A.task_apply() replies[2]({is_ok:false error:"management_task_revision_changed"})
assert(H.task_phase=="failed" && H.task_pending.change.input.expectedRevision==7)
A.task_apply() assert(calls.len()==3)
''',
'restart_review_never_applies_and_unavailable_has_no_writer': '''
A.recover_native({commandId:"original_comment" operation:"task_comment" hasReply:false error:nil})
assert(calls.len()==1 && calls[0].service=="hagency.tasks.command.get" && calls[0].args.commandId=="original_comment")
H.task_page={supported:false} A.task_begin("create","") assert(calls.len()==1)
H.task_detail={supported:true writeSupported:false revision:7} A.task_begin("delete","") assert(calls.len()==1)
''',
'block_requires_both_fields_and_comment_preserves_revision': '''
H.task_detail.task.status="in_progress" H.task_detail.next=["blocked","done"]
A.task_begin("transition","blocked") A.task_prepare() assert(calls.len()==0)
A.task_set("waitingReason","Review") A.task_prepare() assert(calls.len()==0)
A.task_set("waitingUntil","Tomorrow") A.task_prepare()
assert(calls.len()==1 && calls[0].service=="hagency.tasks.transition" && calls[0].args.expectedRevision==7 && calls[0].args.status=="blocked")
''',
'prepare_failure_never_mints_another_request': '''
A.task_begin("create","") A.task_set("title","Original") A.task_prepare()
replies[0]({is_ok:false error:"management_outcome_unknown"})
A.task_prepare() A.task_apply() A.task_begin("create","")
assert(calls.len()==1 && H.task_phase=="unknown" && H.task_pending==nil)
A.recover_native({commandId:"saved_before_reply_loss" operation:"task_create"})
assert(calls.len()==2 && calls[1].service=="hagency.tasks.command.get" && calls[1].args.commandId=="saved_before_reply_loss")
replies[1]({commandId:"saved_before_reply_loss" taskId:"one_task" operation:"task_create" change:{operation:"create" input:{title:"Original"}} result:nil error:nil})
assert(calls.len()==2 && H.task_phase=="saved")
A.task_apply() assert(calls.len()==3 && calls[2].args.commandId=="saved_before_reply_loss")
'''
}
def section(source,page):
 start=source.index('        if H.page == "'+page+'"');return source[start:source.index('\n        if H.page ==',start+10)]
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);source=a.bundle.read_text()
 CASES['editor_widgets_keep_all_priority_granularity_label_callbacks']='''
A.task_begin("edit","") H.task_options_open=true
let tree=View{'''+section(source,'operator_task_edit')+'''} visit(tree)
let priority=[] let granularity=[] let labels=[]
for row in widgets{if row.name=="task_edit_priority"{priority.push(row.value)} if row.name=="task_edit_granularity"{granularity.push(row.value)} if row.name=="task_remove_label"{labels.push(row.value)}}
assert(priority.len()==4 && granularity.len()==3 && labels.len()==2)
priority[0].on_click() granularity[2].on_click() labels[1].on_click()
assert(H.task_draft.input.priority=="p0" && H.task_draft.input.granularity=="subtask" && H.task_draft.input.labels.len()==1 && H.task_draft.input.labels[0]=="one")
find("task_prepare").on_click()
assert(calls.len()==1 && calls[0].args.expectedRevision==7 && calls[0].args.priority=="p0")
'''
 CASES['detail_only_renders_current_legal_transitions']='''
H.page="operator_task"
let tree=View{'''+section(source,'operator_task')+'''} visit(tree)
let transitions=[] for row in widgets{if row.name=="task_transition"{transitions.push(row.value)}}
assert(transitions.len()==1 && find("task_edit")!=nil && find("task_delete")!=nil && find("task_comment")!=nil)
transitions[0].on_click()
assert(H.task_draft.operation=="transition" && H.task_draft.input.status=="in_progress" && H.task_draft.input.expectedRevision==7 && calls.len()==1 && calls[0].service=="hagency.tasks.transition")
'''
 CASES['saved_review_widgets_apply_only_original']='''
A.task_prepared({commandId:"saved_comment" taskId:"task_original" operation:"task_comment" change:{operation:"comment" input:{id:"task_original" expectedRevision:7 text:"Original comment"}} result:nil error:nil})
let tree=View{'''+section(source,'operator_task_command')+'''} visit(tree)
assert(find("task_command_apply")!=nil)
find("task_command_apply").on_click()
assert(calls.len()==1 && calls[0].service=="hagency.tasks.command.apply" && calls[0].args.commandId=="saved_comment")
'''
 CASES['list_starts_compact_and_expanded_filters_keep_callbacks']='''
H.page="operator_tasks" H.task_filters_open=false
let tree=View{'''+section(source,'operator_tasks')+'''} visit(tree)
assert(find("task_create")!=nil && find("task_empty")!=nil && find("task_assignee")==nil && find("task_status")==nil)
find("task_filters_toggle").on_click() assert(H.task_filters_open)
widgets=[]
let expanded=View{'''+section(source,'operator_tasks')+'''} visit(expanded)
let states=[] let priorities=[] for row in widgets{if row.name=="task_status"{states.push(row.value)} if row.name=="task_priority"{priorities.push(row.value)}}
assert(states.len()==6 && priorities.len()==5)
assert(find("task_assignee").empty_text!="Your text here" && find("task_label").empty_text!="Your text here")
find("task_assignee").on_change("Exact agent") find("task_label").on_change("Exact label") find("task_apply").on_click()
assert(calls[0].args.assignee=="Exact agent" && calls[0].args.label=="Exact label")
H.busy=false states[4].on_click() H.busy=false priorities[1].on_click()
assert(calls[1].args.status=="blocked" && calls[2].args.priority=="p0")
H.busy=false find("task_clear").on_click()
assert(H.task_filters.assignee=="" && H.task_filters.label=="" && H.task_filters.status=="" && H.task_filters.priority=="")
'''

 CASES['one_action_comment_uses_one_native_key_and_same_revision']='''
A.task_begin("comment","") A.task_set("text","A useful update") A.task_prepare()
assert(calls.len()==1 && calls[0].service=="hagency.tasks.comment" && calls[0].args.id=="task_original" && calls[0].args.expectedRevision==7)
replies[0]({is_ok:true data:{commandId:"comment_once" taskId:"task_original" operation:"task_comment" change:{operation:"comment" input:calls[0].args} result:nil error:nil}})
assert(calls.len()==2 && calls[1].service=="hagency.tasks.command.apply" && calls[1].args.commandId=="comment_once")
A.task_prepare() A.task_apply() A.task_begin("comment","") assert(calls.len()==2)
replies[1]({is_ok:false error:"management_outcome_unknown"})
A.task_begin("comment","") assert(calls.len()==2 && H.task_pending.commandId=="comment_once")
A.task_apply() assert(calls.len()==3 && calls[2].args.commandId=="comment_once")
'''
 CASES['delete_requires_explicit_button_and_recovery_never_auto_applies']='''
A.task_begin("delete","") assert(calls.len()==0 && H.page=="operator_task_edit")
let confirm=View{'''+section(source,'operator_task_edit')+'''} visit(confirm)
assert(find("task_prepare").text==A.t("Delete task","删除任务"))
find("task_edit_back").on_click() assert(calls.len()==1 && calls[0].service=="hagency.tasks.get")
replies[0]({supported:true writeSupported:true writeOperations:["delete"] revision:7 task:{id:"task_original" title:"Exact task"}})
A.task_begin("delete","") assert(calls.len()==1)
widgets=[] let again=View{'''+section(source,'operator_task_edit')+'''} visit(again)
find("task_prepare").on_click() assert(calls.len()==2 && calls[1].service=="hagency.tasks.delete" && calls[1].args.expectedRevision==7)
let saved={commandId:"delete_once" taskId:"task_original" operation:"task_delete" change:{operation:"delete" input:calls[1].args} result:nil error:nil}
replies[1]({is_ok:true data:saved}) assert(calls.len()==3 && calls[2].args.commandId=="delete_once")
replies[2]({is_ok:false error:"management_outcome_unknown"})
A.open_saved_task({commandId:"delete_once"}) assert(calls.len()==4)
replies[3](saved) assert(calls.len()==4 && H.task_phase=="saved")
'''
 CASES['editor_options_hidden_primary_early_and_metadata_hidden']='''
A.task_begin("create","") let editor=View{'''+section(source,'operator_task_edit')+'''} visit(editor)
assert(find("task_prepare")!=nil && find("task_edit_priority")==nil && find("task_parent_input")==nil)
let save_index=-1 let title_index=-1 for index row in widgets{if row.name=="task_prepare"{save_index=index} if row.name=="task_title_input"{title_index=index}}
assert(save_index>=0 && save_index<title_index)
find("task_options_toggle").on_click() assert(H.task_options_open)
H.task_phase="" H.page="operator_task" widgets=[]
let detail=View{'''+section(source,'operator_task')+'''} visit(detail)
assert(find("task_comments_first")==nil && find("task_details_toggle")!=nil)
H.page="operator_tasks" widgets=[] let list=View{'''+section(source,'operator_tasks')+'''} visit(list)
assert(find("task_first")==nil && find("task_back")==nil)
'''

 CASES['empty_board_has_five_counts_without_reset_or_metadata_dump']='''
H.page="operator_board" H.task_board_query={} H.task_board={supported:true projectCount:0 agentCount:0 taskTotals:{created:0 accepted:0 in_progress:0 blocked:0 done:0} projects:[] nextAfter:nil}
let tree=View{'''+section(source,'operator_board')+'''} visit(tree)
let counts=[] for row in widgets{if row.name=="board_counter"{counts.push(row.value)}}
assert(counts.len()==5 && find("board_first")==nil && find("board_back")==nil && find("board_details")!=nil)
find("board_details").on_click() assert(H.board_details_open)
'''
 results=[]
 for lang in ['en','zh-CN']:
  for name,test in CASES.items():
   path=a.output/f'{lang}-{name}.splash';path.write_text(fixture(source+'\nmod.task_writer_actions=A\n',lang)+SETUP+test+'\ntrue\n');result=subprocess.run([str(a.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);path.with_suffix('.log').write_text(result.stdout);results.append({'language':lang,'case':name,'passed':result.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
