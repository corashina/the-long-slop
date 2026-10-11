import {createInput} from './input.js';
import {createMenu} from './menu.js';
import {createFeedback} from './feedback.js';
import {loadUiAssets} from './ui-layout.js';
import {createAudio} from './audio.js';
const canvas=document.querySelector('#game'),status=document.querySelector('#status'),runtimeError=document.querySelector('#runtime-error');
let menu,feedback,audio,failed=false,timeout,actions=0,frame;
const record={actions:0,context:0,name:'',notice:'',carrying:false,amount:0,capacity:0,hasName:false,noText:true,quantities:[0,0,0,0,0,0]};
const input=createInput(canvas,()=>menu?.model.state==='playing' && document.pointerLockElement===canvas);
window.gameKeys=()=>{frame=input.takeFrame();return frame.held;};
window.gamePressed=()=>frame.pressed;window.gameReleased=()=>frame.released;
window.gameMouseX=()=>frame.mouseX;window.gameMouseY=()=>frame.mouseY;window.gameWheel=()=>frame.wheel;
window.gameActions=()=>{const a=actions;actions=0;return a;};
window.gameUpdate = (firstPerson, travelM, paused, started) => {
  menu?.sync(started,paused,travelM);
  audio?.setPlaying(started&&!paused&&menu?.model.state==='playing');
  feedback?.setVisible(started&&!paused&&firstPerson&&menu?.model.state==='playing');
};
window.gameFeedback=(mask,context,name,notice,carrying,amount,capacity,hasName,noText,gas,oil,alcohol,water,blood,diesel)=>{
 record.actions=mask;record.context=context;record.name=name;record.notice=notice;record.carrying=carrying;record.amount=amount;record.capacity=capacity;record.hasName=hasName;record.noText=noText;
 const q=record.quantities;q[0]=gas;q[1]=oil;q[2]=alcohol;q[3]=water;q[4]=blood;q[5]=diesel;
 feedback?.update(record);
};
function fail(message){failed=true;clearTimeout(timeout);const text='Cannot start the game. '+message+' Reload to try again.';if(menu){menu.fail(text);status.textContent='';}else{status.textContent='';runtimeError.textContent=text;runtimeError.hidden=false;}}
window.gameAudio=view=>audio?.frame(view);
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
 audio=createAudio(manifest.audio);audio.bindButtons(document.querySelector('#menu'));
 for(const type of ['pointerdown','keydown'])window.addEventListener(type,()=>audio.unlock());
 feedback=createFeedback(manifest.ui);
 status.textContent='Loading game code...';
 const {default:init,run}=await import('./pkg/my_summer_slop.js');
 const wasm=await fetch('./pkg/my_summer_slop_bg.wasm');if(!wasm.ok)throw Error('Cannot load game code (HTTP '+wasm.status+').');
 await init({module_or_path:wasm});status.textContent='Loading the garage...';
 timeout=setTimeout(()=>fail('The garage did not load within 120 seconds.'),120000);run(text);
}
load().catch(error=>fail(error.message));
