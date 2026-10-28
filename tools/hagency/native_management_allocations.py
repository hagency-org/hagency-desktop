#!/usr/bin/env python3
"""Read an actual project allocation in an already-open isolated Hagency app.

No server flags, fixture records, approvals, release, account changes or restarts.
The caller supplies the exact existing allocation ID and expected public facts.
"""
import argparse
import hashlib
import json
from pathlib import Path
import time
from native_management_agent_readiness import saved_command
from native_management_projects import idle, project, select_card
from native_management_wizard import App

COPY = {
    'en': {'list':'Project allocations','empty':'No allocations in this view','id':'Allocation · ',
           'grant':'Owner grant','people':'People and record','unknown':'Unknown'},
    'zh-CN': {'list':'项目分配','empty':'当前视图暂无分配','id':'分配 · ',
              'grant':'资源主人授权','people':'人员与记录','unknown':'未知'},
}

def inspect(app, args, phase):
    labels=set();controls=set();last=False
    app.request('/m',k='scroll',x=750,y=650,dy=-100000,wait=0)
    time.sleep(.15)
    app.capture(phase+'-top')
    for _ in range(80):
        rows=app.snap();labels.update(r.get('t','') for r in rows);controls.update(r.get('i','') for r in rows)
        if any(r.get('t')==args.allocation_id and 32<=r['r'][1] and r['r'][1]+r['r'][3]<=780 for r in rows):
            app.capture(phase+'-record');last=True;break
        app.request('/m',k='scroll',x=750,y=650,dy=180,wait=0);time.sleep(.1)
    assert last,'Exact allocation ID was not reachable'
    for expected in args.expect_text:
        assert expected in labels,'Missing actual public fact: '+expected
    assert COPY[args.language]['grant'] in labels
    assert not controls.intersection({'execute','launch_agent','grant_approve','allocation_confirm'})
    for expected in args.expect_control:
        assert expected in controls,'Missing role control: '+expected
    for forbidden in args.forbid_control:
        assert forbidden not in controls,'Unexpected role control: '+forbidden


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--run',type=Path,required=True);p.add_argument('--port',type=int,required=True)
    p.add_argument('--account',required=True);p.add_argument('--project',required=True)
    p.add_argument('--language',choices=COPY,required=True)
    p.add_argument('--expect',choices=['hidden','empty','record'],required=True)
    p.add_argument('--allocation-id');p.add_argument('--expect-text',action='append',default=[])
    p.add_argument('--expect-control',action='append',default=[]);p.add_argument('--forbid-control',action='append',default=[])
    p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    assert a.account.startswith('@') and a.account.endswith(':localhost:8088'),'Use isolated loopback accounts'
    assert a.expect!='record' or a.allocation_id,'Record mode requires an actual allocation ID on the first page'
    a.output.mkdir(mode=0o700,parents=True,exist_ok=False);captures=a.output/'captures';captures.mkdir()
    app=App(a.run,a.port,captures)
    report={'passed':False,'checks':[],'scope':'read-only exact allocation and current human controls; no capacity or execution effect',
            'binarySha256':app.binary_hash,'probeSha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    try:
        app.wait(a.account);idle(app);original=saved_command(app,a.account)
        project(app,a.project);idle(app)
        if a.expect=='hidden':
            assert not any(r.get('i')=='project_allocations' for r in app.snap())
            app.capture('unsupported-project');report['checks'].append('unsupported allocation entry hidden')
        else:
            app.click('project_allocations');idle(app);app.wait(COPY[a.language]['list']);app.capture('allocation-list')
            if a.expect=='empty':
                app.wait(COPY[a.language]['empty']);app.click('allocation_list_refresh');idle(app);app.wait(COPY[a.language]['empty'])
                report['checks'].append('real empty allocation scope persists after refresh')
            else:
                select_card(app,COPY[a.language]['id']+a.allocation_id,'allocation_list_open');idle(app)
                inspect(app,a,'allocation')
                app.click('allocation_refresh');idle(app);inspect(app,a,'allocation-refreshed')
                report['checks'].append('exact allocation, public facts and role controls survive refresh')
                app.click('allocation_back');idle(app);app.wait(COPY[a.language]['list'])
            app.click('allocation_project_back');idle(app);app.wait(a.project)
            app.capture('project-return');report['checks'].append('Back preserves originating project')
        app.wait(a.account);assert saved_command(app,a.account)==original,'Read changed draft presence or original command'
        report['checks'].append('account and original command unchanged')
        with (app.root/'native.log').open('rb') as f:
            f.seek(app.log_offset);errors=[x for x in f.read().decode(errors='replace').splitlines() if '[E]' in x or 'panicked at' in x or 'on_render closure failed' in x]
        report['nativeScriptErrors']=len(errors);assert not errors,'Native errors; inspect retained log'
        report['passed']=True
    except Exception as e:
        report['failure']=str(e);raise
    finally:
        report['trace']=app.trace;(a.output/'report.json').write_text(json.dumps(report,indent=2)+'\n')

if __name__=='__main__':main()
