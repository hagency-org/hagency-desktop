#!/usr/bin/env python3
"""Execute shipped enrollment callbacks in the pinned VM; fixtures are not provider proof."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
SETUP='''
let H=mod.hagency let A=mod.enrollment_actions let host=mod.enrollment_host
A.redraw=fn(){} A.save=fn(){return true} H.busy=false H.command=nil H.page="provider_account" H.account_id="account_original"
let row={id:"account_original" kind:"managed_namespace" state:"active" resourceCount:0 revision:"revision"}
let model={id:"catalog-alias" model:"gpt-5.6-sol" label:"Coding" sharedReasoning:["medium","high"] sharedDefaultReasoning:"medium"}
H.account_detail={account:row} H.account_models={accountId:row.id revision:row.revision enrollmentReady:true provider:{runtime:"codex" connected:true models:[model]}}
let sent=[] let callback=nil
host.request=fn(service,args,done){sent.push({service:service args:args}) callback=done}
A.submit=fn(service,args){sent.push({service:service args:args})}
'''
CASES={
 'current_codex_models_keep_exact_selection': '''
for name in ["gpt-6.1-sol","gpt-6-astra"]{
 model.model=name model.sharedReasoning=["low","medium","high","xhigh","max"] model.sharedDefaultReasoning="medium"
 for effort in model.sharedReasoning{
  H.page="provider_account" A.choose_account_model(row,model)
  assert(H.enrollment_draft.model.model==name && H.enrollment_draft.reasoning=="medium")
  H.enrollment_draft.tokens="12000" H.enrollment_draft.reasoning=effort A.enroll_account()
  let request=sent[sent.len()-1]
  assert(request.service=="hagency.accounts.enroll" && request.args.model==name && request.args.reasoning==effort)
  assert(request.args.account.accountId==row.id && request.args.account.expectedRevision==row.revision)
  assert(request.args.monthlyTokens==12000)
 }
}
assert(sent.len()==10)
''',
 'device_setup_required': '''H.account_models.enrollmentReady=false A.choose_account_model(row,model) assert(H.enrollment_draft==nil && sent.len()==0 && H.notice!="")''',
 'exact_account_model_limit': '''A.choose_account_model(row,model) assert(H.page=="account_enrollment" && sent.len()==0)
H.enrollment_draft.tokens=" 12000 " H.enrollment_draft.reasoning="high" A.enroll_account()
assert(sent.len()==1 && sent[0].service=="hagency.accounts.enroll")
assert(sent[0].args.account.accountId==row.id && sent[0].args.account.expectedRevision==row.revision)
assert(sent[0].args.model=="gpt-5.6-sol" && sent[0].args.monthlyTokens==12000 && sent[0].args.reasoning=="high")''',
 'changed_catalog_revision': '''H.account_models.revision="changed" A.choose_account_model(row,model) assert(H.enrollment_draft==nil && H.page=="provider_account")''',
 'other_account': '''H.account_models.accountId="other" A.choose_account_model(row,model) assert(H.enrollment_draft==nil)''',
 'unsupported_model': '''model.sharedReasoning=[] A.choose_account_model(row,model) assert(H.enrollment_draft==nil)''',
 'pending_or_busy': '''H.busy=true A.choose_account_model(row,model) assert(H.enrollment_draft==nil)
H.busy=false H.command={service:"original"} A.choose_account_model(row,model) assert(H.enrollment_draft==nil)''',
 'invalid_limits': '''A.choose_account_model(row,model)
for text in ["","0","-1","1.5","9007199254740992","unknown"]{H.enrollment_draft.tokens=text A.enroll_account()}
assert(sent.len()==0 && H.notice!="")''',
 'invalid_reasoning': '''A.choose_account_model(row,model) H.enrollment_draft.reasoning="unsupported" A.enroll_account() assert(sent.len()==0 && H.notice==A.error_text("management_model_unqualified"))''',
 'native_cancel': '''H.command={service:"hagency.accounts.enroll" args:{commandId:"original"}} A.retry_command() assert(sent.len()==1)
callback({is_ok:false error:"resource_enrollment_cancelled"}) assert(H.command==nil && !H.busy && sent.len()==2 && sent[1].service=="hagency.intent.pending")
assert(H.notice==A.t("Resource review closed. Existing saved requests are unchanged.","已关闭资源确认，已有的保存请求保持不变。"))
callback({is_ok:true data:{commands:[{commandId:"original" operation:"enroll_account_resource" hasReply:false}]}})
assert(H.saved_commands.len()==1 && H.saved_commands[0].commandId=="original")''',
 'unsupported_quota_not_retry_loop': '''H.command={service:"hagency.accounts.enroll" args:{commandId:"original"}} A.retry_command()
callback({is_ok:false error:"provider_identity_unavailable"}) assert(H.command==nil && !H.busy && sent.len()==2 && sent[1].service=="hagency.intent.acknowledge")
assert(H.notice==A.enrollment_error("provider_identity_unavailable"))''',
 'unknown_retains_original': '''let original={service:"hagency.accounts.enroll" args:{commandId:"original"}} H.command=original A.retry_command()
callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command==original && !H.busy && sent.len()==1)''',
 'saved_receipt_opens_resource': '''let opened="" A.open_resource=fn(id){opened=id} A.acknowledge=fn(id){}
H.command={service:"hagency.accounts.enroll" args:{commandId:"original"}} A.retry_command()
callback({is_ok:true data:{resourceId:"resource_original"}}) assert(H.command==nil && H.enrollment_draft==nil && opened=="resource_original")'''
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);source=(APP/'bundle/main.splash').read_text();rows=[]
 for lang in ['en','zh-CN']:
  for name,code in CASES.items():
   f=a.output/f'{lang}-{name}.splash';f.write_text(fixture(source+'\nmod.enrollment_actions=A\nmod.enrollment_host=host\n',lang)+SETUP+code+'\ntrue\n')
   r=subprocess.run([str(a.vm.resolve()),'eval',str(f)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);f.with_suffix('.log').write_text(r.stdout);rows.append({'language':lang,'case':name,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps({'passed':sum(x['passed'] for x in rows),'total':len(rows)}));raise SystemExit(not all(x['passed'] for x in rows))
if __name__=='__main__':main()
