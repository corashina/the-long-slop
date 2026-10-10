import {createInput} from './input.js';
import {createMenu} from './menu.js';
import {createFeedback} from './feedback.js';
import {loadUiAssets} from './ui-layout.js';
const canvas=document.querySelector('#game'),status=document.querySelector('#status'),runtimeError=document.querySelector('#runtime-error');
let menu,feedback,failed=false,timeout,actions=0,frame;
const record={actions:0,context:0,name:'',notice:'',carrying:false,amount:0,capacity:0,hasName:false,noText:true,quantities:[0,0,0,0,0,0]};
const input=createInput(canvas,()=>menu?.model.state==='playing' && document.pointerLockElement===canvas);
window.gameKeys=()=>{frame=input.takeFrame();return frame.held;};
window.gamePressed=()=>frame.pressed;window.gameReleased=()=>frame.released;
window.gameMouseX=()=>frame.mouseX;window.gameMouseY=()=>frame.mouseY;window.gameWheel=()=>frame.wheel;
window.gameActions=()=>{const a=actions;actions=0;return a;};
window.gameState = { speedKmh: 0, camera: 'First person', x: 0, y: 0, z: 0, contacts: 0,
  frameSteps: 0, totalSteps: 0, activeChunks: 0, colliderCount: 0, absoluteX: 0, absoluteY: 0, absoluteZ: 0,
  originX: 0, originY: 0, originZ: 0, rebases: 0, travelM: 0, lateralBand: 0, fuelLiters: 0,
  paused: false, started: false, target: '', targetAction: 0, carrying:false, fluidVolumes:record.quantities };
window.gameUpdate = (kmh, mode, x, y, z, contacts, steps, chunks, colliders, absoluteX, absoluteY, absoluteZ,
  originX, originY, originZ, rebases, travelM, lateralBand, fuelLiters, paused, started) => {
  const state = window.gameState;
  state.speedKmh = kmh; state.camera = mode ? 'First person' : 'Chase';
  state.x = x; state.y = y; state.z = z; state.contacts = contacts;
  state.frameSteps = steps; state.totalSteps += steps; state.activeChunks = chunks; state.colliderCount = colliders;
  state.absoluteX = absoluteX; state.absoluteY = absoluteY; state.absoluteZ = absoluteZ;
  state.originX = originX; state.originY = originY; state.originZ = originZ;
  state.rebases = rebases; state.travelM = travelM; state.lateralBand = lateralBand;
  state.fuelLiters = fuelLiters; state.paused = paused; state.started = started;
  menu?.sync(started,paused,travelM);
  feedback?.setVisible(started&&!paused&&Boolean(mode)&&menu?.model.state==='playing');
};
window.gameFeedback=(mask,context,name,notice,carrying,amount,capacity,hasName,noText,gas,oil,alcohol,water,blood,diesel)=>{
 record.actions=mask;record.context=context;record.name=name;record.notice=notice;record.carrying=carrying;record.amount=amount;record.capacity=capacity;record.hasName=hasName;record.noText=noText;
 const q=record.quantities;q[0]=gas;q[1]=oil;q[2]=alcohol;q[3]=water;q[4]=blood;q[5]=diesel;
 window.gameState.target=name;window.gameState.targetAction=mask;window.gameState.carrying=carrying;feedback?.update(record);
};
function fail(message){failed=true;clearTimeout(timeout);const text='Cannot start the game. '+message+' Reload to try again.';if(menu){menu.fail(text);status.textContent='';}else{status.textContent='';runtimeError.textContent=text;runtimeError.hidden=false;}}
window.gameFailure=fail;
window.gameReady=()=>{if(failed)return;clearTimeout(timeout);menu.ready();};
window.addEventListener('error',event=>fail(event.message||'The runtime stopped.'));
window.addEventListener('unhandledrejection',event=>fail(event.reason?.message||String(event.reason)));
canvas.addEventListener('webglcontextlost',event=>{event.preventDefault();fail('The graphics context was lost.');});
async function load(){
 const probe=document.createElement('canvas'),graphics=probe.getContext('webgl2');
 if(!graphics)throw Error('This browser cannot use WebGL2.');
 graphics.getExtension('WEBGL_lose_context')?.loseContext();
 const response=await fetch('./assets/reference/manifest.json');
 if(!response.ok)throw Error('Cannot load reference manifest (HTTP '+response.status+').');
 const text=await response.text(),manifest=JSON.parse(text);
 status.textContent='Loading interface assets...';
 await loadUiAssets(manifest.ui);
 menu=createMenu({canvas,ui:manifest.ui,clearInput:()=>input.clear(),queueAction:a=>{actions=a;}});
 feedback=createFeedback(manifest.ui);
 status.textContent='Loading game code...';
 const {default:init,run}=await import('./pkg/my_summer_slop.js');
 const wasm=await fetch('./pkg/my_summer_slop_bg.wasm');if(!wasm.ok)throw Error('Cannot load game code (HTTP '+wasm.status+').');
 await init({module_or_path:wasm});status.textContent='Loading the garage...';
 timeout=setTimeout(()=>fail('The garage did not load within 120 seconds.'),120000);run(text);
}
load().catch(error=>fail(error.message));
