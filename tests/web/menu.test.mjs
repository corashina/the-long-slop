import test from 'node:test';
import assert from 'node:assert/strict';
import {webcrypto as crypto} from 'node:crypto';
import {MenuState,ACTIONS,createMenu} from '../../web/menu.js';
import {createSaveSession} from '../../web/save-session.js';

const deferred=()=>{let resolve;const promise=new Promise(r=>{resolve=r;});return {promise,resolve};};
async function savedRecord(payload={schema_version:1}){const json=JSON.stringify(payload),hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(json));return {version:1,payload,checksum:Array.from(new Uint8Array(hash),b=>b.toString(16).padStart(2,'0')).join('')};}
function setup({record=null,readError,writeError,prepareError,delay,captureDelay,readDelay,started=true}={}){
 const nodes=new Map(),calls={capture:0,commit:0,pointerLock:0,actions:[],write:0};
 function node(selector){if(!nodes.has(selector))nodes.set(selector,{style:{},hidden:false,disabled:false,textContent:'',classList:{toggle(){}},listeners:{},addEventListener(name,handler){this.listeners[name]=handler;},querySelectorAll(){return [];},focus(){},async click(){if(!this.disabled)return this.listeners.click?.();}});return nodes.get(selector);}
 const canvas=node('#game');canvas.requestPointerLock=async()=>{calls.pointerLock++;document.pointerLockElement=canvas;};
 globalThis.document={querySelector:node,addEventListener(){},pointerLockElement:null,fullscreenEnabled:false,exitPointerLock(){this.pointerLockElement=null;}};
 globalThis.window={addEventListener(){},removeEventListener(){}};globalThis.innerWidth=1024;globalThis.innerHeight=768;
 const layout={canvas:{resolution:[1024,768],mode:0},ancestry:[],font:'Arial',font_size:18,color:[1,1,1,1],sprite:null};
 const ui={layouts:new Proxy({},{get:()=>layout}),urls:[],layoutUrls:{},pairUrls:[{background:'art',frame:'frame'}]};
 const storage={async read(){if(readDelay)await readDelay.promise;if(readError)throw Error(readError);return record;},async write(payload){calls.write++;if(writeError)throw Error(writeError);record=await savedRecord(payload);}};
 const runtime={async capture(){calls.capture++;if(captureDelay)await captureDelay.promise;return {schema_version:1};},async prepare(){if(prepareError)throw Error(prepareError);return 7;},async prepareNewGame(){if(delay)await delay.promise;return 8;},async commit(token,onDispatch){onDispatch();calls.commit++;},discard(){}};
 const saveSession=createSaveSession({storage,runtime});
 const menu=createMenu({canvas,ui,saveSession,clearInput(){},queueAction:a=>calls.actions.push(a)});menu.ready();if(started){menu.model.started=true;menu.model.pause();menu.sync(true,true,0);}
 return {menu,storage,calls,node};
}

