#!/usr/bin/env python3
"""Exercise shipped alert mutation/recovery against inert callbacks, not native effects."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import APP,fixture
p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
source=(APP/'bundle/main.splash').read_text();setup='''
let H=mod.hagency let A=mod.alert_actions let host=mod.alert_host
A.redraw=fn(){} A.save=fn(){return true} H.busy=false H.command=nil H.page="resource_alert" H.alert_resource="resource" H.alert_id="alert:original" H.alert_detail={alert:{id:"alert:original" resourceId:"resource" revision:"revision"}}
let sent=[] let callback=nil let opened=nil
host.request=fn(service,args,done){sent.push({service:service args:args}) callback=done}
A.open_alert=fn(resource,id,after){opened={resource:resource id:id after:after}}
let original={service:"hagency.alerts.triage" args:{commandId:"original" resourceId:"resource" id:"alert:original" expectedRevision:"revision" to:"assigned" note:nil}}
'''
cases={
'unknown_retained':'H.command=original A.retry_command() callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command==original && sent.len()==1 && H.alert_detail==nil)',
'cas_refusal':'H.command=original A.retry_command() callback({is_ok:false error:"management_revision_conflict"}) assert(H.command==nil && sent.len()==2 && sent[1].service=="hagency.intent.acknowledge" && H.notice==A.alert_error("management_revision_conflict"))',
'not_found':'H.command=original A.retry_command() callback({is_ok:false error:"management_not_found"}) assert(H.command==nil && sent.len()==2 && sent[1].args.commandId=="original")',
'success_refreshes_original':'H.command=original A.retry_command() callback({is_ok:true data:{resourceId:"resource" id:"alert:original" status:"assigned"}}) assert(H.command==nil && opened.resource=="resource" && opened.id=="alert:original" && opened.after==0)',
'saved_original':'A.recover_native({commandId:"original" operation:"triage_resource_alert"}) callback({is_ok:true data:{operation:"triage_resource_alert" data:{resourceId:"resource" id:"alert:original" status:"assigned"}}}) assert(H.page=="resource_alert" && opened.id=="alert:original" && sent[0].service=="hagency.intent.retry" && sent[0].args.commandId=="original")',
}
rows=[]
for lang in ['en','zh-CN']:
 for name,code in cases.items():
  f=a.output/f'{lang}-{name}.splash';f.write_text(fixture(source+'\nmod.alert_actions=A\nmod.alert_host=host\n',lang)+setup+code+'\ntrue\n');r=subprocess.run([str(a.vm.resolve()),'eval',str(f)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);f.with_suffix('.log').write_text(r.stdout);rows.append({'language':lang,'case':name,'passed':r.returncode==0})
(a.output/'report.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in rows),'total':len(rows)}));raise SystemExit(not all(r['passed'] for r in rows))
