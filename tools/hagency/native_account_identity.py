#!/usr/bin/env python3
"""Native account grouping and disclosure, using fictional records and read fixtures only."""
import argparse, hashlib, json
from pathlib import Path
from native_widget_families import Family, CASES
from native_pool_application import ROOT

def row(name,identity,count,readiness='unknown'):
    return dict(id=name,identityKey=identity,resourceCount=count,readiness=readiness,kind='managed_namespace',runtime='codex',state='active',ordinal=1,revision='original-revision',profile='codex')
def page(rows):
    return dict(protocol='hagency-management-v1',accounts=rows,account=None,resources=[],nextAfter=None)

class Accounts(Family):
    def visible(self,name):
        width,height=self.req('/s')['w'][0]['sz']
        for attempt in range(40):
            r=self.row(name);x,y,w,h=r['r'];top=self.row('content')['r'][1]+4
            # A clipped snapshot can report a button's visible sliver as its
            # height. Scroll until the entire 44px control can receive a click.
            if w>0 and h>=43 and y>=top and y+h<=height-24:
                assert x>=0 and x+w<=width,(name,r)
                return r
            self.req('/m',k='scroll',x=width/2,y=height-100,dy=-10000 if attempt==0 and h==0 else (-220 if h>0 and y<top else 220),wait=1)
        raise AssertionError('Unreachable complete control '+name)

def main():
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--binary',type=Path,default=ROOT/'target/release/examples/hagency_visual');a=p.parse_args()
    a.output.mkdir(parents=True,exist_ok=True);results=[]
    for language,narrow in [('zh-CN',False),('en',True)]:
        for multiple in [False,True]:
            first=row('account_a','identity_one',1)
            unknown=row('unused_login',None,0,'subscription');unknown['ordinal']=2
            records=[first,unknown]
            if multiple:records += [row('account_b','identity_one',2,'subscription'),row('account_c','identity_two',1)]
            CASES['accounts']=dict(page='provider_accounts',entry_role='owner',account_inventory=page(records))
            out=a.output.resolve()/(language+('-many' if multiple else '-single'))
            app=Accounts(out,a.binary.resolve(),language,narrow,'accounts')
            try:
                detail=page([]);detail['account']=unknown
                app.fixture_path.write_text(json.dumps({'hagency.accounts.get':detail,'hagency.accounts.list':page(records)}))
                app.req('/event',data='ux:reload-fixtures')
                app.wait(lambda:len([r for r in app.snap() if r.get('i')=='account_open'])==(3 if multiple else 1))
                assert app.row('account_prepare_review')['r'][2]>0
                assert app.row('account_open')['r'][1]<app.row('account_pending_toggle')['r'][1] or app.row('account_pending_toggle')['r'][2]==0
                app.shot('01-accounts')
                assert not any(r.get('i')=='account_record_open' and r['r'][2]>0 for r in app.snap())
                app.click('account_pending_toggle')
                app.visible('account_record_open')
                app.visible('account_record_open');app.shot('02-unconfirmed-expanded')
                app.click('account_pending_toggle')
                app.click('account_pending_toggle')
                app.visible('account_record_open')
                # Unknown record uses its original login ID, not a group identity.
                detail=page([]);detail['account']=unknown
                app.fixture_path.write_text(json.dumps({'hagency.accounts.get':detail,'hagency.accounts.list':page(records)}))
                app.req('/event',data='ux:reload-fixtures')
                app.click('account_record_open')
                app.visible('account_login')
                app.visible('account_login');app.shot('03-exact-login-detail')
                app.click('accounts_back')
                app.visible('account_pending_toggle');app.visible('account_record_open')
                assert app.row('account_record_open')['r'][2]>0, 'disclosure state retained on return'
                app.req('/event',data='ux:dark');app.visible('account_pending_toggle');app.shot('04-dark-return')
                results.append(dict(language=language,narrow=narrow,multiple=multiple,passed=True))
            except Exception:
                app.shot('failure');raise
            finally:app.stop()
    (a.output/'report.json').write_text(json.dumps(dict(passed=True,checks=results,coverage="Pointer disclosure, original login target, return state, two languages and widths; keyboard focus persistence is not covered",binarySha256=hashlib.sha256(a.binary.read_bytes()).hexdigest()),indent=2)+'\n')
    print(json.dumps(results))
if __name__=='__main__':main()
