#!/usr/bin/env python3
"""Real native Invite members controls, with isolated offline completions."""
import argparse
import time
from urllib.error import HTTPError
from pathlib import Path
from native_review import Review, CASES

class InviteReview(Review):
    def capture(self, name, window=0):
        # A hidden Metal window can be between drawable frames. Retry only the
        # capture, never an interaction or a failed state assertion.
        for attempt in range(3):
            try:
                return super().capture(name, window)
            except HTTPError as error:
                if error.code != 404 or attempt == 2:
                    raise
                time.sleep(.3)

    def scenario(self, name):
        self.request('/event', data='invite:' + name)
        time.sleep(.15)

    def status(self):
        return self.widget('status_label')['t']

    def enabled(self):
        return self.widget('confirm_button').get('enabled', True)

    def open(self):
        self.click('invite_people')
        assert self.widget('title')['t'] == ('邀请加入 Team room' if self.language == 'zh-CN' else 'Invite to Team room')
        self.footer('cancel_button')
        self.footer('confirm_button')

    def closed(self):
        assert not any(w.get('i') == 'user_id_input' and w.get('r', [0,0,0,0])[2] > 0 for w in self.snap())

    def press_return(self, ident='user_id_input'):
        self.click(ident)
        self.request('/k', w=0, c='Return', wait=1)
        time.sleep(.15)

    def visible(self, ident, text=None):
        return [w for w in self.snap() if w.get('i') == ident and w.get('r', [0,0,0,0])[2] > 0
                and (text is None or w.get('t') == text)]

    def click_text(self, ident, text):
        x, y, width, height = self.visible(ident, text)[0]['r']
        self.request('/click', w=0, x=x+width/2, y=y+height/2, wait=1)
        time.sleep(.15)

    def count_text(self, count):
        if self.language == 'zh-CN':
            return f'邀请 {count} 人'
        return 'Invite 1 person' if count == 1 else f'Invite {count} people'

    def run(self):
        self.request('/event', data='picker:members')
        time.sleep(.15)
        self.open()
        self.scenario('many-people')
        field = self.widget('user_id_input')['r']
        assert not self.enabled(), 'Nothing selected: Invite is disabled'
        self.capture('people-list')
        first_id = next(w for w in self.snap() if w.get('i') == 'person_id' and w.get('t', '').startswith('@person_00:'))
        assert first_id['r'][3] >= 14, 'Identity text must have room to render, not just exist in the snapshot'
        # The last of 80 entries must be reachable inside the dialog. It may not
        # cover the search field or footer, even with long federated identities.
        x, y, width, height = self.widget('people_list')['r']
        action = self.widget('confirm_button')['r']
        assert 0 <= x and x+width <= self.width+1
        assert field[1]+field[3] <= y and y+height <= action[1]
        self.request('/m', w=0, k='scroll', x=x+width/2, y=y+height/2,
                     dy=10000, precise=1, wait=1)
        time.sleep(.3)
        self.capture('people-scrolled')
        last = next(w for w in self.snap() if w.get('i') == 'person_name' and w.get('t') == 'Friend 79 · 同名测试')
        lx, ly, lw, lh = last['r']
        assert y <= ly+lh/2 < y+height
        self.request('/click', w=0, x=lx+lw/2, y=ly+lh/2, wait=1)
        time.sleep(.15)
        assert self.visible('chip_name', 'Friend 79 · 同名测试'), 'A picked row joins the selection as a chip'
        assert self.widget('user_id_input')['val'] == '', 'Picking a row does not overwrite the search field'
        assert self.enabled() and self.widget('confirm_button')['t'] == self.count_text(1)
        self.fill('user_id_input', 'Friend 42')
        self.capture('people-filtered')
        assert self.widget('person_name')['t'] == 'Friend 42 · 同名测试'
        assert self.widget('user_id_input')['r'] == field, 'Filtering must not move the input'
        action = self.widget('confirm_button')['r']
        self.click_text('person_name', 'Friend 42 · 同名测试')
        assert self.widget('confirm_button')['t'] == self.count_text(2)
        self.capture('two-selected')
        self.click_text('person_name', 'Friend 42 · 同名测试')
        assert not self.visible('chip_name', 'Friend 42 · 同名测试'), 'Picking a selected row again deselects it'
        assert self.widget('confirm_button')['t'] == self.count_text(1)
        assert self.widget('confirm_button')['r'] == action, 'Filtering and toggling must not move the footer'
        self.fill('user_id_input', 'no-such-person')
        assert not self.visible('person_name')
        assert self.widget('user_id_input')['r'] == field
        self.click_text('chip_name', 'Friend 79 · 同名测试')
        assert not self.visible('chip_name'), 'A chip removes its invitee'
        assert not self.enabled()
        self.footer('confirm_button')
        self.click('cancel_button')
        self.checks.append('80 people scroll within modal; rows toggle a removable chip selection; count on Invite; filtering keeps controls stable')
        self.open()
        assert not self.enabled()
        self.press_return()
        assert self.status() == ('请输入名称或 Matrix ID。' if self.language == 'zh-CN' else 'Enter a name or Matrix ID.')
        self.capture('empty-input')
        self.fill('user_id_input', '@invalid')
        assert not self.enabled(), 'An incomplete ID is not counted'
        self.press_return()
        assert ('无效' in self.status() if self.language == 'zh-CN' else 'Invalid' in self.status())
        self.capture('invalid-id')
        self.fill('user_id_input', '白菜')
        self.press_return()
        assert self.widget('user_id_input')['val'] == ''
        assert self.visible('chip_name', '小白菜'), 'A resolved name joins the selection'
        assert self.widget('confirm_button')['t'] == self.count_text(1)
        self.click('confirm_button')
        assert not self.enabled()
        assert self.status() == ('正在将 Agent 添加到此聊天室…' if self.language == 'zh-CN' else 'Adding Agent to this room…')
        for stale in ['stale','other-room']:
            before=self.status()
            self.scenario(stale)
            assert self.status() == before and not self.enabled()
        self.scenario('slow')
        self.capture('waiting')
        assert not self.enabled()
        self.click('cancel_button')
        self.closed()
        self.checks.append('Member button opens dialog; empty/invalid input shows errors; partial name resolves into the selection; stale results ignored; waiting can close')
        self.open()
        self.fill('user_id_input', '@_hagency_agt_cabbage:example.invalid')
        assert self.enabled() and self.widget('confirm_button')['t'] == self.count_text(1), 'A typed complete ID is counted'
        self.click('confirm_button')
        self.scenario('agent_service_invite_permission_required')
        assert self.enabled()
        assert ('权限' in self.status() if self.language == 'zh-CN' else 'permission' in self.status())
        self.capture('permission-error')
        self.click('confirm_button')
        self.scenario('pending')
        assert self.enabled()
        assert self.widget('confirm_button')['t'] == ('检查状态' if self.language == 'zh-CN' else 'Check status')
        self.capture('pending')
        self.click('confirm_button')
        self.scenario('success')
        self.footer('okay_button')
        assert self.status() == ('Agent 已加入此聊天室。在群聊中，请 @ Agent 让它回答。' if self.language == 'zh-CN' else 'Agent joined this room. In group chats, @ the Agent to ask it to reply.')
        self.capture('joined')
        self.click('okay_button')
        self.closed()
        self.checks.append('Permission error releases busy state; saved pending can check status; confirmed join shows Done')
        self.open()
        self.fill('user_id_input', '@_hagency_agt_expired:example.invalid')
        self.click('confirm_button')
        self.scenario('agent_owner_binding_required')
        assert self.enabled()
        assert ('资源授权' in self.status() if self.language == 'zh-CN' else 'resource authorization' in self.status())
        self.capture('unavailable-agent')
        self.click('cancel_button')
        self.checks.append('Unavailable Agent points to ownership and resource authorization; does not claim the invite succeeded')
        self.open()
        self.fill('user_id_input', '@friend:example.invalid')
        self.click('confirm_button')
        self.scenario('human')
        assert self.status() == ('邀请已发送，等待对方加入。' if self.language == 'zh-CN' else 'Invitation sent. Waiting for them to join.')
        self.capture('human-invited')
        self.click('okay_button')
        self.closed()
        self.checks.append('Human invite is clearly waiting for acceptance')
        self.open()
        self.click_text('person_name', '小白菜')
        self.click_text('person_name', 'Alex')
        self.fill('user_id_input', '@third:example.invalid')
        self.press_return()
        assert self.visible('chip_name', 'third')
        assert self.widget('confirm_button')['t'] == self.count_text(3)
        self.capture('three-selected')
        self.click('confirm_button')
        assert self.status() == ('正在发送 3 个邀请…' if self.language == 'zh-CN' else 'Sending 3 invitations…')
        self.scenario('mixed')
        assert len(self.visible('result_name')) == 3, 'Each invitee reports its own result'
        assert self.visible('result_error'), 'The failure is reported for its invitee'
        assert ('1 个失败' in self.status() if self.language == 'zh-CN' else '1 of 3 invitations failed' in self.status())
        assert self.enabled() and self.widget('confirm_button')['t'] == self.count_text(1), 'Only the failure is sent again'
        self.capture('partial-failure')
        self.click('confirm_button')
        assert self.status() == ('正在发送邀请…' if self.language == 'zh-CN' else 'Sending invite...')
        self.scenario('human')
        self.footer('okay_button')
        assert self.status() == ('已完成 3 个邀请。' if self.language == 'zh-CN' else '3 invitations completed.')
        self.capture('all-invited')
        self.click('okay_button')
        self.closed()
        self.checks.append('Two rows and a typed ID invite in one confirmation; per-invitee results; a partial failure resends only the failure')
        log=(self.output/'native.log').read_text()
        for invalid in ['Expected expression','property crate not defined','panicked at']:
            assert invalid not in log, invalid

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--case',choices=CASES,required=True)
    args=parser.parse_args()
    review=InviteReview(args.binary,args.output,CASES[args.case])
    passed=False
    try:
        review.run()
        passed=True
    finally:
        review.close(passed)

if __name__ == '__main__':
    main()
