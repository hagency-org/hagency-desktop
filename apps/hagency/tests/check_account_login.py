#!/usr/bin/env python3
"""Actual shipped callbacks and closed selections; offline VM, not native authorization."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
CASES={
"selection":'''
let sent=[] A.submit=fn(service,args){sent.push({service:service args:args})}
let row={id:"account_original" kind:"managed_namespace" state:"active" resourceCount:0 revision:"original_revision"}
A.login_account(row) assert(sent.len()==1 && sent[0].service=="hagency.accounts.login" && sent[0].args.accountId==row.id && sent[0].args.expectedRevision==row.revision)
row.kind="resource_binding" A.login_account(row) assert(sent.len()==1)
row.kind="managed_namespace" row.resourceCount=1 A.login_account(row) assert(sent.len()==1)
row.resourceCount=0 row.state="uncertain" A.login_account(row) assert(sent.len()==1)
''',
"cancel_before_start":'''
H.command={service:"hagency.accounts.login" args:{commandId:"original" accountId:"account_original"}}
let opened=[] A.open_account=fn(id,after){opened.push(id)}
host.request=fn(service,args,done){done({is_ok:true data:{commandId:"original" accountId:"account_original" state:"cancelled" recorded:false}})}
A.retry_command() assert(H.command==nil && opened.len()==1 && acks.len()==0)
''',
"uncertain_error_retains_original":'''
let original={service:"hagency.accounts.login" args:{commandId:"original" accountId:"account_original"}}
for code in ["management_scope_changed","management_outcome_unknown","management_revision_conflict","management_command_conflict"] {
 H.command=original host.request=fn(service,args,done){assert(args.commandId=="original") done({is_ok:false error:code})}
 A.retry_command() assert(H.command==original && acks.len()==0)
}
''',
"completed_exact_account":'''
H.command={service:"hagency.accounts.login" args:{commandId:"original" accountId:"account_original"}}
let opened=[] A.open_account=fn(id,after){opened.push(id)}
host.request=fn(service,args,done){done({is_ok:true data:{commandId:"original" accountId:"account_original" state:"authenticated" recorded:true}})}
A.retry_command() assert(H.command==nil && opened[0]=="account_original" && acks.len()==1 && acks[0]=="original")
''',
"resume_pending_keeps_original":'''
let args={commandId:"original" accountId:"account_original" expectedRevision:"rev"}
let opened=[] A.open_account=fn(id,after){opened.push(id)}
A.request=fn(service,input,done){assert(service=="hagency.intent.retry") done({operation:"login_account" data:{commandId:"original" accountId:"account_original" state:"awaiting_authorization"}})}
A.recover_native({commandId:"original" operation:"login_account" input:args})
assert(H.command.service=="hagency.accounts.login" && H.command.args==args && acks.len()==0 && opened[0]=="account_original")
'''
}
def main():
 p=argparse.ArgumentParser();p.add_argument("vm",type=Path);p.add_argument("--output",type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);source=(APP/"bundle/main.splash").read_text();results=[]
 for lang in ["en","zh-CN"]:
  for name,code in CASES.items():
   script=fixture(source+"\nmod.account_actions=A\nmod.account_host=host\n",lang)+"\nlet H=mod.hagency let A=mod.account_actions let host=mod.account_host\nA.redraw=fn(){} A.save=fn(){return true} H.busy=false H.command=nil let acks=[] A.acknowledge=fn(id){acks.push(id)}\n"+code+"\ntrue\n"
   f=a.output/f"{lang}-{name}.splash";f.write_text(script);r=subprocess.run([str(a.vm.resolve()),"eval",str(f)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);f.with_suffix(".log").write_text(r.stdout);results.append({"language":lang,"case":name,"passed":r.returncode==0})
 (a.output/"report.json").write_text(json.dumps(results,indent=2)+"\n");print(json.dumps({"passed":sum(x["passed"] for x in results),"total":len(results)}));raise SystemExit(not all(x["passed"] for x in results))
if __name__=="__main__":main()
