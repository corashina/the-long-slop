import test from 'node:test';
import assert from 'node:assert/strict';
import {webcrypto as crypto} from 'node:crypto';
import {createSaveSession} from '../../web/save-session.js';
const deferred=()=>{let resolve;const promise=new Promise(r=>{resolve=r;});return {promise,resolve};};
const tick=()=>new Promise(resolve=>setImmediate(resolve));
async function record(payload){const hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(JSON.stringify(payload)));return {version:1,payload,checksum:Array.from(new Uint8Array(hash),b=>b.toString(16).padStart(2,'0')).join('')};}
function setup(overrides={}){
 const calls={commit:[],discard:[],write:[],status:[]};const runtime={capture:async()=>({schema_version:2}),prepare:async()=>7,commit:async(token,onDispatch)=>{onDispatch();calls.commit.push(token);},discard:token=>calls.discard.push(token),prepareNewGame:async()=>8,...overrides.runtime};
 const storage={read:async()=>await record({schema_version:2}),write:async save=>{calls.write.push(save);},...overrides.storage};
 const session=createSaveSession({runtime,storage,onStatus:status=>calls.status.push(status)});return {calls,session};
}
test('save_load_and_new_game_finish_and_clear_pending',async()=>{
 const {session,calls}=setup();await session.save();await session.load();await session.newGame();assert.equal(session.pending,false);assert.equal(calls.write.length,1);assert.deepEqual(calls.commit,[7,8]);assert.equal(calls.status.at(-1).state,'success');
});
test('terminal_status_observes_pending_false',async()=>{
 let session;const states=[];
 session=createSaveSession({storage:{write:async()=>{}},runtime:{capture:async()=>({})},onStatus:status=>states.push([status.state,session.pending])});
 await session.save();assert.deepEqual(states,[['pending',true],['success',false]]);
});
test('one_pending_operation_rejects_overlap',async()=>{
 const capture=deferred();const {session}=setup({runtime:{capture:()=>capture.promise}});const save=session.save();assert.equal(session.pending,true);await assert.rejects(session.load(),/pending/i);capture.resolve({});await save;assert.equal(session.pending,false);
});
test('canceled_preparation_discards_stale_candidate_without_commit',async()=>{
 const candidate=deferred();const {session,calls}=setup({runtime:{prepareNewGame:()=>candidate.promise}});const operation=session.newGame();session.cancel();assert.equal(session.pending,false);candidate.resolve(11);await operation;assert.deepEqual(calls.discard,[11]);assert.equal(calls.commit.length,0);
});
test('stale_capture_does_not_write_or_clear_later_pending_operation',async()=>{
 const capture=deferred(),candidate=deferred();const {session,calls}=setup({runtime:{capture:()=>capture.promise,prepareNewGame:()=>candidate.promise}});const first=session.save();session.cancel();const second=session.newGame();capture.resolve({});await first;assert.equal(session.pending,true);assert.equal(calls.write.length,0);candidate.resolve(9);await second;assert.deepEqual(calls.commit,[9]);
});
test('delayed_canceled_write_cannot_overwrite_a_later_save',async()=>{
 const delayed=deferred();let stored;let count=0;let signal;const {session}=setup({runtime:{capture:async()=>({value:++count})},storage:{write:async(save,options)=>{signal=options.signal;if(save.value===1)await delayed.promise;stored=save;}}});
 const first=session.save();await tick();session.cancel();assert.equal(signal.aborted,true);const second=session.save();await tick();assert.equal(stored,undefined);delayed.resolve();await Promise.all([first,second]);assert.deepEqual(stored,{value:2});assert.equal(session.pending,false);
});
test('failed_load_keeps_runtime_and_reports_error',async()=>{
 const {session,calls}=setup({storage:{read:async()=>({version:1,payload:{},checksum:'bad'})}});await assert.rejects(session.load(),/checksum/i);assert.equal(calls.commit.length,0);assert.equal(session.pending,false);assert.equal(calls.status.at(-1).state,'error');
});
test('canceled_read_does_not_prepare_and_failed_write_does_not_block_later_save',async()=>{
 const read=deferred();let preparations=0;const {session,calls}=setup({runtime:{prepare:async()=>{preparations++;return 10;}},storage:{read:()=>read.promise}});
 const load=session.load();session.cancel();read.resolve(await record({}));await load;assert.equal(preparations,0);assert.equal(calls.commit.length,0);
 let writes=0;const retry=setup({storage:{write:async()=>{if(++writes===1)throw new Error('Full');}}});await assert.rejects(retry.session.save(),/Full/);await retry.session.save();assert.equal(writes,2);assert.equal(retry.session.pending,false);
});
test('cancel_after_commit_preserves_committed_candidate',async()=>{
 const committed=deferred();const {session,calls}=setup({runtime:{commit:async(token,onDispatch)=>{onDispatch();calls.commit.push(token);await committed.promise;}}});const operation=session.newGame();await tick();session.cancel();assert.equal(session.pending,true);committed.resolve();await operation;assert.deepEqual(calls.commit,[8]);assert.deepEqual(calls.discard,[]);assert.equal(calls.status.at(-1).state,'success');
});
