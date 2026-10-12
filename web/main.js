import {createAudio} from './audio.js';
import {createSurvival} from './survival.js';
import {createInput} from './input.js';
import {createMenu} from './menu.js';
import {createFeedback} from './feedback.js';
import {loadUiAssets} from './ui-layout.js';
const canvas=document.querySelector('#game'),status=document.querySelector('#status'),runtimeError=document.querySelector('#runtime-error');
let menu,feedback,audio,survival,dead=false,sleeping=false,failed=false,timeout,actions=0,frame;
const record={actions:0,context:0,name:'',notice:'',carrying:false,amount:0,capacity:0,hasName:false,noText:true,fluidName:'',condition:-1,held:'',throwCharge:0,throwActive:false,quantities:[0,0,0,0,0,0]};
const input=createInput(canvas,()=>menu?.model.state==='playing' && document.pointerLockElement===canvas);
window.gameKeys=()=>{frame=input.takeFrame();return frame.held;};
window.gamePressed=()=>frame.pressed;window.gameReleased=()=>frame.released;
window.gameMouseX=()=>frame.mouseX;window.gameMouseY=()=>frame.mouseY;window.gameWheel=()=>frame.wheel;
window.gameActions=()=>{const a=actions;actions=0;return a;};
window.gameState = { speedKmh: 0, camera: 'First person', x: 0, y: 0, z: 0, contacts: 0,
  frameSteps: 0, totalSteps: 0, activeChunks: 0, colliderCount: 0, absoluteX: 0, absoluteY: 0, absoluteZ: 0,
  originX: 0, originY: 0, originZ: 0, rebases: 0, travelM: 0, lateralBand: 0, fuelLiters: 0,
  paused: false, started: false, target: '', targetAction: 0, carrying:false, heldItem:'', rpm:0, ignition:false, running:false, starter:false, seated:false, handbrake:true, throwCharge:0, throwActive:false, fluidVolumes:record.quantities };
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
  feedback?.setVisible(started&&!paused&&!dead&&!sleeping&&Boolean(mode)&&menu?.model.state==='playing');
  survival?.setVisible(started&&!paused&&!sleeping&&menu?.model.state==='playing');
  audio?.setPlaying(started&&!paused&&menu?.model.state==='playing');

};
window.gameFeedback=(mask,context,name,notice,carrying,amount,capacity,hasName,noText,fluidName,condition,held,charge,throwing,rpm,ignition,running,starter,seated,handbrake,gas,oil,alcohol,water,blood,diesel,control,lmbUse)=>{
 record.actions=mask;record.context=context;record.lmbUse=lmbUse;record.name=name;record.notice=notice;record.carrying=carrying;record.amount=amount;record.capacity=capacity;record.hasName=hasName;record.noText=noText;record.fluidName=fluidName;
 record.condition=condition;record.held=held;record.throwCharge=charge;record.throwActive=throwing;
 const q=record.quantities;q[0]=gas;q[1]=oil;q[2]=alcohol;q[3]=water;q[4]=blood;q[5]=diesel;
 const state=window.gameState;
 state.feedbackContext=context;state.availableActions=mask;state.lightControl=control;state.lmbUse=lmbUse;
 state.target=name;state.targetAction=mask;state.carrying=carrying;state.heldItem=held;state.throwCharge=charge;state.throwActive=throwing;
 state.rpm=rpm;state.ignition=ignition;state.running=running;state.starter=starter;state.seated=seated;state.handbrake=handbrake;
 feedback?.update(record);
};
const needs={hp:0,food:0,water:0,bowel:0,bladder:0,opacity:0,dead:false,lookScale:1,menuDue:false};
window.gameSurvival=(hp,food,water,bowel,bladder,opacity,isDead,lookScale,menuDue)=>{
 needs.hp=hp;needs.food=food;needs.water=water;needs.bowel=bowel;needs.bladder=bladder;needs.opacity=opacity;needs.dead=isDead;needs.lookScale=lookScale;needs.menuDue=menuDue;dead=isDead;survival?.update(needs);menu?.survival(isDead,menuDue);
};
const sleep={phase:0,eligible:false,charge:0,cooldownRemaining:0,progress:0,held:false,dead:false};
window.gameSleep=(phase,eligible,charge,cooldownRemaining,progress)=>{
 sleep.phase=phase;sleep.eligible=eligible;sleep.charge=charge;sleep.cooldownRemaining=cooldownRemaining;sleep.progress=progress;sleep.held=(input.state.held&131072)!==0;sleep.dead=dead;sleeping=phase===2;survival?.updateSleep(sleep);
};
window.gameAudioError=message=>audio?.disable(Error(message));
window.gameAudio=(ids,values)=>audio?.publish(ids,values);
window.gameAudioListener=(session,motionRevision,x,y,z,fx,fy,fz,ux,uy,uz,gain,ox,oy,oz,seconds)=>audio?.setListener(session,motionRevision,x,y,z,fx,fy,fz,ux,uy,uz,gain,ox,oy,oz,seconds);
window.addEventListener('pagehide',()=>audio?.dispose());
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
 audio=createAudio({bindings:manifest.audio,onError:error=>console.warn('Audio disabled:',error.message)});
 survival=createSurvival(manifest.ui,manifest.sleep.cooldown_seconds);
 menu=createMenu({canvas,ui:manifest.ui,audio,clearInput:()=>input.clear(),queueAction:a=>{actions=a;}});
 const switchIconUrl='./assets/'+manifest.lighting.switch_icon.path;
 const switchIcon=new Image();switchIcon.src=switchIconUrl;await switchIcon.decode();
 feedback=createFeedback(manifest.ui,switchIconUrl);
 status.textContent='Loading game code...';
 const {default:init,run}=await import('./pkg/my_summer_slop.js');
 const wasm=await fetch('./pkg/my_summer_slop_bg.wasm');if(!wasm.ok)throw Error('Cannot load game code (HTTP '+wasm.status+').');
 await init({module_or_path:wasm});status.textContent='Loading the garage...';
 timeout=setTimeout(()=>fail('The garage did not load within 120 seconds.'),120000);run(text);
}
load().catch(error=>fail(error.message));
