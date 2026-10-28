#!/usr/bin/env python3
"""Pinned VM recovery callbacks; native confirmation is tested separately.
No OAuth flow, native pixels or persistent account mutation is simulated here.
"""
import argparse,json,subprocess
from pathlib import Path
from check_agent_admission import APP,PREFIX

def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 s=(APP/'bundle/main.splash').read_text()
 helpers=s[s.index('A.error_text = fn('):s.index('A.onboarding_reason = fn(')]+s[s.index('A.reauthenticate = fn('):s.index('A.observe_verification = fn(')]
 start=s.index('    if H.needs_signin {',s.index('A.render_page = fn('));view=s[start:s.index('    if !H.ready {',start)]
 fixture='''H.page="overview" H.needs_signin=false H.command={args:{commandId:"original"}} H.saved_commands=[{commandId:"original"}] H.busy=false
'''
 cases={
 'hidden':('', 'assert(button("signin_again")==nil)\nA.reauthenticate()\nassert(H.calls.len()==0)'),
 'missing':('', '''A.request("hagency.session",{},fn(data){})
H.host_callback({is_ok:false error:"native_resume_missing"})
assert(H.needs_signin && H.command.args.commandId=="original" && H.saved_commands.len()==1)
assert(H.calls.len()==1)
A.reauthenticate()
assert(H.calls[1].service=="hagency.account.reauthenticate" && H.calls[1].args.to_json()=="{}")
H.host_callback({is_ok:true data:{state:"cancelled"}})
assert(H.needs_signin && H.command.args.commandId=="original" && H.calls.len()==2)
assert(H.notice==A.t("Sign-in cancelled. Your existing records and requests are unchanged.","已取消重新登录，已有记录和请求保持不变。"))'''),
 'revoked':('', 'A.request("hagency.session",{},fn(data){})\nH.host_callback({is_ok:false error:"native_resume_revoked"})\nassert(H.needs_signin && H.calls.len()==1)'),
 'network':('', 'A.request("hagency.session",{},fn(data){})\nH.host_callback({is_ok:false error:"server_unavailable"})\nassert(!H.needs_signin && H.calls.len()==1)'),
 'button':('H.needs_signin=true','assert(button("signin_again")!=nil)\nbutton("signin_again").on_click()\nassert(H.calls.len()==1 && H.calls[0].service=="hagency.account.reauthenticate")'),
 'notice_once':('H.needs_signin=true H.notice=A.error_text("native_resume_missing")','assert(has(A.t("A fresh sign-in is needed for Hagency.","Hagency 需要重新登录。")))\nlet count=0 for text in texts {if text==A.error_text("native_resume_missing") {count=count+1}}\nassert(count==1)'),
 'notice_other':('H.needs_signin=true H.notice="different-status"','assert(has("different-status"))'),
 'busy':('H.needs_signin=true H.busy=true','assert(!button("signin_again").enabled)\nA.reauthenticate()\nassert(H.calls.len()==0)'),
 }
 results=[]
 for lang in ['en','zh']:
  for name,(state,checks) in cases.items():
   script=PREFIX+helpers+fixture+('A.t=fn(en,zh){return zh}\n' if lang=='zh' else '')+state+'\nlet root=View{'+view+'\nnotice := mod.widgets.HagencyHint{text:A.notice_text()}\n}\ncollect(root)\n'+checks+'\ntrue\n'
   file=a.output/(name+'-'+lang+'.splash');file.write_text(script)
   r=subprocess.run([str(a.vm.resolve()),'eval',str(file)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(a.output/(name+'-'+lang+'.log')).write_text(r.stdout);results.append({'case':name+'-'+lang,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
