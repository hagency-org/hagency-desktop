#!/usr/bin/env python3
"""Read real shared work in an already-open isolated Hagency mini app.

No process restart, account change, flags, work submission or fabricated records.
Use hidden/empty for those actual server states; record requires an existing
request/work ID and asserts only its public recorded facts.
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
    'en': ('Agent work history', 'Work history', 'No recorded work in your current scope',
           'Amounts belong to individual calls. Retained accounting includes known consumption; it is not remaining resource capacity. A completed response does not guarantee complete usage accounting.'),
    'zh-CN': ('Agent 任务历史', '任务历史', '当前访问范围内暂无任务记录',
              '以上数量仅对应单次调用。保留记账量包含已知消耗，并非资源剩余额度。响应完成不代表用量核算已完成。'),
}
MUTATIONS = {'agent_submit', 'agent_confirm', 'agent_approve', 'agent_reject', 'execute', 'work_submit', 'top_up', 'remove', 'publish'}


def top(app):
    app.request('/m', k='scroll', x=750, y=650, dy=-100000, wait=0)
    time.sleep(.15)


def click_request(app, request_id):
    top(app)
    for _ in range(48):
        rows = app.snap()
        labels = [r for r in rows if r.get('t') == request_id]
        assert len(labels) == 1, 'Expected exact request ID on this index page'
        label = labels[0]
        buttons = sorted([r for r in rows if r.get('i') == 'work_list_open' and r['r'][1] > label['r'][1]], key=lambda r: r['r'][1])
        assert buttons, 'No work-history control for exact request ID'
        x, y, w, h = buttons[0]['r']
        if 32 <= y and y + h <= 780:
            app.request('/click', x=x + w/2, y=y + h/2, wait=0)
            time.sleep(.15)
            return
        app.request('/m', k='scroll', x=750, y=650, dy=160, wait=0)
        time.sleep(.15)
    raise AssertionError('Exact request control is unreachable')


def inspect_record(app, args, name):
    top(app)
    app.wait(COPY[args.language][1])
    app.capture(name + '-top')
    found, controls = set(), set()
    reached = False
    for _ in range(100):
        rows = app.snap()
        found.update(r.get('t', '') for r in rows)
        controls.update(r.get('i', '') for r in rows)
        footer = next((r for r in rows if r.get('t') == COPY[args.language][3]), None)
        if footer and 32 <= footer['r'][1] and footer['r'][1] + footer['r'][3] <= 780:
            app.capture(name + '-accounting')
            reached = True
            break
        app.request('/m', k='scroll', x=750, y=650, dy=180, wait=0)
        time.sleep(.15)
    assert reached, 'Work accounting/footer is unreachable'
    assert args.work_id in found, 'Expected real work ID absent from first page'
    for expected in args.expect_text:
        assert expected in found, 'Missing actual recorded fact: ' + expected
    assert not controls.intersection(MUTATIONS), 'History exposes mutation controls'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--run', type=Path, required=True)
    p.add_argument('--port', type=int, required=True)
    p.add_argument('--account', required=True)
    p.add_argument('--language', choices=COPY, required=True)
    p.add_argument('--expect', choices=['hidden', 'empty', 'record'], required=True)
    p.add_argument('--request-id')
    p.add_argument('--work-id')
    p.add_argument('--expect-text', action='append', default=[])
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    assert args.account.startswith('@') and args.account.endswith(':localhost:8088'), 'Use isolated loopback accounts'
    assert args.expect != 'record' or (args.request_id and args.work_id), 'Record mode requires actual IDs on first index/work pages'
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / 'captures'
    captures.mkdir()
    app = App(args.run, args.port, captures)
    report = {'passed': False, 'checks': [], 'binarySha256': app.binary_hash,
              'probeSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'scope': 'read-only existing work observations; no current execution or free-capacity claim'}
    try:
        app.wait(args.account)
        idle(app)
        original = saved_command(app, args.account)
        app.click('overview')
        idle(app)
        top(app)
        if args.expect == 'hidden':
            assert not any(r.get('i') == 'overview_work' for r in app.snap()), 'Unsupported server exposes work history'
            app.capture('unsupported-overview')
            report['checks'].append('old server hides work-history entry')
        else:
            app.click('overview_work')
            idle(app)
            app.wait(COPY[args.language][0])
            app.capture('work-index')
            if args.expect == 'empty':
                app.wait(COPY[args.language][2])
                assert not any(r.get('i') == 'work_list_open' for r in app.snap())
                app.click('work_list_refresh')
                idle(app)
                app.wait(COPY[args.language][2])
                app.capture('empty-index-refreshed')
                report['checks'].append('real empty scope remains empty after fresh read, without zero-usage inference')
            else:
                click_request(app, args.request_id)
                idle(app)
                inspect_record(app, args, 'work-history')
                top(app)
                app.click('work_refresh')
                idle(app)
                inspect_record(app, args, 'work-history-refreshed')
                report['checks'].append('exact real request/work records remain reachable and read-only after refresh')
                top(app)
                app.click('work_back')
                idle(app)
                app.wait(COPY[args.language][0])
            top(app)
            app.click('work_list_back')
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
