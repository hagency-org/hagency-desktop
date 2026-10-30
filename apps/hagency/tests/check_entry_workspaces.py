#!/usr/bin/env python3
"""Exercise workspace boundaries and retention with the shipped VM helpers."""
import argparse
import json
from pathlib import Path
import subprocess
from check_full_bundle import APP, fixture

SETUP = '''
A.redraw=fn(){}
A.save=fn(){H.persisted=H.entry_role return true}
H.ready=true H.busy=false H.command=nil H.saved_commands=[]
H.session={account:"@me:test" canAdmitResources:true features:["resource-pool-agents-v1"]}
H.calls=[] H.persisted=""
A.request=fn(service,args,done){H.calls.push({service:service args:args}) H.callback=done}
'''
CASES = {
    "role_choice_does_not_grant_authority": '''
H.session.canAdmitResources=false
A.entry_choose("admin") assert(H.entry_role=="")
A.entry_choose("made-up") assert(H.entry_role=="")
A.entry_choose("owner") assert(H.entry_role=="owner" && H.persisted=="owner")
assert(!H.session.canAdmitResources)
H.session.canAdmitResources=true A.entry_choose("admin") assert(H.entry_role=="admin")
''',
    "switch_preserves_drafts_and_unknown_operation": '''
H.pool_draft={name:"小蓝" tokens:"123456" roomId:"!saved:test"}
H.command={service:"hagency.pools.agents.request" args:{commandId:"original" requestedTokens:123456}}
let draft=H.pool_draft.to_json() let command=H.command.to_json()
A.entry_choose("owner") A.entry_switch()
assert(H.entry_choosing && H.entry_role=="owner" && H.persisted=="owner")
A.entry_choose("manager")
assert(H.page=="overview" && !H.entry_choosing && H.persisted=="manager")
assert(H.pool_draft.to_json()==draft && H.command.to_json()==command && H.calls.len()==0)
H.busy=true A.entry_choose("admin") A.entry_switch() assert(H.entry_role=="manager" && !H.entry_choosing)
''',
    "owner_and_manager_lists_are_separate": '''
H.pool_agents=[{id:"managed" managerMxid:"@me:test" resourceOwnerMxid:"@other:test"},{id:"owned" managerMxid:"@other:test" resourceOwnerMxid:"@me:test"},{id:"both" managerMxid:"@me:test" resourceOwnerMxid:"@me:test"}]
H.entry_role="manager" assert(A.pool_agent_rows().len()==2 && A.pool_agent_rows()[0].id=="managed")
H.entry_role="owner" assert(A.pool_agent_rows().len()==2 && A.pool_agent_rows()[0].id=="owned")
// Owner requests use the scoped sub-page reader; changing the active filter
// fetches once, while selecting it again preserves the current page.
A.sub_read=fn(service,args,done,failed){H.calls.push({service:service args:args}) H.callback=done return true}
H.page="inbox" H.reqs.filter="all" A.inbox_filter(true)
assert(H.calls[0].service=="hagency.pools.agents.list" && H.calls[0].args.needsMyAction)
A.inbox_filter(true) assert(H.calls.len()==1)
A.inbox_filter(false) assert(!H.calls[1].args.needsMyAction)
''',
    "admin_inbox_has_only_admission": '''
H.entry_role="admin" H.page="inbox"
A.projects_enabled=fn(){return true} A.allocations_enabled=fn(){return true} A.agents_enabled=fn(){return true}
A.load_inbox()
assert(H.calls.len()==1 && H.calls[0].service=="hagency.inbox.list")
assert(!H.calls[0].args.includeProjects && !H.calls[0].args.includeAllocations)
H.callback({items:[] nextAfter:nil}) assert(H.ux.inboxReady && H.calls.len()==1)
''',
    "navigation_matches_task": '''
H.entry_role="owner" H.page="pool_agent" assert(A.entry_parent()=="inbox")
H.entry_role="manager" assert(A.entry_parent()=="agents")
H.entry_role="admin" H.page="connections" assert(A.entry_parent()=="servers")
H.entry_role="owner" assert(A.entry_parent()=="resources")
H.page="provider_account" assert(A.entry_parent()=="accounts")
H.page="device_execution" assert(A.entry_parent()=="accounts")
''',
}

def main():
    p=argparse.ArgumentParser();p.add_argument("vm",type=Path);p.add_argument("--output",type=Path,required=True);a=p.parse_args()
    a.output.mkdir(parents=True,exist_ok=True)
    source=(APP/"bundle/main.splash").read_text();results=[]
    for language in ("en","zh-CN"):
        for name,case in CASES.items():
            path=a.output/f"{language}-{name}.splash";path.write_text(fixture(source+SETUP+case,language))
            r=subprocess.run([str(a.vm.resolve()),"root",str(path)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            path.with_suffix(".log").write_text(r.stdout);results.append(dict(language=language,case=name,passed=r.returncode==0))
    (a.output/"report.json").write_text(json.dumps(results,indent=2)+"\n")
    print(json.dumps(dict(passed=sum(r["passed"] for r in results),total=len(results))))
    raise SystemExit(not all(r["passed"] for r in results))

if __name__=="__main__": main()
