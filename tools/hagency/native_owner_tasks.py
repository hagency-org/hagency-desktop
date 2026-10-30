#!/usr/bin/env python3
"""Three owner journeys in the real desktop renderer, using fictional read fixtures.

No live login, budget allocation, approval or deletion is performed.
"""
import argparse, hashlib, json, time
from pathlib import Path
from native_account_identity import Accounts, row, page
from native_widget_families import CASES, SESSION, MODEL
from native_pool_application import ROOT

ACCOUNT=row('login_original','identity_original',1,'subscription')
RESOURCE=dict(id='resource_original',revision='resource-revision',model='coding-model',framework='codex',reasoning='medium',published=False,capabilities=['coding','documentation'],localName=dict(name='Research · 研究资源池',revision=1),budget=dict(remainingTokens=750000,reserved=250000,pool=dict(ceiling=1000000,committed=250000,period='monthly')),usage=dict(spent=180000,spend_period_key='2026-10'))
CONNECTION=dict(id='pool_original',label='Research · 研究资源池',resourceId=RESOURCE['id'],ownerMxid=SESSION['account'],coordinatorMxid=SESSION['account'],executorDeviceId=SESSION['device'],executorGeneration=SESSION['deviceGeneration'],state='admitted',runtime='codex',capabilities=['coding'],verification=None)
REQUEST=dict(id='request_original',revision=17,poolId=CONNECTION['id'],poolLabel=CONNECTION['label'],resourceOwnerMxid=SESSION['account'],managerMxid='@applicant:example.test',definition=dict(name='小蓝 · Research assistant',description='Research with references / 研究并提供引用',capability='coding'),requestedTokens=100000,state='pending_approval',reservationReason=None,blockingReason=None,assignmentId=None,membershipState=None,runtimeState=None,reservationState=None,canDecide=True,canCancel=False,canStopSupply=False)
DETAIL=dict(protocol='hagency-management-v1',resource=RESOURCE,editing=dict(allowed=True,reason=None),choices=[dict(model='coding-model',reasoning='medium')],executionPolicy=dict(yolo=False))

def fixtures():
 detail=page([]);detail.update(account=ACCOUNT,resources=[RESOURCE])
 return {
 'hagency.accounts.list':page([ACCOUNT]),
 'hagency.accounts.get':{'_cases':[{'when':{'id':ACCOUNT['id']},'response':detail}]},
 'hagency.accounts.models':dict(protocol='hagency-management-v1',accountId=ACCOUNT['id'],revision=ACCOUNT['revision'],enrollmentReady=True,provider=dict(runtime='codex',connected=True,models=[MODEL,dict(MODEL,model="gpt-6.1-terra",label="GPT-6.1 Terra"),dict(MODEL,model="gpt-6-luna",label="GPT-6 Luna")])),
 'hagency.intent.pending':dict(commands=[]),
 'hagency.resources.list':dict(resources=[RESOURCE],nextAfter=None),
 'hagency.resources.get':{'_cases':[{'when':{'id':RESOURCE['id']},'response':DETAIL}]},
 'hagency.connections.list':dict(items=[CONNECTION],nextAfter=None,observedAtMs=0),
 'hagency.pools.list':dict(items=[dict(id=CONNECTION['id'],ownerMxid=SESSION['account'],publishedTokens=None,grantRevision=None)],nextAfter=None),
 'hagency.connections.get':{'_cases':[{'when':{'id':CONNECTION['id']},'response':dict(admission=dict(contribution=CONNECTION,request=dict(canDecide=False,approverMxid='@admin:example.test')),observedAtMs=0)}]},
 'hagency.pools.agents.list':{'_cases':[{'when':{'poolId':CONNECTION['id']},'response':dict(items=[REQUEST],nextAfter=None)},{'when':{},'response':dict(items=[REQUEST],nextAfter=None)}]},
 'hagency.pools.agents.get':{'_cases':[{'when':{'id':REQUEST['id']},'response':dict(request=REQUEST)}]},
 'hagency.onboarding.status':dict(state='active',canReview=False,grantReviewAvailable=False,grantIntents=[]),
 }

