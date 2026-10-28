#!/usr/bin/env python3
"""Read real personal-agent settings in an already-open isolated mini app.

No policy save, agent creation, account transition, restart or flag changes.
Optional editor/observation confirmation cancellation only opens and dismisses
a local unsent draft; this probe never sends the device observation request.
"""
import argparse
import hashlib
import json
from pathlib import Path
import time
from native_management_agent_readiness import saved_command
from native_management_projects import idle
from native_management_wizard import App

COPY = {
    'en': ('Personal agents', 'No personal agents on this page',
           'Model and workspace editing are not available here yet. Provider credentials and paths stay native.',
           'Edit personal policy'),
    'zh-CN': ('个人 Agent', '此页暂无个人 Agent',
              '此处暂不支持编辑模型与工作区，模型服务凭据和路径保留在原生端。',
              '编辑个人策略'),
}

def top(app):
    app.request('/m', k='scroll', x=750, y=650, dy=-100000, wait=0)
    time.sleep(.15)

def click_agent(app, agent_id):
    top(app)
    for _ in range(70):
        rows = app.snap()
        labels = [r for r in rows if r.get('t') == agent_id]
        assert len(labels) <= 1, 'Exact visible personal-agent ID is ambiguous'
        candidates = sorted([r for r in rows if r.get('i') == 'personal_open'
                             and labels and r['r'][1] > labels[0]['r'][1]], key=lambda r:r['r'][1])
        if candidates:
            x,y,w,h = candidates[0]['r']
            if w > 0 and h > 0 and 32 <= y and y+h <= 780:
                app.request('/click', x=x+w/2, y=y+h/2, wait=0)
                return
        app.request('/m', k='scroll', x=750, y=650, dy=160, wait=0)
        time.sleep(.15)
    raise AssertionError('Existing exact personal-agent control is unreachable on this page')

def inspect(app, args, name):
    top(app)
    app.wait(args.agent_id)
    app.capture(name+'-top')
    texts,controls=set(),set()
    reached=False
    for _ in range(90):
        rows=app.snap()
        texts.update(r.get('t','') for r in rows)
        controls.update(r.get('i','') for r in rows)
        expected_footer=COPY[args.language][2] if args.expect=='record' else (
            'Refresh reads the saved observation. Read current settings asks the assigned device to report again.'
            if args.language=='en' else '刷新仅查看已保存的观测；读取当前设置会请求指定设备重新上报。')
        footer=next((r for r in rows if r.get('t')==expected_footer),None)
        if footer and 32 <= footer['r'][1] and footer['r'][1]+footer['r'][3] <= 780:
            app.capture(name+'-settings')
            reached=True
            break
        app.request('/m', k='scroll', x=750, y=650, dy=180, wait=0)
        time.sleep(.15)
    assert reached,'Expected settings/observation state is unreachable'
    assert args.agent_id in texts
    assert ('personal_edit' in controls)==args.can_edit,'Observed policy edit availability differs from expected real state'
    assert 'personal_submit' not in controls,'Detail unexpectedly exposes a save action'
    for value in args.expect_text:
        assert value in texts,'Missing real device observation: '+value

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--run',type=Path,required=True);p.add_argument('--port',type=int,required=True)
    p.add_argument('--account',required=True);p.add_argument('--language',choices=COPY,required=True)
    p.add_argument('--expect',choices=['empty','record','unobserved'],required=True);p.add_argument('--agent-id')
    p.add_argument('--observation-cancel',action='store_true')
    p.add_argument('--can-edit',action='store_true');p.add_argument('--editor-cancel',action='store_true')
    p.add_argument('--expect-text',action='append',default=[]);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    assert a.account.startswith('@') and a.account.endswith(':localhost:8088'),'Use isolated loopback accounts'
    assert a.expect=='empty' or a.agent_id,'Detail mode requires a real personal-agent ID on the first page'
    assert not a.editor_cancel or a.can_edit,'Editor cancellation requires an actually editable policy'
    a.output.mkdir(mode=0o700,parents=True,exist_ok=False);captures=a.output/'captures';captures.mkdir()
    app=App(a.run,a.port,captures)
    report={'passed':False,'checks':[],'binarySha256':app.binary_hash,
            'probeSha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'scope':'existing personal observations; optional unsent editor cancellation; no policy save'}
    try:
        app.wait(a.account);idle(app);original=saved_command(app,a.account)
        app.click('overview');idle(app);app.click('overview_personal');idle(app)
        app.wait(COPY[a.language][0]);app.capture('personal-list')
        if a.expect=='empty':
            app.wait(COPY[a.language][1]);app.click('personal_refresh');idle(app);app.wait(COPY[a.language][1])
            report['checks'].append('real empty personal list remains empty after refresh')
        else:
            click_agent(app,a.agent_id);idle(app);inspect(app,a,'personal')
            top(app);app.click('personal_reload');idle(app);inspect(app,a,'personal-refreshed')
            report['checks'].append('exact personal identity and expected real observations survive refresh')
            if a.observation_cancel:
                top(app)
                for _ in range(70):
                    rows=app.snap()
                    control=next((r for r in rows if r.get('i')=='personal_observe' and
                                  r['r'][2]>0 and 32<=r['r'][1] and r['r'][1]+r['r'][3]<=780),None)
                    if control:
                        x,y,w,h=control['r'];app.request('/click',x=x+w/2,y=y+h/2,wait=0);break
                    app.request('/m',k='scroll',x=750,y=650,dy=160,wait=0);time.sleep(.15)
                else:raise AssertionError('Explicit observation review is not available/reachable')
                app.wait('Read settings from this device?' if a.language=='en' else '从此设备读取设置？')
                app.capture('personal-observation-review')
                app.click('personal_observe_cancel');inspect(app,a,'personal-observation-cancelled')
                report['checks'].append('observation review cancellation leaves original request unsent')
            if a.editor_cancel:
                app.click('personal_edit');app.wait(COPY[a.language][3]);app.capture('personal-edit')
                app.click('personal_edit_cancel');app.wait(a.agent_id);inspect(app,a,'personal-cancelled')
                report['checks'].append('unsent editor cancellation preserves recorded observations')
            top(app);app.click('personal_back');idle(app);app.wait(COPY[a.language][0])
        top(app);app.click('personal_home');idle(app);app.wait(a.account)
        assert saved_command(app,a.account)==original,'Read/cancel changed original command or draft presence'
        report['checks'].append('Back, account and original saved command preserved')
        with (app.root/'native.log').open('rb') as f:
            f.seek(app.log_offset)
            errors=[line for line in f.read().decode(errors='replace').splitlines()
                    if '[E]' in line or 'panicked at' in line or 'on_render closure failed' in line]
        report['nativeScriptErrors']=len(errors)
        assert not errors,'Native errors retained in native.log'
        report['passed']=True
    except Exception as error:
        report['failure']=str(error)
        raise
    finally:
        report['trace']=app.trace
        (a.output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
if __name__=='__main__':main()
