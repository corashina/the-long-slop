import test from 'node:test';
import assert from 'node:assert/strict';
import {MenuState, ACTIONS} from '../../web/menu.js';

test('controls_back_restores_origin',()=>{
 const m=new MenuState();m.ready();m.controls();m.back();assert.equal(m.state,'start-menu');
 m.started=true;m.pause();m.takeActions();m.controls();m.back();assert.equal(m.state,'pause-menu');assert.equal(m.actions,0);
});
test('capture_failure_keeps_retry',async()=>{
 const m=new MenuState();m.ready();
 await m.capture(ACTIONS.START,async()=>{throw Error('blocked');});
 assert.equal(m.state,'start-menu');assert.equal(m.actions,0);assert.match(m.error,/blocked/);
 await m.capture(ACTIONS.START,async()=>{});assert.equal(m.takeActions(),1);assert.equal(m.state,'playing');
});
test('late_capture_after_blur_stays_paused',async()=>{
 const m=new MenuState();m.ready();m.started=true;m.pause();
 let resolve;const pending=m.capture(ACTIONS.CONTINUE,()=>new Promise(r=>{resolve=r;}));
 m.pause();resolve();await pending;assert.equal(m.state,'pause-menu');assert.equal(m.takeActions()&4,0);
});
test('missing_sprite_disables_start',()=>{
 const m=new MenuState();m.fail('Missing sprite enter');m.ready();assert.equal(m.state,'error');assert.equal(m.readyFlag,false);
});
test('recover_does_not_resume',()=>{
 const m=new MenuState();m.ready();m.started=true;m.pause();m.recover();
 assert.equal(m.state,'pause-menu');assert.equal(m.takeActions(),16);
});


test('scene_loading_rejects_late_capture_and_requires_new_continue',async()=>{
 const m=new MenuState();m.ready();m.started=true;m.pause();m.takeActions();
 let resolve;const pending=m.capture(ACTIONS.RESTART,()=>new Promise(r=>{resolve=r;}));
 m.loading();resolve();assert.equal(await pending,false);
 assert.equal(m.readyFlag,false);assert.equal(m.state,'loading');assert.equal(m.takeActions(),0);
 assert.equal(await m.capture(ACTIONS.CONTINUE,async()=>{}),false);
 m.ready();assert.equal(m.state,'pause-menu');
 assert.equal(await m.capture(ACTIONS.CONTINUE,async()=>{}),true);assert.equal(m.takeActions(),ACTIONS.CONTINUE);
});
test('scene_loading_cannot_reopen_a_terminal_failure',()=>{
 const m=new MenuState();m.ready();m.started=true;m.fail('GPU failed');m.loading();m.ready();
 assert.equal(m.state,'error');assert.equal(m.readyFlag,false);assert.equal(m.takeActions(),ACTIONS.PAUSE);
});