test('controls_back_restores_origin',()=>{
 const m=new MenuState();m.ready();m.controls();m.back();assert.equal(m.state,'start-menu');
 m.started=true;m.pause();m.takeActions();m.controls();m.back();assert.equal(m.state,'pause-menu');assert.equal(m.actions,0);
});
test('capture_failure_keeps_retry',async()=>{
 const m=new MenuState();m.ready();m.started=true;m.pause();m.takeActions();
 await m.capture(ACTIONS.CONTINUE,async()=>{throw Error('blocked');});
 assert.equal(m.state,'pause-menu');assert.equal(m.actions,0);assert.match(m.error,/blocked/);
 await m.capture(ACTIONS.CONTINUE,async()=>{});assert.equal(m.takeActions(),4);assert.equal(m.state,'playing');
});
test('late_capture_after_blur_stays_paused',async()=>{
 const m=new MenuState();m.ready();m.started=true;m.pause();
 let resolve;const pending=m.capture(ACTIONS.CONTINUE,()=>new Promise(r=>{resolve=r;}));
 m.pause();resolve();await pending;assert.equal(m.state,'pause-menu');assert.equal(m.takeActions()&4,0);
});
test('missing_sprite_disables_new_game',()=>{
 const m=new MenuState();m.fail('Missing sprite enter');m.ready();assert.equal(m.state,'error');assert.equal(m.readyFlag,false);
});
test('recover_does_not_resume',()=>{
 const m=new MenuState();m.ready();m.started=true;m.pause();m.recover();assert.equal(m.state,'pause-menu');assert.equal(m.takeActions(),16);
});
test('load_failure_preserves_continue',async()=>{
 const {menu,node,calls}=setup({record:await savedRecord(),prepareError:'Missing source asset: reference/garage.glb'});
 await node('#load').click();assert.match(menu.model.error,/reference\/garage.glb/);assert.equal(menu.model.state,'pause-menu');assert.equal(menu.model.started,true);assert.equal(calls.commit,0);assert.equal(node('#resume').disabled,false);assert.equal(calls.pointerLock,0);
 await node('#resume').click();assert.equal(calls.pointerLock,1);assert.equal(calls.actions.at(-1),ACTIONS.CONTINUE);
});
test('pending_disables_conflicting_actions',async()=>{
 const delay=deferred(),{menu,node,calls}=setup({delay});const operation=node('#pause-new-game').click();
 assert.equal(menu.model.pending,true);assert.equal(node('#status').textContent,'Starting a new game...');
 for(const id of ['#resume','#save','#load','#pause-new-game','#recover','#controls'])assert.equal(node(id).disabled,true,id);
 await node('#load').click();await node('#recover').click();await node('#resume').click();assert.equal(calls.pointerLock,0);assert.equal(calls.commit,0);
 menu.model.pause();assert.equal(menu.model.pending,true);delay.resolve();await operation;assert.equal(menu.model.state,'pause-menu');assert.equal(node('#resume').disabled,false);
});
test('storage_failure_keeps_new_game_and_recover',async()=>{
 const {menu,node,calls}=setup({writeError:'Save storage is unavailable.'});await node('#save').click();
 assert.equal(menu.model.error,'Save storage is unavailable.');assert.equal(menu.model.readyFlag,true);
 for(const id of ['#pause-new-game','#recover','#resume'])assert.equal(node(id).disabled,false,id);
 await node('#recover').click();assert.equal(calls.actions.at(-1),ACTIONS.RECOVER);await node('#pause-new-game').click();assert.equal(calls.commit,1);assert.equal(menu.model.state,'pause-menu');
});
test('new_game_keeps_saved_record',async()=>{
 const recordBeforeNewGame=await savedRecord(),{menu,node,storage,calls}=setup({record:recordBeforeNewGame,started:false});
 await node('#new-game').click();assert.equal(menu.model.state,'pause-menu');assert.equal(menu.model.started,true);assert.equal(calls.pointerLock,0);assert.equal(calls.write,0);assert.deepEqual(await storage.read(),recordBeforeNewGame);
});
test('load_reports_exact_missing_and_incompatible_text',async()=>{
 let fixture=setup({started:false});await fixture.node('#start-load').click();assert.equal(fixture.menu.model.error,'No saved game is available.');assert.equal(fixture.menu.model.started,false);
 fixture=setup({record:await savedRecord(),prepareError:'Incompatible save: Schema version'});await fixture.node('#load').click();assert.equal(fixture.menu.model.error,'This save uses a different schema or source export.');assert.equal(fixture.calls.pointerLock,0);
});
test('load_and_save_success_show_status_and_keep_pause',async()=>{
 const {menu,node,calls}=setup({record:await savedRecord(),started:false});await node('#start-load').click();
 assert.equal(menu.model.state,'pause-menu');assert.equal(menu.model.started,true);assert.equal(node('#status').textContent,'Game loaded.');assert.equal(calls.pointerLock,0);
 await node('#save').click();assert.equal(node('#status').textContent,'Save complete.');assert.equal(menu.model.state,'pause-menu');assert.equal(calls.pointerLock,0);
});

test('save_and_load_show_pending_status_and_block_actions',async()=>{
 for(const [selector,field,text] of [['#save','captureDelay','Saving...'],['#load','readDelay','Loading...']]){
  const delay=deferred(),{menu,node,calls}=setup({record:await savedRecord(),[field]:delay});const operation=node(selector).click();
  assert.equal(menu.model.pending,true);assert.equal(node('#status').textContent,text);
  for(const id of ['#resume','#save','#load','#pause-new-game','#recover','#controls'])assert.equal(node(id).disabled,true,id);
  await node('#resume').click();assert.equal(calls.pointerLock,0);delay.resolve();await operation;assert.equal(menu.model.pending,false);assert.equal(menu.model.state,'pause-menu');
 }
});

test('audio_unlock_throw_keeps_capture_working',async()=>{const m=new MenuState();m.ready();m.started=true;m.pause();m.takeActions();await m.capture(ACTIONS.CONTINUE,async()=>{},()=>{throw Error('audio');});assert.equal(m.state,'playing');assert.equal(m.takeActions(),ACTIONS.CONTINUE);});
