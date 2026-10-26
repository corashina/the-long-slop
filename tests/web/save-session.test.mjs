import test from 'node:test';
import assert from 'node:assert/strict';
import {webcrypto as crypto} from 'node:crypto';
import {createSaveSession} from '../../web/save-session.js';
const deferred=()=>{let resolve;const promise=new Promise(r=>{resolve=r;});return {promise,resolve};};
const tick=()=>new Promise(resolve=>setImmediate(resolve));
async function record(payload){const hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(JSON.stringify(payload)));return {version:1,payload,checksum:Array.from(new Uint8Array(hash),b=>b.toString(16).padStart(2,'0')).join('')};}
function setup(overrides={}){
 const calls={commit:[],discard:[],write:[],status:[]};const runtime={capture:async()=>({save:{schema_version:4},summary:{seed:1,driven_km:0,build_id:1}}),prepare:async()=>7,commit:async(token,onDispatch)=>{onDispatch();calls.commit.push(token);},discard:token=>calls.discard.push(token),prepareNewGame:async()=>8,...overrides.runtime};
 const storage={list:async()=>({saves:[],autosave:null,last:'a'}),read:async()=>await record({schema_version:4}),write:async(name,save,entry)=>{calls.write.push({name,save,entry});},...overrides.storage};
 const session=createSaveSession({runtime,storage,drawDistance:()=>2500,onStatus:status=>calls.status.push(status)});return {calls,session};
}
test('canceled_preparation_discards_stale_candidate_without_commit',async()=>{
 const candidate=deferred();const {session,calls}=setup({runtime:{prepareNewGame:()=>candidate.promise}});const operation=session.newGame();session.cancel();assert.equal(session.pending,false);candidate.resolve(11);await operation;assert.deepEqual(calls.discard,[11]);assert.equal(calls.commit.length,0);
});
test('stale_capture_does_not_write_or_clear_later_pending_operation',async()=>{
 const capture=deferred(),candidate=deferred();const {session,calls}=setup({runtime:{capture:()=>capture.promise,prepareNewGame:()=>candidate.promise}});const first=session.save();session.cancel();const second=session.newGame();capture.resolve({save:{}});await first;assert.equal(session.pending,true);assert.equal(calls.write.length,0);candidate.resolve(9);await second;assert.deepEqual(calls.commit,[9]);
});
test('delayed_canceled_write_cannot_overwrite_a_later_save',async()=>{
 const delayed=deferred();let stored;let count=0;let signal;const {session}=setup({runtime:{capture:async()=>({save:{value:++count},summary:{seed:1,driven_km:0,build_id:1}})},storage:{write:async(name,save,entry,options)=>{signal=options.signal;if(save.value===1)await delayed.promise;stored=save;}}});
 const first=session.save();await tick();session.cancel();assert.equal(signal.aborted,true);const second=session.save();await tick();assert.equal(stored,undefined);delayed.resolve();await Promise.all([first,second]);assert.deepEqual(stored,{value:2});assert.equal(session.pending,false);
});
test('failed_load_keeps_runtime_and_reports_error',async()=>{
 const {session,calls}=setup({storage:{read:async()=>({version:1,payload:{},checksum:'bad'})}});await assert.rejects(session.load(),/checksum/i);assert.equal(calls.commit.length,0);assert.equal(session.pending,false);assert.equal(calls.status.at(-1).state,'error');
});
test('cancel_after_commit_preserves_committed_candidate',async()=>{
 const committed=deferred();const {session,calls}=setup({runtime:{commit:async(token,onDispatch)=>{onDispatch();calls.commit.push(token);await committed.promise;}}});const operation=session.newGame();await tick();session.cancel();assert.equal(session.pending,true);committed.resolve();await operation;assert.deepEqual(calls.commit,[8]);assert.deepEqual(calls.discard,[]);assert.equal(calls.status.at(-1).state,'success');
});

test('menu_cancel_ignores_late_prepare_and_protects_dispatched_commit',async()=>{
 const {MenuState}=await import('../../web/menu.js');
 const candidate=deferred();const {session,calls}=setup({runtime:{prepareNewGame:()=>candidate.promise}});
 const menu=new MenuState();menu.ready();const operation=menu.run('newGame',session);
 menu.pause(session);assert.equal(menu.pending,false);candidate.resolve(19);await operation;
 assert.equal(menu.started,false);assert.equal(menu.status,'');assert.deepEqual(calls.discard,[19]);
 const committed=deferred();const second=setup({runtime:{commit:async(token,onDispatch)=>{onDispatch();await committed.promise;}}});
 const next=menu.run('newGame',second.session);await tick();menu.pause(second.session);
 assert.equal(menu.pending,true);committed.resolve();await next;assert.equal(menu.started,true);
});
