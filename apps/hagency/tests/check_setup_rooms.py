#!/usr/bin/env python3
"""Evaluate the actual setup room subtree in the pinned Makepad script VM.

The VM helper has `parse <path>` and `eval <path>` modes. It asserts that eval
returns true and no diagnostics were captured. This does not emulate native
widgets or replace the full-window instrument checks.
"""
import argparse
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
APP = HERE.parent


def run(vm, mode, path, log):
    result = subprocess.run([str(vm), mode, str(path)], text=True, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, check=False)
    log.write_text(result.stdout)
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("vm", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    vm = args.vm.resolve()
    source = (APP / "bundle/main.splash").read_text()
    assert run(vm, "parse", APP / "bundle/main.splash", args.output / "parse.log").returncode == 0
    start = source.index('                mod.widgets.HagencySection{\n                    mod.widgets.HagencySectionTitle{text:A.t("Matrix rooms"')
    end = source.index('                if p.pendingCommands', start)
    section = source[start:end].strip()
    helpers = source[source.index('A.setup_stage_unknown = fn('):source.index('A.setup_reason = fn(')]
    prefix = (HERE / "fixtures/setup-rooms-release7.splash").read_text().split('\nlet p=H.setup_progress')[0]
    fixtures = {
        "empty": ('', 3, '''assert(section[1][1].text == A.t("Room creation has not begun", "尚未开始创建房间"))'''),
        "remote": ('H.setup_progress.readiness.setup={spaceId:"!space:test" roomId:"!room:test"}', 6, '''
assert(section[1][1].text == A.t("History is on the original device", "创建历史保存在原设备上"))
assert(section[3][1].text == "!space:test")
assert(section[4][1].text == "!room:test")'''),
        "known": ('''H.setup_progress.readiness.setup={spaceId:"!space:test" roomId:"!room:test"}
H.setup_progress.localHistory=true
H.setup_progress.space={state:"room_returned" roomId:"!space:test"}
H.setup_progress.room={state:"room_returned" roomId:"!room:test"}''', 7, '''
assert(section[2][1].text == A.t("Room ID received", "已收到房间 ID"))
assert(section[3].text == "!space:test")
assert(section[4][1].text == A.t("Room ID received", "已收到房间 ID"))
assert(section[5].text == "!room:test")'''),
        "unknown": ('''H.setup_progress.readiness.setup={spaceId:nil roomId:nil}
H.setup_progress.localHistory=true
H.setup_progress.space={state:"outcome_unknown" roomId:nil}
H.setup_progress.room={state:"outcome_unknown" roomId:nil}''', 6, '''
assert(section[2][1].text == A.t("Reply missing · recover the original request", "响应未收到 · 恢复原始请求"))
assert(section[3][1].text == A.t("Reply missing · recover the original request", "响应未收到 · 恢复原始请求"))
assert(section[4].len() == 3)
let space_key=false
let room_key=false
for input_group in [section[4][1], section[4][2]] {
for i key value in input_group {
    if key.to_string() == "setup_space_input" {space_key=true value.on_change("!recover-space:test")}
    if key.to_string() == "setup_room_input" {room_key=true value.on_change("!recover-room:test")}
}
}
assert(space_key && room_key)
assert(H.setup_space == "!recover-space:test" && H.setup_room == "!recover-room:test")'''),
    }
    for pending, other in (("space", "room"), ("room", "space")):
        fixtures[f"unknown-{pending}"] = (
            'H.setup_progress.readiness.setup={spaceId:nil roomId:nil}\n'
            'H.setup_progress.localHistory=true\n'
            f'H.setup_progress.{pending}={{state:"outcome_unknown" roomId:nil}}\n'
            f'H.setup_progress.{other}={{state:"room_returned" roomId:"!known:test"}}',
            7,
            f'''assert(section[5].len() == 2)
let input_found=false
for i key value in section[5][1] {{
    if key.to_string() == "setup_{pending}_input" {{input_found=true value.on_change("!candidate:test")}}
    assert(key.to_string() != "setup_{other}_input")
}}
assert(input_found)
assert(H.setup_{pending} == "!candidate:test")''',
        )
    results = []
    for language in ("en", "zh"):
        for name, (state, count, assertions) in fixtures.items():
            test = prefix + '\n' + helpers + '\n'
            if language == "zh":
                test += 'A.t=fn(en,zh){return zh}\n'
            test += state + '\nlet p=H.setup_progress\nlet r=p.readiness\nlet section=' + section
            test += '\nassert(section.len() == ' + str(count) + ')\n'
            test += 'assert(section[0].text == A.t("Matrix rooms", "Matrix 房间"))\n'
            test += 'assert(section[' + str(count - 1) + '][1].text == A.t("Not currently verified", "当前未验证"))\n'
            test += assertions + '\ntrue\n'
            path = args.output / f"{name}-{language}.splash"
            path.write_text(test)
            result = run(vm, "eval", path, args.output / f"{name}-{language}.log")
            results.append({"case": f"{name}-{language}", "passed": result.returncode == 0})
    # Retain the historical compiler failures as evidence; don't count them as
    # acceptance tests. An eventual framework fix may make these pass.
    for fixture in ("skipped-named-child.splash", "setup-rooms-release7.splash"):
        result = run(vm, "eval", HERE / "fixtures" / fixture, args.output / f"{fixture}.log")
        results.append({"historical_reproducer": fixture, "exit_code": result.returncode})
    (args.output / "results.json").write_text(json.dumps(results, indent=2) + '\n')
    failures = [r["case"] for r in results if r.get("passed") is False]
    assert not failures, failures
    print("12 actual-subtree VM checks passed; historical reproducer results recorded")


if __name__ == "__main__":
    main()
