import {createInput} from './input.js';
import {createMenu} from './menu.js';
import {loadUiAssets} from './ui-layout.js';
const canvas=document.querySelector('#game'),status=document.querySelector('#status'),runtimeError=document.querySelector('#runtime-error');
const hud=document.querySelector('#hud'),crosshair=document.querySelector('#crosshair');
const prompt=document.querySelector('#action-prompt'),target=document.querySelector('#target'),detail=document.querySelector('#target-detail');
const heldLabel=document.querySelector('#held'),engineLabel=document.querySelector('#engine'),speed=document.querySelector('#speed'),cameraMode=document.querySelector('#camera-mode'),fuel=document.querySelector('#fuel'),fuelLevel=document.querySelector('#fuel-level'),trip=document.querySelector('#trip');
let menu,failed=false,timeout,actions=0,frame;
let lastPaused,lastFuelNumber,lastTripNumber,lastSpeedNumber,lastAmount,lastCondition,lastHeld,lastRpm,lastIgnition,lastRunning,lastStarter,lastHandbrake;
const lastVolumes=[-2,-2,-2,-2,-2,-2],fluidNames=['gasoline','oil','alcohol','water','blood','diesel'],cached=new Map();
function write(element,value){if(cached.get(element)!==value){element.textContent=value;cached.set(element,value);}}
const input=createInput(canvas,()=>menu?.model.state==='playing' && document.pointerLockElement===canvas);
window.gameKeys=()=>{frame=input.takeFrame();return frame.held;};
window.gamePressed=()=>frame.pressed;window.gameReleased=()=>frame.released;
window.gameMouseX=()=>frame.mouseX;window.gameMouseY=()=>frame.mouseY;window.gameWheel=()=>frame.wheel;
window.gameActions=()=>{const a=actions;actions=0;return a;};
window.gameState = { speedKmh: 0, camera: 'First person', x: 0, y: 0, z: 0, contacts: 0,
  frameSteps: 0, totalSteps: 0, activeChunks: 0, colliderCount: 0, absoluteX: 0, absoluteY: 0, absoluteZ: 0,
  originX: 0, originY: 0, originZ: 0, rebases: 0, travelM: 0, lateralBand: 0, fuelLiters: 0,
  paused: false, started: false, target: '', targetAction: '', heldItem: '', rpm: 0, ignition: false,
  running: false, starter: false, seated: false, handbrake: true, fluidVolumes:[0,0,0,0,0,0] };
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
  const roundedSpeed=Math.round(Math.abs(kmh));
  if (roundedSpeed!==lastSpeedNumber) {write(speed,String(roundedSpeed));lastSpeedNumber=roundedSpeed;}
  write(cameraMode, state.camera);
  const liters = Math.round(fuelLiters * 10) / 10;
  if (liters!==lastFuelNumber) {write(fuel,liters.toFixed(1));fuelLevel.value=liters;lastFuelNumber=liters;}
  const distance=Math.round(travelM / 10) / 100;
  if (distance!==lastTripNumber) {write(trip,distance.toFixed(2));lastTripNumber=distance;}
  menu?.sync(started,paused,travelM);
  crosshair.hidden = paused || !started || !mode;
};
window.gameDetail = (name, action, amount, condition, held, rpm, ignition, running, starter, seated, handbrake, gasoline,oil,alcohol,water,blood,diesel) => {
  const state = window.gameState;
  state.target = name; state.targetAction = action; state.heldItem = held;
  state.rpm = rpm; state.ignition = ignition; state.running = running;
  state.starter = starter; state.seated = seated; state.handbrake = handbrake;
  const volumes=state.fluidVolumes;
  volumes[0]=gasoline;volumes[1]=oil;volumes[2]=alcohol;volumes[3]=water;volumes[4]=blood;volumes[5]=diesel;
  write(target, name);
  write(prompt, action);
  const roundedAmount=Math.round(amount*10)/10;
  const roundedCondition=Math.round(condition*100);
  let changed=roundedAmount!==lastAmount || roundedCondition!==lastCondition;
  for (let i=0;i<6;i++) {const rounded=Math.round(volumes[i]*10)/10;if (rounded!==lastVolumes[i]) {lastVolumes[i]=rounded;changed=true;}}
  if (changed) {
    if (name==='Gasoline pump') write(detail,'Unlimited gasoline');
    else if (amount>=0) {
      const parts=[];
      for (let i=0;i<6;i++) {if (volumes[i]>0.0001) parts.push(`${lastVolumes[i].toFixed(1)} L ${fluidNames[i]}`);}
      write(detail,parts.length ? parts.join(' Â· ') : 'Empty');
    } else write(detail,condition>=0 ? `Condition ${roundedCondition}%` : '');
    lastAmount=roundedAmount;lastCondition=roundedCondition;
  }
  if (held!==lastHeld) {write(heldLabel,held ? `Carrying ${held}` : '');lastHeld=held;}
  const roundedRpm=Math.round(rpm);
  if (roundedRpm!==lastRpm || ignition!==lastIgnition || running!==lastRunning || starter!==lastStarter || handbrake!==lastHandbrake) {
    write(engineLabel,`${starter ? 'Starter' : running ? 'Engine running' : ignition ? 'Ignition on' : 'Ignition off'} Â· ${roundedRpm} rpm${handbrake ? ' Â· Handbrake on' : ''}`);
    lastRpm=roundedRpm;lastIgnition=ignition;lastRunning=running;lastStarter=starter;lastHandbrake=handbrake;
  }
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
 status.textContent='Loading interface assets…';
 await loadUiAssets(manifest.ui);
 menu=createMenu({canvas,ui:manifest.ui,clearInput:()=>input.clear(),queueAction:a=>{actions=a;}});
 const tank=manifest.tanks.find(t=>t.item===manifest.engine.fuel_tank);fuelLevel.max=tank.capacity_liters;fuelLevel.optimum=tank.capacity_liters;
 status.textContent='Loading game code…';
 const {default:init,run}=await import('./pkg/my_summer_slop.js');
 const wasm=await fetch('./pkg/my_summer_slop_bg.wasm');if(!wasm.ok)throw Error('Cannot load game code (HTTP '+wasm.status+').');
 await init({module_or_path:wasm});status.textContent='Loading the garage…';
 timeout=setTimeout(()=>fail('The garage did not load within 120 seconds.'),120000);run(text);
}
load().catch(error=>fail(error.message));
