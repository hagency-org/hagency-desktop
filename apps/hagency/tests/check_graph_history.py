#!/usr/bin/env python3
"""Shipped graph-history navigation, retained targets and states in the VM.

Inert widgets verify callbacks and EN/CN construction. Pixel review is separate.
"""
import argparse, json, subprocess
from pathlib import Path
from check_full_bundle import fixture, APP

SETUP = '''
let H=mod.hagency let A=mod.graph_actions
A.redraw=fn(){return nil} H.busy=false H.ready=true H.notice=""
let original={service:"original" args:{commandId:"preserve"}} H.command=original
let calls=[] let callback=nil A.request=fn(service,args,done){calls.push({service:service args:args}) callback=done}
H.graph_archive="archive-original" H.graph_id="graph-original" H.graph_node_id="second"
let graph={id:"graph-original" label:"Original graph" owner:"Recorded owner" status:"active" nodeCount:2 completeCount:1}
let row={commandId:"archive-original" state:"archived" history:{graphs:1 nodes:2}}
let inventory={supported:true readOnly:true identityMapped:false archives:[row]}
let page={archiveId:"archive-original" graphs:[graph] nextAfter:"graph-original"}
let detail={archiveId:"archive-original" graph:graph nodes:[{id:"first" assignee:"Recorded agent" status:"complete" dependencyCount:0},{id:"second" assignee:"Recorded agent" status:"pending" dependencyCount:1}] nextAfter:nil}
let node={archiveId:"archive-original" graphId:"graph-original" id:"second" assignee:"Recorded agent" description:"Original description" status:"pending" dependsOn:["first"] resultRecorded:false conditionRecorded:false}
let controls={graph_back:nil graph_refresh:nil graph_archive_open:nil graph_open:nil graph_first:nil graph_next:nil graph_node_open:nil graph_dependency:nil graph_dependencies_previous:nil graph_dependencies_next:nil} let texts=[] let dependency_count=0
let visit=fn(n){if !n.is_object(){return} for i key child in n{if key.to_string()=="graph_dependency"{dependency_count=dependency_count+1}if key.to_string()=="text" && child.is_string(){texts.push(child)} for wanted in ["graph_back","graph_refresh","graph_archive_open","graph_open","graph_first","graph_next","graph_node_open","graph_dependency","graph_dependencies_previous","graph_dependencies_next"]{if key.to_string()==wanted{controls[key]=child}} visit(child)}}
let has=fn(text){for t in texts{if t==text{return true}} return false}
'''

