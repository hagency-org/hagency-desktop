#!/usr/bin/env python3
"""Exercise the shipped selected-account callbacks in the actual pinned VM."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
SETUP='''
let H=mod.hagency let A=mod.account_actions let host=mod.account_host
A.redraw=fn(){} A.save=fn(){return true} H.busy=false H.command=nil H.page="provider_account" H.account_id="account_original"
let row={id:"account_original" kind:"managed_namespace" state:"active" resourceCount:0 revision:"revision"}
H.account_detail={account:row} let sent=[] let callback=nil
host.request=fn(service,args,done){sent.push({service:service args:args}) callback=done}
let reply={protocol:"hagency-management-v1" accountId:"account_original" revision:"revision" provider:{runtime:"codex" connected:true models:[{model:"original-model" label:"Original" sharedReasoning:["high"]}]}}
'''
CASES={
"original_selection":'''
A.load_account_models(row) assert(sent.len()==1 && sent[0].service=="hagency.accounts.models")
assert(sent[0].args.accountId==row.id && sent[0].args.expectedRevision==row.revision)
callback({is_ok:true data:reply}) assert(H.account_models==reply && !H.busy && H.command==nil)
''',
"guard_pending_and_other_kind":'''
H.command={service:"other"} A.load_account_models(row) assert(sent.len()==0)
H.command=nil row.kind="resource_binding" A.load_account_models(row) assert(sent.len()==0)
row.kind="managed_namespace" row.state="retired" A.load_account_models(row) assert(sent.len()==0)
row.state="active" H.account_id="different" A.load_account_models(row) assert(sent.len()==0)
''',
"signed_out_is_not_empty_catalog":'''
A.load_account_models(row) callback({is_ok:false error:"provider_connection_required"})
assert(H.account_models==nil && H.account_models_attempted && !H.busy && H.command==nil)
assert(H.notice==A.t("Sign in to this account before checking its models.","请先登录此账号，再检查其可用模型。"))
''',
"late_reply":'''
A.load_account_models(row) H.account_id="other" callback({is_ok:true data:reply}) assert(H.account_models==nil)
H.account_id=row.id A.load_account_models(row) H.page="overview" callback({is_ok:false error:"provider_connection_required"}) assert(H.notice=="")
''',
"read_failure_is_not_saved_command":'''
A.load_account_models(row) callback({is_ok:false error:"management_unavailable"})
assert(H.command==nil && H.notice==A.t("Models could not be read from this account. Check its connection and try again.","无法读取此账号的模型，请检查连接后重试。"))
''',
"wrong_revision":'''
A.load_account_models(row) reply.revision="changed" callback({is_ok:true data:reply}) assert(H.account_models==nil && H.notice!="")
''',
"wrong_account":'''
A.load_account_models(row) reply.accountId="different" callback({is_ok:true data:reply}) assert(H.account_models==nil && H.notice!="")
''',
"empty_success_and_refresh":'''
A.load_account_models(row) reply.provider.models=[] callback({is_ok:true data:reply}) assert(H.account_models.provider.models.len()==0 && H.notice=="")
A.open_account(row.id,nil) assert(H.account_models==nil && !H.account_models_attempted)
'''
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);source=(APP/'bundle/main.splash').read_text();results=[]
 for language in ['en','zh-CN']:
  for name,code in CASES.items():
   script=fixture(source+'\nmod.account_actions=A\nmod.account_host=host\n',language)+SETUP+code+'\ntrue\n';f=a.output/f'{language}-{name}.splash';f.write_text(script);r=subprocess.run([str(a.vm.resolve()),'eval',str(f)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);f.with_suffix('.log').write_text(r.stdout);results.append({'language':language,'case':name,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps({'passed':sum(x['passed'] for x in results),'total':len(results)}));raise SystemExit(not all(x['passed'] for x in results))
if __name__=='__main__':main()
