#!/usr/bin/env python3
"""Shipped retirement callbacks in the pinned VM, separate from native pixels."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import fixture,APP
from check_account_enrollment import SETUP
CASES={
 'exact_original': '''A.retire_account(row)
assert(sent.len()==1 && sent[0].service=="hagency.accounts.retire" && sent[0].args.accountId==row.id && sent[0].args.expectedRevision==row.revision)
assert(H.account_models==nil)''',
 'busy_pending': '''H.busy=true A.retire_account(row) H.busy=false H.command={service:"other"} A.retire_account(row) assert(sent.len()==0)''',
 'bound_account': '''row.resourceCount=1 A.retire_account(row) assert(sent.len()==0)''',
 'retired_account': '''row.state="retired" A.retire_account(row) assert(sent.len()==0)''',
 'other_selection': '''H.account_id="other" A.retire_account(row) assert(sent.len()==0)''',
 'native_cancel': '''H.command={service:"hagency.accounts.retire" args:{commandId:"original"}} A.retry_command()
callback({is_ok:false error:"account_retirement_cancelled"})
assert(H.command==nil && !H.busy && sent.len()==2 && sent[1].service=="hagency.intent.pending")
callback({is_ok:true data:{commands:[{commandId:"original" operation:"retire_account" hasReply:false}]}})
assert(H.saved_commands.len()==1 && H.saved_commands[0].commandId=="original")''',
 'expired_review': '''H.command={service:"hagency.accounts.retire" args:{commandId:"original"}} A.retry_command()
callback({is_ok:false error:"provider_confirmation_expired"})
assert(H.command==nil && sent.len()==2 && sent[1].service=="hagency.intent.pending")''',
 'dependency_refusal': '''H.command={service:"hagency.accounts.retire" args:{commandId:"original"}} A.retry_command()
callback({is_ok:false error:"account_retirement_dependencies"})
assert(H.command==nil && H.notice==A.retirement_error("account_retirement_dependencies") && sent.len()==2 && sent[1].service=="hagency.intent.acknowledge")''',
 'revision_refusal': '''H.command={service:"hagency.accounts.retire" args:{commandId:"original"}} A.retry_command()
callback({is_ok:false error:"account_retirement_revision_changed"})
assert(H.command==nil && H.notice==A.retirement_error("account_retirement_revision_changed"))''',
 'unknown_keeps_original': '''let original={service:"hagency.accounts.retire" args:{commandId:"original"}} H.command=original A.retry_command()
callback({is_ok:false error:"management_outcome_unknown"}) assert(H.command==original && !H.busy && sent.len()==1)''',
 'success_reads_original': '''let opened="" A.open_account=fn(id,after){opened=id} A.acknowledge=fn(id){}
H.command={service:"hagency.accounts.retire" args:{commandId:"original"}} A.retry_command()
callback({is_ok:true data:{accountId:"account_original" state:"retired" providerLogout:"unobserved"}})
assert(H.command==nil && opened=="account_original" && H.page=="provider_account")''',
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);source=(APP/'bundle/main.splash').read_text();rows=[]
 for lang in ['en','zh-CN']:
  for name,code in CASES.items():
   f=a.output/f'{lang}-{name}.splash';f.write_text(fixture(source+'\nmod.enrollment_actions=A\nmod.enrollment_host=host\n',lang)+SETUP+code+'\ntrue\n')
   r=subprocess.run([str(a.vm.resolve()),'eval',str(f)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);f.with_suffix('.log').write_text(r.stdout);rows.append({'language':lang,'case':name,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps({'passed':sum(x['passed'] for x in rows),'total':len(rows)}));raise SystemExit(not all(x['passed'] for x in rows))
if __name__=='__main__':main()
