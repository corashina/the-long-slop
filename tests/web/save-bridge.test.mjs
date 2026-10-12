import test from 'node:test';
import assert from 'node:assert/strict';
import {createSaveBridge} from '../../web/save-bridge.js';
import {createSaveSession} from '../../web/save-session.js';
import {MenuState} from '../../web/menu.js';
test('requests drain once and results settle only matching IDs and kinds',async()=>{
 const host={},runtime=createSaveBridge(host);
 assert.equal(host.gameHasSaveRequest(),false);
 const capture=runtime.capture();const request=JSON.parse(host.gameSaveRequest());
 assert.equal(request.kind,'Capture');assert.equal(host.gameHasSaveRequest(),false);
 host.gameSaveResult(request.id,'Prepare','{}','');
 host.gameSaveResult(request.id,'Capture','{"value":1}','');
 assert.deepEqual(await capture,{value:1});
});
test('missing_asset_rejects_before_swap',async()=>{
 const host={},runtime=createSaveBridge(host);const pending=runtime.prepare({missing:true});
 const request=JSON.parse(host.gameSaveRequest());
 host.gameSaveResult(request.id,'Prepare','','Missing source asset: reference/missing.glb');
 await assert.rejects(pending,/reference\/missing.glb/);assert.equal(host.gameHasSaveRequest(),false);
});
test('late_prepare_after_cancel_is_discarded',async()=>{
 const host={},runtime=createSaveBridge(host);const session=createSaveSession({runtime,storage:{}});
 const pending=session.newGame();const request=JSON.parse(host.gameSaveRequest());session.cancel();
 host.gameSaveResult(request.id,'PrepareNewGame','7','');await pending;
 const discard=JSON.parse(host.gameSaveRequest());assert.equal(discard.kind,'Discard');assert.equal(discard.payload,7);
 host.gameSaveResult(discard.id,'Discard','','');assert.equal(host.gameHasSaveRequest(),false);
});
test('cancel_queued_commit_discards_before_next_prepare',async()=>{
 const host={},runtime=createSaveBridge(host),states=[];
 const session=createSaveSession({runtime,storage:{},onStatus:s=>states.push([s.state,session.pending])});
 const first=session.newGame(),prepare=JSON.parse(host.gameSaveRequest());
 host.gameSaveResult(prepare.id,prepare.kind,'7','');await Promise.resolve();
 session.cancel();assert.equal(session.pending,false);
 const second=session.newGame();
 const discard=JSON.parse(host.gameSaveRequest());assert.equal(discard.kind,'Discard');assert.equal(discard.payload,7);
 host.gameSaveResult(discard.id,discard.kind,'','');await first;assert.equal(session.pending,true);
 const next=JSON.parse(host.gameSaveRequest());assert.equal(next.kind,'PrepareNewGame');
 host.gameSaveResult(next.id,next.kind,'8','');await Promise.resolve();
 const commit=JSON.parse(host.gameSaveRequest());assert.equal(commit.kind,'Commit');assert.equal(commit.payload,8);
 host.gameSaveResult(commit.id,commit.kind,'','');await second;
 assert.deepEqual(states,[['pending',true],['canceled',false],['pending',true],['success',false]]);
 assert.equal(host.gameHasSaveRequest(),false);
});
test('cancel_after_synchronous_dispatch_keeps_pending_and_completion',async()=>{
 const host={},runtime=createSaveBridge(host),states=[];
 const session=createSaveSession({runtime,storage:{},onStatus:s=>states.push([s.state,session.pending])});
 const operation=session.newGame(),prepare=JSON.parse(host.gameSaveRequest());
 host.gameSaveResult(prepare.id,prepare.kind,'7','');await Promise.resolve();
 const commit=JSON.parse(host.gameSaveRequest());assert.equal(commit.kind,'Commit');
 session.cancel();assert.equal(session.pending,true);
 await assert.rejects(session.newGame(),/pending/);
 host.gameSaveResult(commit.id,commit.kind,'','');await operation;
 assert.deepEqual(states,[['pending',true],['success',false]]);assert.equal(host.gameHasSaveRequest(),false);
});
test('menu_keeps_dispatched_replacement_pending_through_cancel_and_pause',async()=>{
 const host={},runtime=createSaveBridge(host),session=createSaveSession({runtime,storage:{}}),menu=new MenuState();
 menu.ready();menu.started=true;menu.pause();
 const operation=menu.run('newGame',session),prepare=JSON.parse(host.gameSaveRequest());
 host.gameSaveResult(prepare.id,prepare.kind,'7','');await Promise.resolve();
 const commit=JSON.parse(host.gameSaveRequest());session.cancel();menu.pause();
 assert.equal(session.pending,true);assert.equal(menu.pending,true);assert.equal(menu.status,'Starting a new game...');
 host.gameSaveResult(commit.id,commit.kind,'','');await operation;
 assert.equal(menu.pending,false);assert.equal(menu.status,'New game ready.');assert.equal(menu.state,'pause-menu');
});
