#!/usr/bin/env python3
"""Evaluate the entire shipped bundle and verify its real root layout.

Widget prototypes are inert; this checks VM construction, not pixels. In
particular the streaming parser's EOF auto-close must not swallow startup/root
widgets into an accidentally unclosed function. No source subtree is extracted.
"""
import argparse,json,re,subprocess
from pathlib import Path
APP=Path(__file__).resolve().parents[1]

def fixture(source,language):
 prefix='''use mod.std.assert
use mod.math.*
let base={text:"" draw_bg:{} draw_icon:{} draw_text:{text_style:{}}}
let View=base
let RoundedView=base
let SolidView=base
let ScrollYView={..base body:{}}
let Icon=base
let Fill={}
let Fit=2
let FitBound={Abs:fn(value){return value}}
let Down=3
let Right=4
let Flow={Right:{}}
let Walk={}
let Align={}
let Inset={}
let instance=fn(value){return value}
let uniform=fn(value){return value}
let theme={corner_radius:4 font_bold:{}}
mod.widgets={}
mod.rinx_icons={}
mod.rinx_theme={reloading:fn(){return false}}
'''
 prefix+='mod.rinx_i18n={language:fn(){return '+json.dumps(language)+'}}\n'
 prefix+='''let timers=[]
let start_timeout=fn(delay,callback){timers.push(callback)}
let host={request:fn(service,args,callback){}}
'''
 for key in sorted(set(re.findall(r'mod\.widgets\.([A-Za-z_0-9]+)',source))):
  value='1' if key.startswith('RINX_') else '{..base body:{page:{}}}' if key=='RinxPageScroll' else 'base'
  prefix+=f'mod.widgets.{key}={value}\n'
 for key in sorted(set(re.findall(r'mod\.rinx_icons\.([A-Za-z_0-9]+)',source))):prefix+=f'mod.rinx_icons.{key}=1\n'
 return prefix+'View{\n'+source+'\n}\n'

def main():
 p=argparse.ArgumentParser();p.add_argument('vm',type=Path);p.add_argument('--bundle',type=Path,default=APP/'bundle/main.splash');p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 source=a.bundle.read_text();results=[]
 for language in ['en','zh-CN']:
  script=a.output/(language+'.splash');script.write_text(fixture(source,language))
  r=subprocess.run([str(a.vm.resolve()),'root',str(script)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(a.output/(language+'.log')).write_text(r.stdout);results.append({'language':language,'passed':r.returncode==0})
 (a.output/'report.json').write_text(json.dumps({'bundle':str(a.bundle),'checks':results},indent=2)+'\n');print(json.dumps({'passed':sum(r['passed'] for r in results),'total':len(results)}));raise SystemExit(not all(r['passed'] for r in results))
if __name__=='__main__':main()
