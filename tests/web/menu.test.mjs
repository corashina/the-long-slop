import test from 'node:test';
import assert from 'node:assert/strict';
import {webcrypto as crypto} from 'node:crypto';
import {readFileSync} from 'node:fs';
const drivingBindings=JSON.parse(readFileSync(new URL('../common/reference.json',import.meta.url))).driving.bindings;
import {MenuState,ACTIONS,createMenu} from '../../web/menu.js';
import {createSaveSession} from '../../web/save-session.js';

const deferred=()=>{let resolve;const promise=new Promise(r=>{resolve=r;});return {promise,resolve};};
async function savedRecord(payload={schema_version:4}){const json=JSON.stringify(payload),hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(json));return {version:1,payload,checksum:Array.from(new Uint8Array(hash),b=>b.toString(16).padStart(2,'0')).join('')};}
function setup({record=null,readError,writeError,prepareError,delay,captureDelay,readDelay,started=true}={}){
 const nodes=new Map(),calls={capture:0,commit:0,pointerLock:0,actions:[],write:0};
 function node(selector){if(!nodes.has(selector))nodes.set(selector,{style:{},hidden:false,disabled:false,textContent:'',classList:{toggle(){}},listeners:{},addEventListener(name,handler){this.listeners[name]=handler;},querySelectorAll(){return [];},focus(){},async click(){if(!this.disabled)return this.listeners.click?.();}});return nodes.get(selector);}
 const canvas=node('#game');canvas.requestPointerLock=async()=>{calls.pointerLock++;document.pointerLockElement=canvas;};
 globalThis.document={querySelector:node,addEventListener(){},pointerLockElement:null,fullscreenEnabled:false,exitPointerLock(){this.pointerLockElement=null;}};
 globalThis.window={addEventListener(){},removeEventListener(){}};globalThis.innerWidth=1024;globalThis.innerHeight=768;
 const layout={canvas:{resolution:[1024,768],mode:0},ancestry:[],font:'Arial',font_size:18,color:[1,1,1,1],sprite:null};
 const ui={layouts:new Proxy({},{get:()=>layout}),urls:[],layoutUrls:{},pairUrls:[{background:'art',frame:'frame'}]};
 const storage={async read(){if(readDelay)await readDelay.promise;if(readError)throw Error(readError);return record;},async write(payload){calls.write++;if(writeError)throw Error(writeError);record=await savedRecord(payload);}};
 const runtime={async capture(){calls.capture++;if(captureDelay)await captureDelay.promise;return {schema_version:4};},async prepare(){if(prepareError)throw Error(prepareError);return 7;},async prepareNewGame(){if(delay)await delay.promise;return 8;},async commit(token,onDispatch){onDispatch();calls.commit++;},discard(){}};
 const saveSession=createSaveSession({storage,runtime});
 const menu=createMenu({canvas,ui,drivingBindings,saveSession,clearInput(){},queueAction:a=>calls.actions.push(a)});menu.ready();if(started){menu.model.started=true;menu.model.pause();menu.sync(true,true,0);}
 return {menu,storage,calls,node};
}

test('scene_failure_rejects_late_save_completion_and_readiness',async()=>{
 const m=new MenuState(),commit=deferred();m.ready();
 const pending=m.run('newGame',{newGame:()=>commit.promise});
 m.loading();m.fail('GPU failed');commit.resolve();await pending;m.ready();
 assert.equal(m.state,'error');assert.equal(m.error,'GPU failed');assert.equal(m.readyFlag,false);
 assert.equal(m.pending,false);assert.equal(m.operation,'');assert.equal(m.started,false);
});

test('world_dependency_failure_keeps_start_disabled',()=>{
 const model=new MenuState();
 model.fail('Missing required source world dependency');
 model.ready();
 assert.equal(model.readyFlag,false);
 assert.equal(model.state,'error');
});
