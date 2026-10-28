#!/usr/bin/env python3
"""Shipped alert views and callbacks in the pinned VM; no native pixel claims."""
import argparse,json,subprocess
from pathlib import Path
from check_agent_admission import APP,PREFIX
p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
source=(APP/'bundle/main.splash').read_text();helpers=source[source.index('A.alert_state = fn('):source.index('A.personal_reason = fn(')]
pages=source[source.index('        if H.page=="resource_alerts" ||'):source.index('        if H.page == "resource_detail" {mod.widgets.HagencyDetailCard{')]
fixture='''
let Down=3 mod.widgets.RINX_SPACING=1 mod.rinx_icons.inbox=1
H.page="resource_alerts" H.alert_resource="resource_original" H.alert_id="alert:original" H.alert_after="" H.alert_status=nil H.alert_note_after=0 H.alert_page=nil H.alert_detail=nil H.alert_draft=nil
A.refresh=fn(){if H.page=="resource_alerts"{A.open_alerts(H.alert_resource,H.alert_after,H.alert_status)}else{A.open_alert(H.alert_resource,H.alert_id,H.alert_note_after)}}
A.open_resource=fn(id){H.page="resource_detail" H.resource_id=id}
let row={id:"alert:original" resourceId:"resource_original" revision:"original_revision" summary:"Resource exceeded its declared ceiling" runbook:"Review commitments" impact:"Existing work remains authorized" recoveryCondition:"Drawn usage falls below the ceiling" status:"open" severity:"warning" occurrences:3 assignee:nil suppressUntil:nil lastSeen:"2026-10-18 00:00:00 UTC" next:["acknowledged","assigned","resolved","suppressed"]}
let page={resourceId:row.resourceId alerts:[row] nextAfter:nil}
let detail={alert:row notes:[{author:"@owner:test" text:"Checked usage" at:"2026-10-18 00:01:00 UTC"}] nextNote:nil}
'''
cases={
'loading':('H.busy=true','assert(has(A.t("Reading alerts…","正在读取告警…"))) assert(!button("alerts_refresh").enabled)'),
'error':('','assert(has(A.t("Alert list unavailable","告警列表暂不可用"))) assert(!has(A.t("No recorded alerts in this view","当前视图暂无告警记录")))'),
'empty':('H.alert_page=page page.alerts=[]','assert(has(A.t("No recorded alerts in this view","当前视图暂无告警记录")))'),
'open_detail':('H.alert_page=page','button("alert_open").on_click() assert(H.calls[0].service=="hagency.alerts.get" && H.calls[0].args.resourceId==row.resourceId && H.calls[0].args.id==row.id)'),
'next_page':('H.alert_page=page page.nextAfter="alert:cursor"','button("alerts_next").on_click() assert(H.calls[0].args.after=="alert:cursor")'),
'filter':('H.alert_page=page','button("alerts_open").on_click() assert(H.calls[0].args.status=="open" && H.calls[0].args.after=="")'),
'back_resource':('','button("alerts_back").on_click() assert(H.page=="resource_detail" && H.resource_id==row.resourceId)'),
'all_actions':('H.page="resource_alert" H.alert_detail=detail','for action in row.next {assert(has(A.alert_action(action)))}'),
'detail':('H.page="resource_alert" H.alert_detail=detail','assert(has(row.summary) && has(row.impact) && has(row.runbook) && has(row.recoveryCondition) && has("Checked usage"))'),
'resolved':('H.page="resource_alert" H.alert_detail=detail row.status="resolved" row.next=[]','assert(button("alert_choose")==nil && button("alert_save")==nil)'),
'review':('H.page="resource_alert" H.alert_detail=detail A.choose_alert_action("assigned")','assert(has(A.t("Assign to me","由我处理"))) button("alert_note").on_change("  Reviewed  ") button("alert_save").on_click() assert(H.calls.len()==1 && H.calls[0].service=="hagency.alerts.triage" && H.calls[0].args.id==row.id && H.calls[0].args.expectedRevision==row.revision && H.calls[0].args.to=="assigned" && H.calls[0].args.note=="Reviewed")'),
'cancel':('H.page="resource_alert" H.alert_detail=detail A.choose_alert_action("suppressed")','button("alert_cancel").on_click() assert(H.alert_draft==nil && H.calls.len()==0)'),
'pending_guard':('H.page="resource_alert" H.alert_detail=detail A.choose_alert_action("assigned") H.command={service:"original"}','assert(!button("alert_save").enabled) button("alert_save").on_click() assert(H.calls.len()==0)'),
'note_page':('H.page="resource_alert" H.alert_detail=detail detail.nextNote=16','button("alert_notes_next").on_click() assert(H.calls[0].args.afterNote==16 && H.calls[0].args.id==row.id)'),
}
callbacks={
'late_list':'A.open_alerts(row.resourceId,"",nil) H.page="overview" H.callback(page) assert(H.alert_page==nil)',
'wrong_resource':'A.open_alerts(row.resourceId,"",nil) page.resourceId="other" H.callback(page) assert(H.alert_page==nil && H.notice!="")',
'late_detail':'A.open_alert(row.resourceId,row.id,0) H.alert_id="other" H.callback(detail) assert(H.alert_detail==nil)',
'valid_detail':'A.open_alert(row.resourceId,row.id,0) H.callback(detail) assert(H.alert_detail.alert.id==row.id)',
'illegal_action':'H.alert_detail=detail A.choose_alert_action("open") assert(H.alert_draft==nil && H.calls.len()==0)',
'stale_draft':'H.alert_detail=detail A.choose_alert_action("assigned") row.revision="new_observation" A.submit_alert() assert(H.calls.len()==0 && H.alert_draft==nil && H.notice!="")',
'busy_guard':'H.busy=true A.open_alerts(row.resourceId,"",nil) A.open_alert(row.resourceId,row.id,0) assert(H.calls.len()==0)',
}
records=[]
def run(name,code):
 f=a.output/(name+'.splash');f.write_text(code+'\ntrue\n');r=subprocess.run([str(a.vm.resolve()),'eval',str(f)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);f.with_suffix('.log').write_text(r.stdout);records.append({'case':name,'passed':r.returncode==0})
for lang in ['en','zh']:
 base=PREFIX+helpers+fixture+('A.t=fn(en,zh){return zh}\n' if lang=='zh' else '')
 for name,(state,check) in cases.items():run(name+'-'+lang,base+state+'\nlet root=View{'+pages+'}\ncollect(root)\n'+check)
 for name,check in callbacks.items():run(name+'-'+lang,base+check)
(a.output/'report.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps({'passed':sum(x['passed'] for x in records),'total':len(records)}));raise SystemExit(not all(x['passed'] for x in records))
