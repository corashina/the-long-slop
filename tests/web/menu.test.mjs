import test from 'node:test';
import assert from 'node:assert/strict';
import {createMenu,ACTIONS} from '../../web/menu.js';
import {createInput} from '../../web/input.js';

const settle=()=>new Promise(resolve=>setImmediate(resolve));
function event(type,properties={}){return Object.assign(new Event(type,{cancelable:true}),properties);}
function setup(t,session={}){
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
 let menu,locked=null,newGame=async()=>{};
 const input=createInput(canvas,()=>menu.model.state==='playing'&&document.pointerLockElement===canvas,[]);
 menu=createMenu({canvas,ui,drivingBindings:[],clearInput:input.clear,queueAction:action=>actions.push(action),slider:{lock(value){locked=value;}},saveSession:{newGame:()=>newGame(),cancel:()=>false,list:async()=>({saves:[],autosave:null,last:null}),...session},buildId:'b',thumbnail:()=>'thumb'});
 const key=(code,repeat=false)=>window.dispatchEvent(event('keydown',{code,repeat}));
 const start=async()=>{menu.ready();document.querySelector('#new-game').dispatchEvent(event('click'));await settle();assert.equal(menu.model.state,'pause-menu');actions.length=0;};
 t.after(()=>{input.dispose();menu.dispose();for(const [key,descriptor] of originals){if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key];}});
 return {menu,input,canvas,document,actions,key,start,get captures(){return captures;},get locked(){return locked;},set newGame(work){newGame=work;}};
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

test('capture_loss_clears_horn',async t=>{
 const app=setup(t);await app.start();app.key('Escape');await settle();
 app.key('KeyH');assert.equal(app.input.takeFrame().held,131072);
 app.document.exitPointerLock();await settle();
 assert.equal(app.menu.model.state,'pause-menu');
 assert.equal(app.input.takeFrame().held,0);
 app.key('Escape');await settle();
 const frame=app.input.takeFrame();assert.equal(frame.held,0);assert.equal(frame.pressed,0);
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

test('draw_distance_unlocks_only_before_start_and_shows_refusal',async t=>{
 const app=setup(t),click=id=>app.document.querySelector(id).dispatchEvent(event('click'));
 assert.equal(app.locked,true);
 app.menu.ready();assert.equal(app.locked,false);
 click('#start-controls');assert.equal(app.menu.model.state,'controls');assert.equal(app.locked,false);
 click('#back');
 let refuse;app.newGame=()=>new Promise((_,reject)=>{refuse=reject;});
 click('#new-game');assert.equal(app.locked,true);
 refuse(new Error('Reconstruction failed: Draw distance 25000 m does not fit. Largest: 9000 m'));await settle();
 assert.equal(app.menu.model.state,'start-menu');assert.equal(app.locked,false);
 assert.equal(app.document.querySelector('#capture-error').textContent,'Draw distance 25000 m does not fit. Largest: 9000 m.');
 app.newGame=async()=>{};click('#new-game');await settle();
 assert.equal(app.menu.model.state,'pause-menu');assert.equal(app.locked,true);
 click('#controls');assert.equal(app.menu.model.state,'controls');assert.equal(app.locked,true);
});

test('save_screen_opens_from_both_menus_and_escape_clears_selection_then_returns',async t=>{
 const calls=[],entry={name:'one',date:'d',seed:7,drivenKm:12.4,version:'b',thumbnail:null};
 const app=setup(t,{list:async()=>({saves:[entry],autosave:null,last:'one'}),save:async(...args)=>{calls.push(['save',...args]);},remove:async name=>{calls.push(['remove',name]);}});
 const click=id=>app.document.querySelector(id).dispatchEvent(event('click'));
 await app.menu.ready();await settle();
 assert.equal(app.document.querySelector('#start-load-last').hidden,false);
 click('#start-load');await settle();
 assert.equal(app.menu.model.state,'saves');assert.equal(app.document.querySelector('#save-name-row').hidden,true);
 click('#save-back');assert.equal(app.menu.model.state,'start-menu');
 await app.start();
 click('#save');await settle();
 assert.equal(app.menu.model.state,'saves');assert.equal(app.menu.model.origin,'pause-menu');
 click('#save-tile-0');assert.equal(app.document.querySelector('#save-info-driven').textContent,'0012 km');
 click('#save-save');assert.equal(app.document.querySelector('#save-save-confirm').hidden,false);
 click('#save-save-yes');await settle();await settle();
 assert.deepEqual(calls,[['save','one','thumb']]);assert.equal(app.menu.model.status,'Save complete.');
 app.key('Escape');await settle();assert.equal(app.menu.model.state,'saves');assert.equal(app.menu.model.status,'Save complete.');
 app.key('Escape');await settle();assert.equal(app.menu.model.state,'pause-menu');
});
