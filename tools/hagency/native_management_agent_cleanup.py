#!/usr/bin/env python3
"""Read real agent cleanup in an already-open isolated Hagency mini app.

No process restart, account change, flags, work submission or fabricated records.
Use hidden/empty for those actual server states; record requires an existing
request ID and asserts only its public recorded facts.
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
    'en': ('Agent removal and cleanup', 'Agent removal status', 'No agent requests are visible in this scope.',
           'Retained amounts include spent tokens and unresolved liability. They are not available capacity or a refund.'),
    'zh-CN': ('Agent 移除与清理', 'Agent 移除状态', '当前访问范围内没有可见的 Agent 申请。',
              '保留记账量包括已消耗 token 与尚未核清的责任，不代表可用容量或退回额度。'),
}
MUTATIONS = {'agent_submit', 'agent_confirm', 'agent_approve', 'agent_reject', 'execute', 'work_submit', 'top_up', 'remove', 'publish'}


def top(app):
    app.request('/m', k='scroll', x=900, y=600, dy=-100000, wait=0)
    time.sleep(.15)


def click_request(app, request_id):
    top(app)
    for _ in range(48):
        rows = app.snap()
        labels = [r for r in rows if r.get('t') == request_id]
        if not labels:
            app.request('/m', k='scroll', x=900, y=600, dy=160, wait=0)
            time.sleep(.15)
            continue
        assert len(labels) == 1, 'Exact request ID is ambiguous'
        label = labels[0]
        buttons = sorted([r for r in rows if r.get('i') == 'removal_open' and r['r'][1] > label['r'][1]], key=lambda r: r['r'][1])
        if not buttons:
            app.request('/m', k='scroll', x=900, y=600, dy=160, wait=0)
            time.sleep(.15)
            continue
        x, y, w, h = buttons[0]['r']
        if 32 <= y and y + h <= 780:
            app.request('/click', x=x + w/2, y=y + h/2, wait=0)
            time.sleep(.15)
            return
        app.request('/m', k='scroll', x=900, y=600, dy=160, wait=0)
        time.sleep(.15)
    raise AssertionError('Exact request control is unreachable')


def inspect_record(app, args, name):
    top(app)
    app.wait(COPY[args.language][1])
    app.capture(name + '-top')
    found, controls, visible_actions = set(), set(), set()
    reached = False
    for _ in range(100):
        rows = app.snap()
        found.update(r.get('t', '') for r in rows)
        controls.update(r.get('i', '') for r in rows)
        for row in rows:
            if row.get('i') in {'removal_cancel', 'removal_revoke'}:
                x, y, w, h = row['r']
                if w > 0 and h > 0 and 32 <= y and y + h <= 780:
                    visible_actions.add(row['i'])
        footer = next((r for r in rows if r.get('t') == COPY[args.language][3]), None)
        if footer and 32 <= footer['r'][1] and footer['r'][1] + footer['r'][3] <= 780:
            app.capture(name + '-accounting')
            reached = True
            break
        app.request('/m', k='scroll', x=900, y=600, dy=180, wait=0)
        time.sleep(.15)
    assert reached, 'Cleanup accounting/footer is unreachable'
    # The current UI keeps original receipt/ID in a separate audit section.
    # Exercise the real disclosure; current process observations do not replace it.
    if args.request_id not in found:
        for _ in range(20):
            row = next((r for r in app.snap() if r.get('i') == 'removal_details'), None)
            if row and 32 <= row['r'][1] and row['r'][1] + row['r'][3] <= 780:
                x, y, w, h = row['r']
                app.request('/click', x=x+w/2, y=y+h/2, wait=0)
                time.sleep(.2)
                break
            app.request('/m', k='scroll', x=900, y=600, dy=150, wait=0)
            time.sleep(.15)
        else:
            raise AssertionError('Cleanup receipt disclosure is unreachable')
        for _ in range(30):
            rows = app.snap()
            visible = [r for r in rows if 32 <= r['r'][1] and r['r'][1]+r['r'][3] <= 780]
            found.update(r.get('t', '') for r in visible)
            if args.request_id in found:
                app.capture(name + '-original-receipt')
                break
            app.request('/m', k='scroll', x=900, y=600, dy=150, wait=0)
            time.sleep(.15)
    assert args.request_id in found, 'Expected real request ID absent'
    assert ('removal_cancel' in controls) == args.can_cancel, 'Manager cancellation permission differs from expected real role'
    assert ('removal_revoke' in controls) == args.can_revoke, 'Assigned-admin revocation permission differs from expected real role'
    if args.can_cancel:
        assert 'removal_cancel' in visible_actions, 'Cancellation control is not fully reachable'
    if args.can_revoke:
        assert 'removal_revoke' in visible_actions, 'Revocation control is not fully reachable'
    for expected in args.expect_text:
        assert expected in found, 'Missing actual recorded fact: ' + expected
    assert not controls.intersection(MUTATIONS), 'Cleanup view exposes unrelated execution controls'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--run', type=Path, required=True)
    p.add_argument('--port', type=int, required=True)
    p.add_argument('--account', required=True)
    p.add_argument('--language', choices=COPY, required=True)
    p.add_argument('--expect', choices=['hidden', 'empty', 'record'], required=True)
    p.add_argument('--request-id')
    p.add_argument('--can-cancel', action='store_true')
    p.add_argument('--can-revoke', action='store_true')
    p.add_argument('--expect-text', action='append', default=[])
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    assert args.account.startswith('@') and args.account.endswith(':localhost:8088'), 'Use isolated loopback accounts'
    assert args.expect != 'record' or args.request_id, 'Record mode requires an actual request ID on the first index page'
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / 'captures'
    captures.mkdir()
    app = App(args.run, args.port, captures)
    report = {'passed': False, 'checks': [], 'binarySha256': app.binary_hash,
              'probeSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'scope': 'read-only existing cleanup observations; no current execution or free-capacity claim'}
    try:
        app.wait(args.account)
        idle(app)
        original = saved_command(app, args.account)
        app.click('overview')
        idle(app)
        top(app)
        if args.expect == 'hidden':
            assert not any(r.get('i') == 'overview_cleanup' for r in app.snap()), 'Unsupported server exposes cleanup history'
            app.capture('unsupported-overview')
            report['checks'].append('old server hides cleanup entry')
        else:
            app.click('overview_cleanup')
            idle(app)
            app.wait(COPY[args.language][0])
            app.capture('cleanup-index')
            if args.expect == 'empty':
                app.wait(COPY[args.language][2])
                assert not any(r.get('i') == 'removal_open' for r in app.snap())
                app.click('removals_refresh')
                idle(app)
                app.wait(COPY[args.language][2])
                app.capture('empty-index-refreshed')
                report['checks'].append('real empty scope remains empty after fresh read, without zero-usage inference')
            else:
                click_request(app, args.request_id)
                idle(app)
                inspect_record(app, args, 'cleanup')
                top(app)
                app.click('removal_refresh')
                idle(app)
                inspect_record(app, args, 'cleanup-refreshed')
                report['checks'].append('exact real request cleanup records remain reachable and read-only after refresh')
                top(app)
                app.click('removal_back')
                idle(app)
                app.wait(COPY[args.language][0])
            top(app)
            app.click('removals_back')
            idle(app)
            report['checks'].append('Back returns to overview without a new command')
        app.wait(args.account)
        assert saved_command(app, args.account) == original, 'Read changed original command or draft presence'
        report['checks'].append('account and original saved command unchanged')
        with (app.root / 'native.log').open('rb') as f:
            f.seek(app.log_offset)
            errors = [line for line in f.read().decode(errors='replace').splitlines()
                      if '[E]' in line or 'panicked at' in line or 'on_render closure failed' in line]
        report['nativeScriptErrors'] = len(errors)
        assert not errors, 'Native errors; retained native.log needs inspection'
        report['passed'] = True
    except Exception as error:
        report['failure'] = str(error)
        raise
    finally:
        report['trace'] = app.trace
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
