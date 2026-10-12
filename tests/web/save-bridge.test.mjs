import test from 'node:test';
import assert from 'node:assert/strict';
import {createSaveBridge} from '../../web/save-bridge.js';
import {createSaveSession} from '../../web/save-session.js';
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
test('commit barrier orders later work and ignores discard of committed token',async()=>{
 const host={},runtime=createSaveBridge(host);const commit=runtime.commit(7);runtime.discard(7);const capture=runtime.capture();
 const first=JSON.parse(host.gameSaveRequest());assert.equal(first.kind,'Commit');
 const second=JSON.parse(host.gameSaveRequest());assert.equal(second.kind,'Capture');
 host.gameSaveResult(first.id,'Commit','','');await commit;
 host.gameSaveResult(second.id,'Capture','{}','');await capture;
});
