#!/usr/bin/env python3
"""Inspect real agent admission discovery/readiness without issuing a command.

Operator supplies an already-open isolated localhost profile and real project.
This probe never enables flags, seeds capacity, restarts, changes accounts or
submits an agent request. Use hidden against an unsupported server; blocked
requires actual advertised admission and a project with unmet prerequisites.
"""
import argparse
import hashlib
import json
from pathlib import Path

from native_management_projects import idle, project
from native_management_wizard import App

EFFECTS = {"agent_new", "agent_submit", "agent_confirm", "agent_approve", "agent_reject", "agent_cancel"}


def saved_command(app, account):
    """Read-only accounts may never have created an onboarding draft."""
    try:
        draft = app.draft(account)
    except FileNotFoundError:
        return False, None
    return True, draft.get('command')


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--run',type=Path,required=True)
    p.add_argument('--port',type=int,required=True)
    p.add_argument('--account',required=True)
    p.add_argument('--project',required=True)
    p.add_argument('--expect',choices=('hidden','blocked'),required=True)
    p.add_argument('--expected-blocker',help='Exact actual EN/CN blocker text required for blocked mode')
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    assert a.account.startswith('@') and a.account.endswith(':localhost:8088'), 'Use isolated localhost account'
    if a.expect=='blocked' and not a.expected_blocker:
        p.error('--expected-blocker is required; do not infer absence of capacity from an empty page')
    a.output.mkdir(mode=0o700,parents=True,exist_ok=False)
    captures=a.output/'captures';captures.mkdir()
    app=App(a.run,a.port,captures)
    report={'passed':False,'checks':[],'scope':'read-only real native feature/readiness, no request, reservation or execution',
            'binarySha256':app.binary_hash,'probeSha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'account':a.account,'project':a.project,'expect':a.expect}
    try:
        app.wait(a.account);idle(app)
        original=saved_command(app,a.account)
        project(app,a.project);idle(app)
        controls={r.get('i') for r in app.snap()}
        if a.expect=='hidden':
            assert 'project_agents' not in controls, 'Unsupported server exposes agent admission'
            report['checks'].append('server without discovery has no agent admission control')
            app.capture('unsupported-project')
        else:
            # The coordinator's extra connection section pushes this control
            # below the viewport. Click searches by scrolling to the real button.
            app.click('project_agents');idle(app)
            app.wait(a.project);app.wait(a.expected_blocker)
            assert not EFFECTS.intersection({r.get('i') for r in app.snap()}), 'Unready project offers effect control'
            app.capture('agent-requirements')
            report['checks'].append('actual current blocker is visible without request or approval controls')
            app.click('agents_refresh');idle(app);app.wait(a.expected_blocker)
            assert not EFFECTS.intersection({r.get('i') for r in app.snap()})
            report['checks'].append('refresh preserves blocked read-only state')
            app.click('agents_project_back');idle(app);app.wait(a.project)
            app.capture('project-return')
        app.wait(a.account)
        assert saved_command(app,a.account)==original, 'Read-only navigation changed original command or draft presence'
        report['checks'].append('account, project and original command are preserved')
        with (app.root/'native.log').open('rb') as f:
            f.seek(app.log_offset)
            errors=[line for line in f.read().decode(errors='replace').splitlines()
                    if '[E]' in line or 'panicked at' in line or 'on_render closure failed' in line]
        report['nativeScriptErrors']=len(errors)
        assert not errors,'Native errors; inspect retained native.log'
        report['passed']=True
    except Exception as e:
        report['failure']=str(e)
        raise
    finally:
        report['trace']=app.trace
        (a.output/'report.json').write_text(json.dumps(report,indent=2)+'\n')

if __name__=='__main__':main()