class Owner(Accounts):
 def visible(self,name):
  if name in {'entry_switch','refresh','accounts','resources','inbox','pool_agent_back'}:
   r=self.row(name);assert r['r'][2]>0 and r['r'][3]>=43;return r
  return super().visible(name)
 def set_fixtures(self,data):
  self.fixture_path.write_text(json.dumps(data));self.req('/event',data='ux:reload-fixtures')
 def text_present(self,text):return any(text in r.get('t','') for r in self.snap())
 def run(self):
  f=fixtures();self.set_fixtures(f)
  self.wait(lambda:self.row('account_open')['r'][2]>0)
  assert self.nav()=={'accounts','resources','inbox'}
  self.shot('01-coding-tools')
  self.click('account_open');self.wait(lambda:self.row('account_tab_servers')['r'][2]>0)
  self.shot('02-login-status')
  assert not any(r.get('i')=='account_models_check' and r['r'][2]>0 for r in self.snap())
  self.click('account_tab_login');time.sleep(.15)
  # Continue with keyboard after content was rebuilt. Tab => Servers, Enter.
  self.req('/k',c='Tab',wait=1);self.req('/k',c='ReturnKey',wait=1)
  self.wait(lambda:self.row('account_server')['r'][2]>0)
  self.shot('03-server-tab-keyboard')
  self.click('account_tab_pools');self.click('account_models_check')
  self.wait(lambda:self.row('account_model_enroll')['r'][2]>0)
  self.shot('04-model-selection')
  self.click('account_model_enroll');self.wait(lambda:self.row('enrollment_tokens')['r'][2]>0)
  self.field('input','0.345678',True)
  assert self.row('enrollment_tokens')['t']=='345678'
  self.click('enrollment_model_toggle')
  choice=next(r for r in self.snap() if r.get('i')=='enrollment_model_choice' and r.get('t')=='GPT-6.1 Terra')
  # Bring the named model into view through its matching row, then click.
  for _ in range(12):
   choice=next(r for r in self.snap() if r.get('i')=='enrollment_model_choice' and r.get('t')=='GPT-6.1 Terra')
   x,y,w,h=choice['r']
   if h>0 and 250<=y and y+h<self.req('/s')['w'][0]['sz'][1]-20:break
   self.req('/m',k='scroll',x=200,y=600,dy=180,wait=1)
  self.click_row(choice)
  assert self.text_present('GPT-6.1 Terra')
  self.visible('enrollment_tokens');assert self.row('enrollment_tokens')['t']=='345678'
  self.click('amount_slider');self.req('/k',c='End',wait=1)
  self.wait(lambda:self.row('enrollment_tokens')['t']=='10000000')
  self.req('/k',c='ArrowLeft',wait=1);self.wait(lambda:self.row('enrollment_tokens')['t']=='9900000')
  self.field('input','0.445678',True)
  self.wait(lambda:self.row('enrollment_tokens')['t']=='445678')
  self.visible('enrollment_tokens');self.shot('05-create-pool-widgets')
  # No enrollment mutation: return to the same account and inspect existing pools.
  self.click('enrollment_back');self.click('account_tab_pools')
  self.click('owner_pool_continue');self.wait(lambda:self.row('enrollment_tokens')['t']=='445678')
  self.click('enrollment_back');self.click('account_tab_pools')
  self.click('owner_account_pools');self.visible('resource_detail')
  assert self.text_present('0.75 M')
  self.shot('06-pools-scoped-to-tool')
  self.click('resource_detail');self.wait(lambda:self.row('resource_alerts_open')['r'][2]>0)
  # The model and limit editor is reached from the pool page's Model row.
  self.visible('owner_pool_config');self.shot('07-pool-management')
  self.click('resource_publish_review');self.wait(lambda:self.row('resource_publish_confirm')['r'][2]>0);self.shot('07b-catalogue-alert')
  self.click('resource_publish_cancel')
  # One status-aware Server and publishing row replaces the Servers tile.
  assert not any(r.get('i')=='resource_all_connections' for r in self.snap())
  self.visible('resource_alerts_open');self.shot('07c-pool-manage-tiles')
  self.visible('resource_publish_review');self.shot('07d-pool-on-device')
  self.req('/m',k='scroll',x=200,y=400,dy=-10000,wait=1)
  # The server page is a detail page: wait for it, then scroll to each control.
  self.click('resource_connections');self.wait(lambda:any(r.get('i')=='owner_connection_requests' for r in self.snap()))
  self.visible('owner_connection_capacity');self.shot('08-server-connection')
  assert not any(r.get('i')=='verify' and r['r'][2]>0 and r['r'][3]>0 for r in self.snap())
  self.click('connection_diagnostics');self.visible('verify')
  self.click('connection_diagnostics')
  # A stale diagnostic is not a withdrawn grant. Owners need not be applicants.
  published=dict(id=CONNECTION['id'],ownerMxid=SESSION['account'],publishedTokens=1000000,grantRevision=1,canRequest=False,blockingReason='pool_not_shared_with_account')
  current=dict(CONNECTION,verification=dict(status='stale',verifiedAtMs=100,freshUntilMs=300100))
  f['hagency.connections.get']={'_cases':[{'when':{'id':CONNECTION['id']},'response':dict(admission=dict(contribution=current,request=dict(canDecide=False,approverMxid="@admin:example.test")),observedAtMs=400000)}]}
  f['hagency.pools.list']=dict(items=[published],nextAfter=None)
  self.set_fixtures(f);self.click('refresh')
  self.wait(lambda:self.text_present('本月已发布') or self.text_present('Published for this month'))
  self.visible('owner_connection_capacity')
  assert self.text_present('1 M')
  assert not self.text_present('Connection needs a fresh check') and not self.text_present('连接需要重新检查')
  self.shot('08b-published-with-expired-check')
  f['hagency.pools.list']={'_cases':[{'when':{},'error':'management_unavailable'}]}
  self.set_fixtures(f);self.click('refresh')
  self.wait(lambda:self.text_present('暂时无法确认已发布的额度') or self.text_present('Couldn’t confirm what is published'))
  assert not any(r.get('i')=='owner_connection_capacity' and r['r'][2]>0 and r['r'][3]>0 for r in self.snap())
  self.visible('owner_capacity_retry');self.shot('08c-capacity-read-error')
  f['hagency.pools.list']=dict(items=[dict(published,publishedTokens=None,grantRevision=None)],nextAfter=None)
  self.set_fixtures(f);self.click('owner_capacity_retry')
  self.wait(lambda:self.text_present('本月还没发布') or self.text_present('Not published this month'))
  self.visible('owner_connection_capacity');self.shot('08d-capacity-no-longer-active')
  f['hagency.pools.list']=dict(items=[published],nextAfter=None)
  self.set_fixtures(f);self.click('refresh')
  self.wait(lambda:self.text_present('本月已发布') or self.text_present('Published for this month'))
  self.click('owner_connection_requests');self.wait(lambda:self.row('owner_request_open')['r'][2]>0)
  assert self.text_present(CONNECTION['label']) and self.text_present('0.1 M')
  self.shot('09-pool-agent-requests')
  self.click('owner_request_open');self.visible('owner_approve');self.shot('10-request-review')
  self.click('owner_approve');self.visible('owner_decision_cancel');self.shot('11-confirmation')
  self.click('owner_decision_cancel');self.click('owner_reject');self.click('owner_decision_cancel')
  # Back returns to the pool's own request list, where the filter lives.
  self.click('pool_agent_back');self.wait(lambda:self.row('owner_history')['r'][2]>0)
  self.click('owner_history');self.wait(lambda:self.row('owner_request_open')['r'][2]>0)
  self.shot('12-filter-retained')
  # Real read failure, then empty inventory and recovery, with action callbacks.
  f['hagency.resources.list']={'_cases':[{'when':{},'error':'management_busy'}]};self.set_fixtures(f)
  self.click('resources');self.wait(lambda:self.row('owner_pool_all')['r'][2]>0)
  self.click('owner_pool_all');self.wait(lambda:self.text_present('could not be loaded') or self.text_present('暂时无法加载'))
  self.shot('13-read-error')
  f['hagency.resources.list']=dict(resources=[],nextAfter=None);self.set_fixtures(f)
  self.click('refresh');self.wait(lambda:self.row('create')['r'][2]>0);self.shot('14-empty-pools')
  self.set_fixtures(fixtures());self.click('accounts');self.wait(lambda:self.row('account_open')['r'][2]>0)
  self.req('/event',data='ux:dark');time.sleep(.4);self.shot('15-dark')
  self.req('/event',data='ux:violet');time.sleep(.4);self.shot('16-violet')

def main():
 p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--binary',type=Path,default=ROOT/'target/release/examples/hagency_visual');p.add_argument('--one',action='store_true');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);results=[]
 for lang,narrow in ([('zh-CN',False)] if a.one else [('zh-CN',False),('en',True),('zh-CN',True),('en',False)]):
  CASES['owner_tasks']=dict(page='provider_accounts',entry_role='owner',account_inventory=page([ACCOUNT]))
  out=a.output.resolve()/(lang+('-narrow' if narrow else '-wide'));app=Owner(out,a.binary.resolve(),lang,narrow,'owner_tasks')
  try:app.run()
  except Exception:app.shot('failure');raise
  finally:app.stop()
  results.append(dict(language=lang,narrow=narrow,passed=True));print(results[-1],flush=True)
  (a.output/'report.json').write_text(json.dumps(dict(checks=results,sourceSha256=hashlib.sha256((ROOT/'apps/hagency/bundle/main.splash').read_bytes()).hexdigest(),binarySha256=hashlib.sha256(a.binary.read_bytes()).hexdigest(),scope='Native UI with read fixtures; live provider login, capacity publication and approval effects are not executed.'),indent=2)+'\n')
if __name__=='__main__':main()
