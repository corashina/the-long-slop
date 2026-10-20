import test from 'node:test';
import assert from 'node:assert/strict';
import {createMenu,ACTIONS} from '../../web/menu.js';
import {createInput} from '../../web/input.js';

const settle=()=>new Promise(resolve=>setImmediate(resolve));
function event(type,properties={}){return Object.assign(new Event(type,{cancelable:true}),properties);}
function setup(t){
 const originals=new Map(['window','document','innerWidth','innerHeight'].map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]));
 const window=new EventTarget(),document=new EventTarget(),nodes=new Map(),actions=[];
 class Element extends EventTarget {
  style={};classList={toggle(){}};hidden=false;disabled=false;checked=false;
  querySelectorAll(){return [];}
  focus(){document.activeElement=this;}
 }
 document.querySelector=selector=>{if(!nodes.has(selector))nodes.set(selector,new Element());return nodes.get(selector);};
 document.pointerLockElement=null;document.hidden=false;document.fullscreenEnabled=false;
 Object.assign(globalThis,{window,document,innerWidth:1280,innerHeight:720});
 const canvas=document.querySelector('#game');let captures=0;
 canvas.requestPointerLock=async()=>{captures++;document.pointerLockElement=canvas;document.dispatchEvent(event('pointerlockchange'));};
 document.exitPointerLock=()=>{document.pointerLockElement=null;document.dispatchEvent(event('pointerlockchange'));};
 const layout={canvas:{resolution:[1280,720],mode:0},ancestry:[],font_size:16,font:'sans-serif',color:[1,1,1,1],sprite:null};
 const roles=['start','start-controls','continue','restart','recover','controls','back'];
 const ui={layouts:Object.fromEntries(roles.map(role=>[role,layout])),urls:[],layoutUrls:{},pairUrls:[{background:'background',frame:'frame'}]};
 let menu;
 const input=createInput(canvas,()=>menu.model.state==='playing'&&document.pointerLockElement===canvas,[]);
 menu=createMenu({canvas,ui,drivingBindings:[],clearInput:input.clear,queueAction:action=>actions.push(action),saveSession:{newGame:async()=>{},cancel:()=>false}});
 const key=(code,repeat=false)=>window.dispatchEvent(event('keydown',{code,repeat}));
 const start=async()=>{menu.ready();document.querySelector('#new-game').dispatchEvent(event('click'));await settle();assert.equal(menu.model.state,'pause-menu');actions.length=0;};
 t.after(()=>{input.dispose();menu.dispose();for(const [key,descriptor] of originals){if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key];}});
 return {menu,input,canvas,document,window,actions,key,start,get captures(){return captures;}};
}

test('escape_resumes_pauses_and_recaptures_without_stale_input',async t=>{
 const app=setup(t);await app.start();
 app.key('Escape');await settle();
 assert.equal(app.captures,1);
 assert.equal(app.menu.model.state,'playing');
 assert.deepEqual(app.actions,[ACTIONS.CONTINUE]);
 app.key('KeyW');assert.equal(app.input.state.held,1);
 app.key('Escape');await settle();
 assert.equal(app.menu.model.state,'pause-menu');
 assert.equal(app.document.pointerLockElement,null);
 assert.equal(app.input.state.held,0);
 assert.equal(app.actions.at(-1),ACTIONS.PAUSE);
 app.key('Escape');await settle();
 assert.equal(app.menu.model.state,'playing');
 assert.equal(app.input.state.held,0);
 app.key('KeyW');app.document.exitPointerLock();await settle();
 assert.equal(app.menu.model.state,'pause-menu');
 assert.equal(app.input.state.held,0);
 app.key('Escape');await settle();
 assert.equal(app.captures,3);
 assert.equal(app.menu.model.state,'playing');
});

test('escape_capture_failure_stays_paused_and_allows_retry',async t=>{
 const app=setup(t);await app.start();
 app.canvas.requestPointerLock=async()=>{throw Error('Capture denied');};
 app.key('Escape');await settle();
 assert.equal(app.menu.model.state,'pause-menu');
 assert.match(app.menu.model.error,/Capture denied/);
 assert.deepEqual(app.actions,[]);
 app.canvas.requestPointerLock=async()=>{app.document.pointerLockElement=app.canvas;};
 app.key('Escape');await settle();
 assert.equal(app.menu.model.state,'playing');
 assert.equal(app.menu.model.error,'');
 assert.deepEqual(app.actions,[ACTIONS.CONTINUE]);
});

test('escape_ignores_repeat_and_cancels_a_late_capture',async t=>{
 const app=setup(t);await app.start();
 app.key('Escape',true);await settle();assert.equal(app.captures,0);
 let resolve;
 app.canvas.requestPointerLock=()=>new Promise(done=>{resolve=()=>{app.document.pointerLockElement=app.canvas;done();};});
 app.key('Escape');assert.equal(app.menu.model.pending,true);
 app.key('Escape');resolve();await settle();
 assert.equal(app.menu.model.state,'pause-menu');
 assert.equal(app.document.pointerLockElement,null);
 assert.equal(app.actions.includes(ACTIONS.CONTINUE),false);
});

for(const loss of ['escape','pointer','blur','hidden']){
 test(loss+'_cancels_short_q_hold_without_release',async t=>{
  const app=setup(t);await app.start();app.key('Escape');await settle();
  app.key('KeyQ');let f=app.input.takeFrame();assert.equal(f.playerHeld,1);assert.equal(f.playerPressed,1);
  await new Promise(resolve=>setTimeout(resolve,100));
  if(loss==='escape')app.key('Escape');
  if(loss==='pointer')app.document.exitPointerLock();
  if(loss==='blur')app.window.dispatchEvent(event('blur'));
  if(loss==='hidden'){app.document.hidden=true;app.document.dispatchEvent(event('visibilitychange'));}
  await settle();assert.equal(app.menu.model.state,'pause-menu');assert.equal(app.actions.at(-1),ACTIONS.PAUSE);
  f=app.input.takeFrame();assert.equal(f.playerHeld,0);assert.equal(f.playerPressed,0);assert.equal(f.playerReleased,0);
  app.document.hidden=false;app.key('Escape');await settle();assert.equal(app.menu.model.state,'playing');
  app.window.dispatchEvent(event('keyup',{code:'KeyQ'}));
  f=app.input.takeFrame();assert.equal(f.playerHeld,0);assert.equal(f.playerPressed,0);assert.equal(f.playerReleased,0);
 });
}
