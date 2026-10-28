#!/usr/bin/env python3
"""Provider identity grouping, pagination and exact login action targets in the shipped VM."""
import argparse
import json
from pathlib import Path
import subprocess
from check_full_bundle import APP, fixture

SETUP = '''
A.redraw=fn(){} A.save=fn(){return true}
H.ready=true H.busy=false H.command=nil H.calls=[] H.callbacks=[]
host.request=fn(service,args,done){H.calls.push({service:service args:args}) H.callbacks.push(done)}
let record=fn(id,key,count){return {id:id identityKey:key resourceCount:count runtime:"codex" kind:"managed_namespace" state:"active" readiness:"unknown" revision:"original-revision" ordinal:1}}
let page=fn(rows,next){return {protocol:"hagency-management-v1" accounts:rows account:nil resources:[] nextAfter:next}}
'''
CASES = {
    'identity_not_runtime_or_readiness': '''
let a=record("login_a","identity_one",1) let b=record("login_b","identity_one",2)
b.readiness="subscription"
let c=record("login_c","identity_two",1) let unknown=record("unused",nil,0)
unknown.readiness="subscription"
H.account_inventory=page([a,b,c,unknown],nil)
let groups=A.account_groups()
assert(groups.known.len()==2 && groups.unconfirmed.len()==1)
assert(groups.known[0].members.len()==2 && groups.known[0].resources==3)
assert(groups.known[0].members[0].readiness=="unknown")
assert(groups.known[0].members[1].readiness=="subscription")
assert(groups.unconfirmed[0].id=="unused")
''',
    'missing_identity_compatible_and_distinct': '''
H.account_inventory=page([{id:"old" runtime:"codex" kind:"resource_binding" resourceCount:1},record("new",nil,0)],nil)
let groups=A.account_groups()
assert(groups.known.len()==0 && groups.unconfirmed.len()==2 && H.calls.len()==0)
''',
    'groups_across_pages_only_when_complete': '''
A.open_accounts(nil) assert(H.busy && H.calls.len()==1)
H.callbacks[0]({is_ok:true data:page([record("a","same",1)],"a")})
assert(H.busy && H.account_inventory==nil && H.calls.len()==2 && H.calls[1].args.after=="a")
H.callbacks[1]({is_ok:true data:page([record("b","same",2)],nil)})
assert(!H.busy && H.account_inventory.accounts.len()==2)
let groups=A.account_groups() assert(groups.known.len()==1 && groups.known[0].resources==3)
''',
    'failed_or_repeating_page_never_shows_partial_account_count': '''
A.open_accounts(nil)
H.callbacks[0]({is_ok:true data:page([record("a","same",1)],"a")})
H.callbacks[1]({is_ok:true data:page([record("a","same",1)],"a")})
assert(!H.busy && H.account_inventory==nil && H.notice!="" && H.calls.len()==2)
A.open_accounts(nil)
H.callbacks[2]({is_ok:false error:"native_resume_revoked"})
assert(H.needs_signin && H.account_inventory==nil)
''',
    'stale_reply_does_not_replace_newer_inventory': '''
A.open_accounts(nil) let epoch=H.account_list_epoch
H.page="resources"
H.callbacks[0]({is_ok:true data:page([record("stale","old",1)],nil)})
assert(H.account_inventory==nil)
A.open_accounts(nil) assert(H.account_list_epoch>epoch)
H.callbacks[1]({is_ok:true data:page([record("current","new",1)],nil)})
H.callbacks[0]({is_ok:true data:page([record("stale","old",1)],nil)})
assert(H.account_inventory.accounts[0].id=="current")
''',
    'group_actions_keep_original_login_and_revision': '''
let chosen=record("login_b","same",0)
A.open_account(chosen.id,nil)
assert(H.calls[0].service=="hagency.accounts.get" && H.calls[0].args.id=="login_b")
let response=page([],nil) response.account=chosen
H.callbacks[0]({is_ok:true data:response})
assert(H.account_detail.account.id=="login_b")
A.submit=fn(service,args){H.submission={service:service args:args}}
A.login_account(chosen)
assert(H.submission.service=="hagency.accounts.login")
assert(H.submission.args.accountId=="login_b" && H.submission.args.expectedRevision=="original-revision")
''',
    'automatic_refresh_keeps_complete_groups': '''
H.page="provider_accounts" H.account_inventory=page([record("kept","same",1)],nil)
let plan=A.web_refresh_plan() assert(plan.service=="hagency.accounts.list")
plan.done(page([record("a","same",1)],"a"))
assert(H.account_inventory==nil && H.calls[0].service=="hagency.accounts.list")
H.callbacks[0]({is_ok:true data:page([record("a","same",1)],"a")})
H.callbacks[1]({is_ok:true data:page([record("b","same",1)],nil)})
assert(H.account_inventory.accounts.len()==2 && A.account_groups().known.len()==1)
''',
    'view_preserves_saved_operation_and_resource_journey': '''
H.command={service:"hagency.accounts.login" args:{commandId:"original-request" accountId:"login_b"}}
H.ux.accountId="login_b" H.ux.resourceJourney=true
let original=H.command.to_json()
A.open_accounts(nil)
H.callbacks[0]({is_ok:true data:page([record("a","same",1)],nil)})
let viewed=A.account_groups()
assert(H.command.to_json()==original && H.ux.accountId=="login_b" && H.ux.resourceJourney)
assert(H.calls.len()==1 && H.calls[0].service=="hagency.accounts.list")
''',
}

def main():
    p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    a.output.mkdir(parents=True,exist_ok=True);source=(APP/'bundle/main.splash').read_text();results=[]
    for language in ('en','zh-CN'):
        for name,case in CASES.items():
            path=a.output/f'{language}-{name}.splash';path.write_text(fixture(source+SETUP+case,language))
            r=subprocess.run([str(a.vm.resolve()),'root',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
            path.with_suffix('.log').write_text(r.stdout);results.append(dict(language=language,case=name,passed=r.returncode==0))
    (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n')
    print(json.dumps(dict(passed=sum(r['passed'] for r in results),total=len(results))))
    raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
