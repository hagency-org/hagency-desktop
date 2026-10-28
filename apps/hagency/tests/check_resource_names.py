#!/usr/bin/env python3
"""Full-source local naming callbacks, original custody and latest projection.

No live host or pixels: native SQLite tests qualify CAS/receipt enforcement.
"""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
SETUP='''
let H=mod.hagency
let A=mod.name_actions
A.redraw=fn(){return nil}
A.save=fn(){return true}
H.busy=false H.command=nil
H.session={account:"@owner:test"}
H.resource_id="resource_original"
H.resource_detail={protocol:"hagency-management-v1" editing:{allowed:false reason:"contribution_history_not_proven"} resource:{id:"resource_original" model:"Exact model" localName:{name:"Local original" revision:3}}}
let sent=[]
A.submit=fn(service,args){sent.push({service:service args:args})}
'''
CASES={
'edit_review_and_exact_cas': '''
let original=H.resource_detail.to_json()
A.edit_resource_name()
assert(H.resource_name_mode == "edit" && H.resource_name_draft.expectedNameRevision == 3)
H.resource_name_draft.name="  写作资源  "
A.submit_resource_name()
assert(sent.len() == 0)
A.review_resource_name()
assert(H.resource_name_mode == "review" && H.resource_name_draft.name == "写作资源")
assert(sent.len() == 0)
A.submit_resource_name()
assert(sent.len() == 1 && sent[0].service == "hagency.resources.rename")
assert(sent[0].args.resourceId == "resource_original" && sent[0].args.expectedNameRevision == 3 && sent[0].args.name == "写作资源")
assert(H.resource_detail.to_json() == original)
assert(A.resource_title(H.resource_detail.resource) == "Local original")
''',
'legacy_clear_and_stale': '''
assert(A.local_name({model:"Original"}) == nil)
assert(A.resource_title({model:"Original"}) == "Original")
A.edit_resource_name()
H.resource_name_draft.name=""
A.review_resource_name()
A.submit_resource_name()
assert(sent.len() == 1 && sent[0].args.name == "")
H.resource_detail.resource.localName.revision=4
A.submit_resource_name()
assert(sent.len() == 1 && H.resource_name_draft == nil && H.resource_name_mode == "")
H.resource_detail.resource={id:"legacy" model:"Original"}
A.edit_resource_name()
assert(H.resource_name_draft == nil)
''',
'unknown_and_conflicts_retain_original': '''
let command={service:"hagency.resources.rename" args:{commandId:"original_command" resourceId:"resource_original" expectedNameRevision:3 name:"Requested"}}
let acks=[]
A.acknowledge=fn(id){acks.push(id)}
for code in ["management_outcome_unknown","management_scope_changed","management_revision_conflict","management_command_conflict"] {
 H.command=command
 host.request=fn(service,args,done){assert(service == command.service && args.commandId == "original_command") done({is_ok:false error:code})}
 A.retry_command()
 assert(H.command == command && acks.len() == 0)
}
host.request=fn(service,args,done){done({is_ok:false error:"management_resource_name_changed"})}
A.retry_command()
assert(H.command == nil && acks.len() == 1 && acks[0] == "original_command")
''',
'original_receipt_reads_latest_name': '''
let reads=[]
let acks=[]
A.acknowledge=fn(id){acks.push(id)}
A.request=fn(service,args,done){
 reads.push({service:service args:args})
 if service == "hagency.intent.retry" {done({operation:"rename_resource" data:{resourceId:"resource_original" localName:{name:"Old receipt" revision:1}}})}
 else {assert(service == "hagency.resources.get") done({protocol:"hagency-management-v1" resource:{id:"resource_original" model:"Exact model" localName:{name:"Latest actual" revision:4}}})}
}
A.recover_native({commandId:"original_command" operation:"rename_resource"})
assert(reads.len() == 2 && reads[0].args.commandId == "original_command" && reads[1].args.id == "resource_original")
assert(H.resource_detail.resource.localName.name == "Latest actual" && H.resource_detail.resource.localName.revision == 4)
assert(acks.len() == 1 && acks[0] == "original_command")
'''
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 source=a.bundle.read_text();results=[]
 start=source.index('        if H.page == "resource_detail" {mod.widgets.HagencyDetailCard{')
 end=source.index('\n        if H.page ==',start)
 section=source[start:end]
 CASES['actual_detail_controls']='\n'.join([
  'H.page="resource_detail"',
  'H.resource_detail.resource.framework="codex" H.resource_detail.resource.reasoning=nil H.resource_detail.resource.capabilities=["coding"] H.resource_detail.resource.published=true',
  'H.resource_detail.resource.budget={remainingTokens:100 pool:{ceiling:100 committed:0 period:"monthly"}}',
  'H.resource_detail.resource.usage={spent:nil spend_period_key:nil}',
  'let ids=[] let visit=fn(node){if !node.is_object(){return} for i key child in node {if key.to_string() == "resource_name_edit" || key.to_string() == "resource_name_input" || key.to_string() == "resource_name_cancel" || key.to_string() == "resource_name_review" || key.to_string() == "resource_name_save" {ids.push(key.to_string())} visit(child)}}',
  'let root=View{'+section+'} visit(root) assert(ids.len() == 1 && ids[0] == "resource_name_edit")',
  'A.edit_resource_name() ids=[] root=View{'+section+'} visit(root) assert(ids.len() == 3)',
  'A.review_resource_name() ids=[] root=View{'+section+'} visit(root) assert(ids.len() == 2)',
  'H.resource_name_mode="" H.resource_name_draft=nil H.resource_detail.resource.localName=nil ids=[] root=View{'+section+'} visit(root) assert(ids.len() == 0)',
 ])
 for lang in ['en','zh-CN']:
  for name,test in CASES.items():
   # Export existing scoped objects; no production callback is replaced except
   # the host boundary/drawing/storage seams that would need a live application.
   script=fixture(source+'\nmod.name_actions=A\nmod.name_host=host\n',lang)+'\nlet host=mod.name_host\n'+SETUP+test+'\ntrue\n'
   path=a.output/f'{lang}-{name}.splash';path.write_text(script)
   r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);path.with_suffix('.log').write_text(r.stdout);results.append({'language':lang,'case':name,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
