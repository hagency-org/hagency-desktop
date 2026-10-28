#!/usr/bin/env python3
"""Real Splash callbacks for public connection metadata and native custody."""
import argparse,json,subprocess
from pathlib import Path
from check_full_bundle import APP,fixture
SETUP=r'''
let H=mod.hagency let A=mod.web_actions let c=H.webConnection
A.redraw=fn(){return nil} A.save=fn(){return true} H.ready=true H.busy=false H.command=nil
let calls=[] let replies=[]
host.request=fn(service,args,done){calls.push({service:service args:args}) replies.push(done)}
let record={id:"matrix.test" label:"Test server" apiBaseUrl:"https://matrix.test" revision:2 credentialKind:nil hasCredential:false awaitingInstall:false accessState:"unconfigured" checkedAt:nil allocatedTokens:5000 registrationAvailable:false}
let original={service:"hagency.web_connections.allowance" args:{id:"matrix.test" expectedRevision:1 commandId:"original" allocatedTokens:5000}}
let receipt={commandId:"original" operation:"allowance" connection:record}
let widgets=[] let texts=[]
let visit=fn(n){if !n.is_object(){return} for index key child in n{widgets.push({name:key.to_string() value:child}) if key.to_string()=="text" && child.is_string(){texts.push(child)} visit(child)}}
let find=fn(name){for row in widgets{if row.name==name{return row.value}} return nil}
let detail=fn(){H.page="web_connection" c.id=record.id c.detail=record c.tokens="5000"}
let staged=fn(service,args){H.command={service:service args:args} H.command.args.commandId="original"}
A.submit=staged
'''
CASES={
 'list_and_next':'A.web_connections_open("a.test") assert(calls[0].args.after=="a.test") replies[0]({is_ok:true data:{items:[record] nextAfter:"matrix.test"}}) RENDER find("web_connections_more").on_click() assert(calls[1].args.after=="matrix.test")',
 'detail_exact_identity':'A.web_connection_open("matrix.test") replies[0]({is_ok:true data:record}) assert(c.detail==record)',
 'detail_wrong_identity':'A.web_connection_open("other.test") replies[0]({is_ok:true data:record}) assert(c.detail==nil)',
 'new_server_probe':'A.web_connection_new() c.draft.id="matrix.test" A.web_connection_probe() assert(calls[0].service=="hagency.web_connections.probe" && calls[0].args.serverName=="matrix.test" && calls[0].args.url==nil) replies[0]({is_ok:true data:{reachable:true origin:"https://matrix.test"}}) assert(c.draft.origin=="https://matrix.test")',
 'native_import_only_public_metadata':'A.web_connection_new() c.draft={id:"matrix.test" label:"Test" origin:"https://matrix.test"} RENDER find("web_server_import").on_click() assert(H.command.service=="hagency.web_connections.import" && H.command.args.id=="matrix.test" && H.command.args.expectedRevision==0 && A.recipient_optional(H.command.args,"path")==nil && A.recipient_optional(H.command.args,"credential")==nil)',
 'allowance_button_uses_current_revision':'detail() RENDER find("web_connection_allowance_review").on_click() assert(H.command.service=="hagency.web_connections.allowance" && H.command.args.expectedRevision==2 && H.command.args.allocatedTokens==5000)',
 'allowance_clear':'detail() c.tokens="" A.web_connection_edit("allowance") assert(H.command.args.allocatedTokens==nil)',
 'invalid_allowance_refused':'detail() c.tokens="1.5" A.web_connection_edit("allowance") assert(H.command==nil)',
 'staged_registration_can_verify':'detail() record.awaitingInstall=true RENDER assert(find("web_connection_verify").enabled) find("web_connection_verify").on_click() assert(H.command.service=="hagency.web_connections.verify")',
 'credential_replacement_never_collects_secret':'detail() RENDER find("web_connection_replace").on_click() assert(H.command.args.to_json()=={id:"matrix.test" expectedRevision:2 commandId:"original"}.to_json())',
 'registration_review_contains_public_url':'detail() c.registrationUrl="https://agent.test" A.web_connection_edit("issue") assert(H.command.args.url=="https://agent.test" && A.recipient_optional(H.command.args,"asToken")==nil)',
 'download_original_revision':'detail() record.registrationAvailable=true RENDER find("web_registration_download").on_click() assert(calls[0].service=="hagency.web_connections.download" && calls[0].args.to_json()=={id:"matrix.test" expectedRevision:2}.to_json())',
 'receipt_routes_to_saved_connection':'H.command=original A.web_connection_reply(original,{is_ok:true data:receipt}) assert(H.command==nil && c.detail==record && H.page=="web_connection")',
 'wrong_operation_remains_unknown':'H.command=original receipt.operation="withdraw" A.web_connection_reply(original,{is_ok:true data:receipt}) assert(H.command==original && c.detail==nil)',
 'unknown_result_keeps_original':'H.command=original A.web_connection_reply(original,{is_ok:false error:"management_outcome_unknown"}) assert(H.command==original) A.web_connection_check() assert(calls[0].service=="hagency.web_connections.command.get" && calls[0].args.commandId=="original") replies[0]({is_ok:true data:receipt}) assert(H.command==nil)',
 'empty_result_does_not_reapply':'H.command=original A.web_connection_check() replies[0]({is_ok:true data:nil}) assert(calls.len()==1 && H.command==original)',
 'native_cancellation_clears_draft_command':'H.command=original A.web_connection_reply(original,{is_ok:false error:"web_connection_cancelled"}) assert(H.command==nil && c.detail==nil)',
}
def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
 s=(APP/'bundle/main.splash').read_text();start=s.index('        if H.page=="web_connections"{');end=s.index('        if H.page=="operator_graphs" {',start);render='\nlet tree=View{\n'+s[start:end]+'\n}\nvisit(tree)\n';results=[]
 for lang in ['en','zh-CN']:
  prefix=fixture(s+'\nmod.web_actions=A\n',lang)
  for name,body in CASES.items():
   path=a.output/f'{lang}-{name}.splash';path.write_text(prefix+SETUP+body.replace('RENDER',render)+'\ntrue\n');r=subprocess.run([str(a.vm.resolve()),'eval',str(path)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);path.with_suffix('.log').write_text(r.stdout);results.append({'case':name,'language':lang,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results),'failed':[r for r in results if not r['passed']]}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
