#!/usr/bin/env python3
"""Private details appear only after the native authenticated inbox hydrates them."""
import argparse, json, subprocess
from pathlib import Path
from check_full_bundle import APP, fixture

CHECKS = '''
H.session={account:"@owner:test"}
let row={approval:{id:"sap_private" agentName:"Agent" ownerMxid:"@owner:test" request:{encrypted:{requestDigest:"opaque"} sessionAllowed:true}} decision:nil canDecide:false encryptedPending:true}
H.shared_approvals=[row]
assert(A.shared_rows(true).len()==0 && A.shared_rows(false).len()==1)
assert(A.perm_info(row).exact=="" && A.perm_info(row).reason=="")
let pending_title=A.perm_info(row).title
row.approval.request={method:"item/commandExecution/requestApproval" params:{command:"private command" cwd:"/workspace"} sessionAllowed:true managerCanDecide:false}
row.encryptedPending=false row.canDecide=true
assert(A.shared_rows(true).len()==1 && A.shared_rows(false).len()==0)
assert(A.perm_info(row).exact=="private command\n"+A.t("in ","位置：")+"/workspace" && A.perm_info(row).title!=pending_title)
assert(A.shared_session(row))
row.decision="approve_session"
assert(A.shared_rows(true).len()==0 && A.shared_rows(false).len()==0)
'''

def main():
    p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
    source=(APP/'bundle/main.splash').read_text();results=[]
    for language in ['en','zh-CN']:
        path=a.output/(language+'.splash');path.write_text(fixture(source+CHECKS,language))
        r=subprocess.run([str(a.vm.resolve()),'root',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        path.with_suffix('.log').write_text(r.stdout);results.append({'language':language,'passed':r.returncode==0})
    (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n')
    print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}))
    raise SystemExit(not all(r['passed'] for r in results))

if __name__=='__main__':main()
