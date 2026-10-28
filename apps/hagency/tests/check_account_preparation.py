#!/usr/bin/env python3
"""Original shipped preparation callbacks; host IO is replaced, not native acceptance."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
CASES={
'review_and_original_submit': '''
H.page="provider_accounts"
let sent=[]
A.submit=fn(service,args){sent.push({service:service args:args})}
A.submit_account_preparation() assert(sent.len()==0)
A.review_account_preparation() assert(H.page=="account_prepare" && sent.len()==0)
A.submit_account_preparation() assert(sent.len()==1 && sent[0].service=="hagency.accounts.prepare")
H.command={service:"hagency.accounts.prepare" args:{commandId:"original"}}
A.submit_account_preparation() assert(sent.len()==1)
''',
'uncertain_keeps_original_command': '''
let original={service:"hagency.accounts.prepare" args:{commandId:"original"}}
for code in ["management_outcome_unknown","management_scope_changed","management_revision_conflict","management_command_conflict","management_capacity"] {
 H.command=original
 host.request=fn(service,args,done){assert(service==original.service && args.commandId=="original") done({is_ok:false error:code})}
 A.retry_command() assert(H.command==original && acks.len()==0)
}
''',
'success_observes_exact_account': '''
H.command={service:"hagency.accounts.prepare" args:{commandId:"original"}}
let opened=[]
A.open_account=fn(id,after){opened.push(id)}
host.request=fn(service,args,done){done({is_ok:true data:{accountId:"account_original" state:"active"}})}
A.retry_command()
assert(H.command==nil && H.page=="provider_account" && opened.len()==1 && opened[0]=="account_original")
assert(acks.len()==1 && acks[0]=="original")
''',
'recovered_uncertain_reads_instead_of_recreating': '''
let calls=[]
A.request=fn(service,args,done){calls.push({service:service args:args}) if service=="hagency.intent.retry"{done({operation:"prepare_account" data:{accountId:"account_original" state:"uncertain"}})}}
A.recover_native({commandId:"original" operation:"prepare_account"})
assert(calls.len()==2 && calls[0].service=="hagency.intent.retry" && calls[0].args.commandId=="original")
assert(calls[1].service=="hagency.accounts.get" && calls[1].args.id=="account_original")
assert(acks.len()==1 && acks[0]=="original")
'''
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 source=(APP/'bundle/main.splash').read_text();results=[]
 for lang in ['en','zh-CN']:
  for name,code in CASES.items():
   script=fixture(source+'\nmod.account_actions=A\nmod.account_host=host\n',lang)+'''\nlet H=mod.hagency let A=mod.account_actions let host=mod.account_host
A.redraw=fn(){} A.save=fn(){return true}
H.busy=false H.command=nil
let acks=[] A.acknowledge=fn(id){acks.push(id)}
'''+code+'\ntrue\n'
   path=a.output/f'{lang}-{name}.splash';path.write_text(script);r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);path.with_suffix('.log').write_text(r.stdout);results.append({'language':lang,'case':name,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
