#!/usr/bin/env python3
"""Check alert faces and corner ownership in the pinned VM; pixels need a native verifier."""
import argparse
import json
from pathlib import Path
import subprocess
from check_full_bundle import APP, fixture

p = argparse.ArgumentParser()
p.add_argument('vm', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=True)
source = (APP/'bundle/main.splash').read_text()
styles = source[source.index('mod.widgets.HagencyAlert ='):source.index('mod.widgets.HagencySkeleton =')]
start = source.index('        if kind=="policy" || kind=="model" || kind=="pool"')
views = source[start:source.index('        if kind=="failure"', start)]
# A flat pixel function intentionally ignores the platform bevel/gradient/material paths.
face = styles[styles.index('pixel:fn()'):styles.index('mod.widgets.HagencyAlertBold')]
assert 'sdf.box_all(' in face and 'sdf.stroke' not in face and 'face_fill' not in face
assert 'self.color.mix(self.color_hover,self.hover).mix(self.color_down,self.down)' in face
setup = '''
theme.font_bold={weight:700}
mod.widgets.RINX_FIELD=10 mod.widgets.RINX_SURFACE=11 mod.widgets.RINX_HOVER=12 mod.widgets.RINX_PRESSED=13 mod.widgets.RINX_DANGER_FG=14 mod.widgets.RINX_ACCENT=15
let H={local_pending:"" local_discard:"" local_after:"" local_confirm:""}
let A={t:fn(en,zh){return if mod.rinx_i18n.language()=="en" {en} else {zh}} local_discard_changes:fn(){} redraw:fn(){} local_encrypt:fn(){}}
let find=fn(value,id){
    if !value.is_object(){return nil}
    for i key child in value {
        if key.to_string()==id {return child}
        let found=find(child,id)
        if found!=nil {return found}
    }
    return nil
}
'''
checks = '''
assert(mod.widgets.HagencyAlert.height==Fit)
assert(mod.widgets.HagencyAlert.spacing==0 && mod.widgets.HagencyAlert.padding==0)
assert(mod.widgets.HagencyAlert.draw_bg.border_radius==7.0)
assert(mod.widgets.HagencyAlert.draw_bg.color==if mod.widgets.RINX_DARK {10} else {11})
for id in ids {
    let button=find(card,id)
    assert(button!=nil)
    assert(button.draw_bg.color_hover==12 && button.draw_bg.color_down==13)
    assert(button.draw_bg.color==#0000 && button.draw_bg.color_focus==#0000)
    assert(button.draw_bg.color_2==#0000 && button.draw_bg.color_2_hover==#0000 && button.draw_bg.color_2_down==#0000)
    assert(button.draw_bg.color_disabled==#0000 && button.draw_bg.color_2_disabled==#0000)
}
let left=find(card,ids[0]) let right=find(card,ids[1])
assert(left.draw_bg.border_radius_bl==7.0)
assert(right.draw_bg.border_radius_br==7.0)
if kind!="encrypt" {
    let discard=find(card,"local_discard_confirm")
    assert(discard.draw_text.color==14 && discard.draw_text.color_focus==14)
    let keep=find(card,"local_discard_keep")
    assert(keep.draw_text.text_style.weight==700)
    H.local_discard=kind H.local_after="pending"
    keep.on_click()
    assert(H.local_discard=="" && H.local_after=="")
}

'''
rows = []
for language in ['en', 'zh-CN']:
 for dark in [False, True]:
  for kind in ['model', 'policy', 'pool', 'encrypt']:
   ids = ['local_encrypt_cancel', 'local_encrypt_confirm'] if kind=='encrypt' else ['local_discard_keep', 'local_discard_confirm'] if kind=='pool' else ['local_discard_keep', 'local_discard_keep']
   script = fixture(setup + f'let kind="{kind}"\nmod.widgets.RINX_DARK={str(dark).lower()}\n' + styles + '\nlet card=mod.widgets.HagencyAlert{\n' + views + '\n}\nlet ids=' + json.dumps(ids) + '\n' + checks, language) + '\ntrue\n'
   path = a.output/f'{language}-{dark}-{kind}.splash'
   path.write_text(script)
   result = subprocess.run([str(a.vm.resolve()), 'eval', str(path)], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
   path.with_suffix('.log').write_text(result.stdout)
   rows.append({'language':language, 'dark':dark, 'kind':kind, 'passed':result.returncode==0})
(a.output/'report.json').write_text(json.dumps(rows, indent=2)+'\n')
print(json.dumps({'passed':sum(row['passed'] for row in rows), 'total':len(rows)}))
raise SystemExit(not all(row['passed'] for row in rows))
