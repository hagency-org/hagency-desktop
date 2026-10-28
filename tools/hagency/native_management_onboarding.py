#!/usr/bin/env python3
"""Inspect native onboarding readiness and optionally close an initial review.

Requires an already-open isolated loopback fixture. No migration confirmation,
grant reservation, server publication, account change, flags, seed or restart.
The optional review probe is limited to an observed fresh legacy state; it opens
native read-only context and closes without entering a ceiling or confirming.
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
    'en': {
        'title': 'Set up shared capacity',
        'steps': ('1 · Review this device', '2 · Check server admission', '3 · Review capacity and publication'),
        'states': {'legacy': 'Device migration needs review', 'recovery_required': 'Resume the original migration',
                   'active': 'Shared resource inventory available', 'unavailable': 'Shared-capacity setup unavailable'},
        'native': 'Review resource migration', 'choose': 'Enter the shared monthly account limit.',
        'primary': 'Review migration', 'close': 'Close review', 'closed': 'Native review closed',
    },
    'zh-CN': {
        'title': '设置共享容量',
        'steps': ('1 · 核对此设备', '2 · 检查服务器准入', '3 · 核对容量与发布'),
        'states': {'legacy': '需要核对设备资源迁移', 'recovery_required': '恢复原始迁移',
                   'active': '共享资源清单已可用', 'unavailable': '共享容量设置暂不可用'},
        'native': '核对资源迁移', 'choose': '请输入共享账号每月上限',
        'primary': '核对迁移', 'close': '关闭核对', 'closed': '已关闭原生核对',
    },
}


def top(app):
    app.request('/m', k='scroll', x=750, y=650, dy=-100000, wait=0)
    time.sleep(.15)


def inspect_steps(app, copy, state, capture):
    top(app)
    app.wait(copy['title'])
    app.wait(copy['states'][state])
    app.capture(capture + '-top')
    texts, controls = set(), set()
    for _ in range(28):
        rows = app.snap()
        texts.update(row.get('t', '') for row in rows)
        controls.update(row.get('i', '') for row in rows)
        bottom = next((row for row in rows if row.get('t') == copy['steps'][-1]), None)
        if bottom and 32 <= bottom['r'][1] and bottom['r'][1] + bottom['r'][3] <= 780:
            app.capture(capture + '-capacity')
            break
        app.request('/m', k='scroll', x=750, y=650, dy=180, wait=0)
        time.sleep(.15)
    assert set(copy['steps']).issubset(texts), 'The actual three-stage guidance is not reachable'
    assert not controls.intersection({'tokens', 'migration_confirm', 'grant_confirm', 'publish', 'execute'}), 'Script exposes native effect controls'
    if state == 'unavailable':
        assert 'onboarding_review' not in controls, 'Unavailable native migration offers a review action'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--run', type=Path, required=True)
    p.add_argument('--port', type=int, required=True)
    p.add_argument('--account', required=True)
    p.add_argument('--expect', choices=('legacy', 'recovery_required', 'active', 'unavailable'), required=True)
    p.add_argument('--language', choices=COPY, required=True)
    p.add_argument('--open-legacy-review', action='store_true')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    assert a.account.startswith('@') and a.account.endswith(':localhost:8088'), 'Use an isolated loopback account'
    if a.open_legacy_review and a.expect != 'legacy':
        p.error('--open-legacy-review requires fresh legacy state; original recovery may perform a pending native handoff')
    a.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = a.output / 'captures'
    captures.mkdir()
    app = App(a.run, a.port, captures)
    copy = COPY[a.language]
    report = {'passed': False, 'checks': [], 'account': a.account, 'expect': a.expect,
              'binarySha256': app.binary_hash, 'probeSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'scope': 'actual readiness, stage navigation and optional initial native review cancellation; no confirmation'}
    try:
        app.wait(a.account)
        idle(app)
        original = saved_command(app, a.account)
        app.click('resources')
        idle(app)
        app.click('shared_capacity')
        idle(app)
        inspect_steps(app, copy, a.expect, 'onboarding')
        report['checks'].append('actual native state and three separate stages are visible without script consent')
        if a.open_legacy_review:
            top(app)
            app.click('onboarding_review')
            app.wait(copy['native'])
            app.wait(copy['choose'])
            app.wait(a.account)
            rows = app.snap()
            inputs = [row for row in rows if row.get('i') == 'tokens']
            assert len(inputs) == 1 and not inputs[0].get('t'), 'Native shared ceiling must have no default'
            for text in (copy['primary'], copy['close']):
                buttons = [row for row in rows if row.get('t') == text]
                assert len(buttons) == 1, 'Native control not unique: ' + text
                x, y, width, height = buttons[0]['r']
                assert x >= 0 and y >= 32 and width > 0 and height >= 40 and y + height <= 790, 'Native footer is clipped: ' + text
            app.capture('native-initial-review')
            app.click(copy['close'])
            idle(app)
            top(app)
            app.wait(copy['closed'])
            app.wait(copy['states']['legacy'])
            report['checks'].append('initial native review has an empty ceiling, reachable controls, and closes without activation')
        top(app)
        app.click('onboarding_refresh')
        idle(app)
        inspect_steps(app, copy, a.expect, 'onboarding-refreshed')
        report['checks'].append('refresh reads actual readiness and preserves expected native state')
        top(app)
        app.click('onboarding_back')
        idle(app)
        assert any(row.get('i') == 'shared_capacity' for row in app.snap()), 'Back did not return to Resources'
        app.wait(a.account)
        assert saved_command(app, a.account) == original, 'Navigation changed the saved command or draft presence'
        report['checks'].append('Resources, account and original command are preserved')
        with (app.root / 'native.log').open('rb') as stream:
            stream.seek(app.log_offset)
            errors = [line for line in stream.read().decode(errors='replace').splitlines()
                      if '[E]' in line or 'panicked at' in line or 'on_render closure failed' in line]
        report['nativeScriptErrors'] = len(errors)
        assert not errors, 'Native errors; inspect retained native.log'
        report['passed'] = True
    except Exception as error:
        report['failure'] = str(error)
        raise
    finally:
        report['trace'] = app.trace
        (a.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