CASES = {
 'archives_open': ('A.open_graph_archives() callback(inventory)', 'assert(H.graph_archives==inventory && calls[0].service=="hagency.graph_history.archives")'),
 'archives_unavailable': ('H.page="graph_archives"', 'assert(has(A.t("History unavailable. Refresh to try again.","历史暂不可用，请刷新重试。"))) assert(controls.graph_archive_open==nil)'),
 'archives_loading': ('H.page="graph_archives" H.busy=true', 'assert(has(A.t("Reading history…","正在读取历史…"))) assert(!controls.graph_refresh.enabled)'),
 'archives_empty': ('H.page="graph_archives" H.graph_archives=inventory inventory.archives=[]', 'assert(has(A.t("No graph archives in this account","此账号暂无任务图归档")))'),
 'archives_unsupported': ('H.page="graph_archives" H.graph_archives=inventory inventory.supported=false inventory.archives=[]', 'assert(has(A.t("Graph history has not been transferred to this device profile.","任务图历史尚未迁移至此设备档案。"))) assert(!has(A.t("No graph archives in this account","此账号暂无任务图归档")))'),
 'copy_not_readable': ('H.page="graph_archives" H.graph_archives=inventory row.state="copying"', 'assert(controls.graph_archive_open==nil && has(A.graph_state("copying")))'),
 'sealed_open': ('H.page="graph_archives" H.graph_archives=inventory', 'controls.graph_archive_open.on_click() assert(calls[0].service=="hagency.graph_history.list" && calls[0].args.archiveId=="archive-original")'),
 'graph_original_target': ('H.page="graph_history" H.graph_page=page', 'controls.graph_open.on_click() assert(calls[0].service=="hagency.graph_history.get" && calls[0].args.id=="graph-original")'),
 'graph_pagination': ('H.page="graph_history" H.graph_page=page', 'controls.graph_next.on_click() assert(calls[0].args.after=="graph-original" && calls[0].args.archiveId=="archive-original")'),
 'graph_first': ('H.page="graph_history" H.graph_page=page H.graph_after="cursor"', 'controls.graph_first.on_click() assert(calls[0].args.after==nil)'),
 'graph_loading': ('H.page="graph_history" H.busy=true', 'assert(has(A.t("Reading graphs…","正在读取任务图…"))) assert(!controls.graph_refresh.enabled)'),
 'graph_empty': ('H.page="graph_history" H.graph_page=page page.graphs=[]', 'assert(has(A.t("No graphs on this page","本页暂无任务图"))) assert(controls.graph_open==nil)'),
 'nodes_original_target': ('H.page="history_graph" H.graph_detail=detail', 'controls.graph_node_open.on_click() assert(calls[0].service=="hagency.graph_history.node" && calls[0].args.nodeId=="second")'),
 'nodes_back_preserves_page': ('H.page="history_graph" H.graph_detail=detail H.graph_after="cursor"', 'controls.graph_back.on_click() assert(calls[0].args.after=="cursor")'),
 'node_dependency': ('H.page="history_node" H.graph_node=node', 'controls.graph_dependency.on_click() assert(calls[0].args.nodeId=="first" && calls[0].args.id=="graph-original" && calls[0].args.archiveId=="archive-original")'),
 'node_back_preserves_page': ('H.page="history_node" H.graph_node=node H.graph_node_after="node-cursor"', 'controls.graph_back.on_click() assert(calls[0].args.after=="node-cursor")'),
 'node_empty_dependencies': ('H.page="history_node" H.graph_node=node node.dependsOn=[]', 'assert(has(A.t("No dependencies","无依赖"))) assert(controls.graph_dependency==nil)'),
 'node_loading': ('H.page="history_node" H.busy=true', 'assert(has(A.t("Reading node…","正在读取节点…"))) assert(!controls.graph_back.enabled)'),
 'wrong_archive_reply': ('A.open_graph_history("archive-original",nil) page.archiveId="wrong" callback(page)', 'assert(H.graph_page==nil && H.notice!="")'),
 'wrong_graph_reply': ('A.open_history_graph("archive-original","graph-original",nil) detail.graph.id="wrong" callback(detail)', 'assert(H.graph_detail==nil && H.notice!="")'),
 'wrong_node_reply': ('A.open_history_node("archive-original","graph-original","second") node.id="wrong" callback(node)', 'assert(H.graph_node==nil && H.notice!="")'),
 'stale_reply': ('A.open_history_node("archive-original","graph-original","second") H.page="overview" callback(node)', 'assert(H.graph_node==nil && H.page=="overview")'),
 'dependencies_bounded_first': ('H.page="history_node" H.graph_node=node node.dependsOn=[] for i in 0..20 {node.dependsOn.push("node_"+i.to_string())}', 'assert(dependency_count==16) controls.graph_dependencies_next.on_click() assert(H.graph_dep_offset==16 && calls.len()==0)'),
 'dependencies_bounded_next': ('H.page="history_node" H.graph_node=node H.graph_dep_offset=16 node.dependsOn=[] for i in 0..20 {node.dependsOn.push("node_"+i.to_string())}', 'assert(dependency_count==4 && controls.graph_dependencies_next==nil) controls.graph_dependencies_previous.on_click() assert(H.graph_dep_offset==0 && calls.len()==0)'),
 'busy_ignores_navigation': ('H.busy=true A.open_graph_archives() A.open_graph_history("archive-original",nil) A.open_history_graph("archive-original","graph-original",nil) A.open_history_node("archive-original","graph-original","first")', 'assert(calls.len()==0)'),
}

def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
 source=(APP/'bundle/main.splash').read_text()
 start=source.index('        if H.page=="graph_archives" {');end=source.index('        if H.page == "operator_tasks" {',start);pages=source[start:end]
 results=[]
 for language in ['en','zh-CN']:
  prefix=fixture(source+'\nmod.graph_actions=A\n',language)
  for name,(setup,check) in CASES.items():
   script=a.output/f'{language}-{name}.splash';script.write_text(prefix+SETUP+setup+'\nlet root=View{\n'+pages+'\n}\nvisit(root)\n'+check+'\nassert(H.command==original)\ntrue\n')
   r=subprocess.run([str(a.vm.resolve()),'eval',str(script)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);script.with_suffix('.log').write_text(r.stdout)
   results.append({'case':name,'language':language,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
