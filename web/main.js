import {installSceneResolution} from './scene-resolution.js';
installSceneResolution(window,document.querySelector('#scene-resolution'));
import {createSurvival} from './survival.js';
import {createAudio,installAudioBridge} from './audio.js';
import {manifestStamp,createSaveStorage} from './save-storage.js';
import {createSaveSession} from './save-session.js';
import {createSaveBridge} from './save-bridge.js';
export const saveRuntime=createSaveBridge();
const saveSession=createSaveSession({storage:createSaveStorage(),runtime:saveRuntime});
import {createInput} from './input.js';
import {createCpuProfile,observeRenderMainThread} from './cpu-profile.js';
export const cpuProfile=createCpuProfile(()=>performance.now(),performance.timeOrigin);
const startupCpuProfile=new URL(location.href).searchParams.get('profileCpu')==='1';
const startup=createStartupTrace(()=>performance.now(),{hidden:document.hidden,focused:document.hasFocus(),profileCpu:startupCpuProfile});
const startupVisibilityChanged=()=>startup.environment(document.hidden,document.hasFocus());
const startupBlurred=()=>startup.environment(document.hidden,false);
document.addEventListener('visibilitychange',startupVisibilityChanged);
window.addEventListener('blur',startupBlurred);
function stopStartupEnvironment(){startupVisibilityChanged();document.removeEventListener('visibilitychange',startupVisibilityChanged);window.removeEventListener('blur',startupBlurred);}
cpuProfile.enable(startupCpuProfile);
let rustCpuProfile,profileStep=0,startupRenderTrace=startupCpuProfile?observeRenderMainThread():null;
window.gameCpuProfile=enabled=>{cpuProfile.enable(enabled);profileStep=0;rustCpuProfile?.(enabled);if(enabled){terrainClient.profileHandshake();roadClients.profileHandshake();grassClients.profileHandshake();}};
window.gameCpuProfileSnapshot=()=>cpuProfile.snapshot();
window.gameCpuInstrumentationState=()=>({browserEnabled:cpuProfile.enabled,terrain:terrainClient.profileState(),road:roadClients.profileState(),grass:grassClients.profileState(),startupRenderTraceActive:startupRenderTrace!==null});
window.gameCpuStartupRenderTrace=()=>{const trace=startupRenderTrace?.stop()??null;startupRenderTrace=null;return trace;};
window.gameCpuInputConsumed=()=>input?.consume(++profileStep);
window.gameCpuClearInput=()=>cpuProfile.clearInput();
window.gameCpuSubmitted=token=>cpuProfile.submitFrame(token);
import {createMenu} from './menu.js';
import {createFeedback} from './feedback.js';
import {createLoading} from './loading.js';
import {createStartupTrace} from './startup-trace.js';
import {createPerformance} from './performance.js';
import {loadUiAssets} from './ui-layout.js';
import {createTerrain,installTerrainBridge} from './terrain.js';
import {installRoadBridge} from './road.js';
import {installGrassBridge} from './grass.js';
import {configureAssetCatalog,readAsset,readPackageAsset,assetReceipt,checkImageSize,worldBootstrapMeasurements,worldInputBytes,worldInputMetadataBytes,setAssetCpuProfile} from './world-assets.js';
setAssetCpuProfile(cpuProfile);
// Start the terrain worker and its WASM download early.
const terrainWorkerUrl=new URL('./terrain-worker.js',import.meta.url);
if(startupCpuProfile)terrainWorkerUrl.searchParams.set('profileCpu','1');
const terrainClient=createTerrain(new Worker(terrainWorkerUrl,{type:'module'}),cpuProfile);
installTerrainBridge(window,terrainClient);
const roadClients=installRoadBridge(window,cpuProfile);
const grassClients=installGrassBridge(window,cpuProfile);
window.sourceReadAsset=(base,receipt)=>readAsset(base,JSON.parse(receipt));
window.sourceInputBytes=worldInputBytes;
window.sourceInputMetadataBytes=worldInputMetadataBytes;
const canvas=document.querySelector('#game'),status=document.querySelector('#status'),runtimeError=document.querySelector('#runtime-error');
const loading=createLoading();
performance.setResourceTimingBufferSize?.(16000);
let startupFailureDetails=null,startupDone=false,startupReady=false;
function fetched(record){
 const entry=performance.getEntriesByName?.(new URL(record.path,location.href).href)?.at(-1);
 startup.fetched({...record,cache:entry?entry.transferSize===0?'hit':'miss':'unknown',downloadMs:entry?entry.responseEnd-entry.requestStart:null});
}
window.gameStartupTrace=()=>startup.snapshot();
window.gameStartupDone=()=>startupDone;
window.gameStartupFailureDetails=json=>{startupFailureDetails=JSON.parse(json);};
window.gameStartupStage=(stage,started)=>{if(started){startup.begin(stage);cpuProfile.begin(stage);}else{startup.end(stage);cpuProfile.end(stage);}};
window.gameLoadingProgress=(roads,total,rendering,world)=>loading.progress(roads,total,rendering,world);
window.gameLoadingBudget=(name,used,limit,unit)=>loading.budget(name,used,limit,unit);
const bootStarted=performance.now();
let wasmMemory,initialReadyMs=null,peakFrameMs=0,lastFrameTime=null,measureRequested=false;
const performancePanel=createPerformance(()=>wasmMemory?.buffer.byteLength);
window.gamePerformanceFrame=(physicsMs,worldMs)=>performancePanel.frame(performance.now(),physicsMs,worldMs);
window.gamePerformanceCounters=performancePanel.counters;
const measurementWaiters=[];
const bootstrap={manifestUtf16PayloadBytes:0,dependencyGate:worldBootstrapMeasurements};
window.gameNow=()=>performance.now();
window.measureWorld=()=>new Promise(resolve=>{measurementWaiters.push(resolve);measureRequested=true;});
window.gameMeasureRequested=()=>{const requested=measureRequested;measureRequested=false;return requested;};
window.gameMeasurements=json=>{
 const snapshot=JSON.parse(json);
 snapshot.browser={wasmLinearBytes:wasmMemory?.buffer.byteLength??null,jsHeapBytes:performance.memory?.usedJSHeapSize??null,
  initialStartReadyMs:initialReadyMs,peakFrameIntervalMs:peakFrameMs,bootstrap};
 for(const resolve of measurementWaiters.splice(0))resolve(snapshot);
};
let input,survival,dead=false,sleeping=false,audio,menu,feedback,failed=false,worldAdmitted=false,actions=0,frame,renderFailure;
const inventory={selected:0,slots:[0,0,0,0],progress:0};
window.gameInventory=(selected,a,b,c,d,progress)=>{
 inventory.selected=selected;inventory.slots[0]=a;inventory.slots[1]=b;inventory.slots[2]=c;inventory.slots[3]=d;inventory.progress=progress;
 window.gameState.inventory=inventory;feedback?.inventory(inventory);
};
const record={actions:0,context:0,name:'',notice:'',carrying:false,amount:0,capacity:0,hasName:false,noText:true,fluidName:'',condition:-1,held:'',throwCharge:0,throwActive:false,radioVolume:undefined,radioFrequency:undefined,radioAm:false,quantities:[0,0,0,0,0,0]};
const settleSave=window.gameSaveResult;
window.gameSaveResult=(id,kind,payload,error)=>{
 if(kind==='Commit'&&!error){input.clear();actions=0;}
 settleSave(id,kind,payload,error);
};
window.gameInventoryKeys=()=>frame.inventoryHeld|(frame.inventoryReleased<<4);
window.gameKeys=()=>{frame=input.takeFrame();return frame.held;};
window.gamePressed=()=>frame.pressed;window.gameReleased=()=>frame.released;
window.gameMouseX=()=>frame.mouseX;window.gameMouseY=()=>frame.mouseY;window.gameWheel=()=>frame.wheel;
window.gameDrivingAssists=()=>menu?.takeDrivingAssists()??0;
window.gameDrivingState=(gear,assists,seated)=>{window.gameState.gear=gear;window.gameState.drivingAssists=assists;menu?.drivingState(assists);feedback?.drivingState(gear,seated);};
window.gameActions=()=>{const a=actions;actions=0;return a;};
window.gameState = { speedKmh: 0, camera: 'First person', x: 0, y: 0, z: 0, contacts: 0,
  frameSteps: 0, totalSteps: 0, activeSurfaces: 0, colliderCount: 0, absoluteX: 0, absoluteY: 0, absoluteZ: 0,
  originX: 0, originY: 0, originZ: 0, rebases: 0, travelM: 0, preparedRoads: 0, fuelLiters: 0,
  paused: false, started: false, target: '', targetAction: 0, carrying:false, heldItem:'', rpm:0, ignition:false, running:false, starter:false, seated:false, handbrake:true, throwCharge:0, throwActive:false, fluidVolumes:record.quantities };
window.gameUpdate = (kmh, mode, x, y, z, contacts, steps, surfaces, colliders, absoluteX, absoluteY, absoluteZ,
  originX, originY, originZ, rebases, travelM, preparedRoads, fuelLiters, paused, started) => {
  const now=performance.now();
  if(lastFrameTime!==null)peakFrameMs=Math.max(peakFrameMs,now-lastFrameTime);
  lastFrameTime=now;
  const state = window.gameState;
  state.speedKmh = kmh; state.camera = mode ? 'First person' : 'Chase';
  state.x = x; state.y = y; state.z = z; state.contacts = contacts;
  state.frameSteps = steps; state.totalSteps += steps; state.activeSurfaces = surfaces; state.colliderCount = colliders;
  state.absoluteX = absoluteX; state.absoluteY = absoluteY; state.absoluteZ = absoluteZ;
  state.originX = originX; state.originY = originY; state.originZ = originZ;
  state.rebases = rebases; state.travelM = travelM; state.preparedRoads = preparedRoads;
  state.fuelLiters = fuelLiters; state.paused = paused; state.started = started;
  menu?.sync(started,paused,travelM);
  if(record.context===11||record.context===12){const radio=audio?.radioFrame;record.radioVolume=radio?.volume;record.radioFrequency=radio?.frequency;record.radioAm=radio?.am;feedback?.update(record);}
  feedback?.setVisible(started&&!paused&&!dead&&!sleeping&&Boolean(mode)&&menu?.model.state==='playing');
  survival?.setVisible(started&&!paused&&!sleeping&&menu?.model.state==='playing');
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
 sleep.phase=phase;sleep.eligible=eligible;sleep.charge=charge;sleep.cooldownRemaining=cooldownRemaining;sleep.progress=progress;sleep.held=(input.state.held&262144)!==0;sleep.dead=dead;sleeping=phase===2;survival?.updateSleep(sleep,feedback?.state.enabled);
};
function fail(message,notifyRenderer=true){if(failed)return;failed=true;performancePanel.stop();startup.reconcile(performance.getEntriesByType('resource'));stopStartupEnvironment();startup.fail({...startupFailureDetails,message,loading:loading.snapshot()});startupDone=true;loading.fail();const text='Cannot start the game. '+message+' Reload to try again.';if(menu){menu.fail(text);status.textContent='';}else{status.textContent='';runtimeError.textContent=text;runtimeError.hidden=false;}if(notifyRenderer)renderFailure?.(message);}
function finishStartup(){if(failed||startupDone||!startupReady||!input||!menu)return;startup.reconcile(performance.getEntriesByType('resource'));stopStartupEnvironment();startup.finish({inputReady:true});startupDone=true;}
window.gameFailure=message=>fail(message,false);
window.gameLoading=()=>{if(failed)return;loading.start();loading.stage('Loading the garage...');menu.loading();};
window.gameReady=()=>{if(failed||!worldAdmitted)return;if(initialReadyMs===null)initialReadyMs=performance.now()-bootStarted;loading.finish();menu.ready();startupReady=true;finishStartup();};
window.addEventListener('error',event=>fail(event.message||'The runtime stopped.'));
window.addEventListener('unhandledrejection',event=>fail(event.reason?.message||String(event.reason)));
canvas.addEventListener('webglcontextlost',event=>{event.preventDefault();fail('The graphics context was lost.');});
async function load(){
 cpuProfile.begin('startup');
 startup.begin('webgl');
 const probe=document.createElement('canvas'),graphics=probe.getContext('webgl2');
 if(!graphics)throw Error('This browser cannot use WebGL2.');
 graphics.getExtension('WEBGL_lose_context')?.loseContext();
 startup.end('webgl');startup.begin('manifest');
 // Download and compile the game code while the manifest downloads.
 const code=Promise.all([import('./pkg/my_summer_slop.js'),fetch('./pkg/my_summer_slop_bg.wasm').then(response=>{
  if(!response.ok)throw Error('Cannot load game code (HTTP '+response.status+').');
  return WebAssembly.compileStreaming(response);
 })]);
 code.catch(()=>{});
 const response=await fetch('./assets/reference/manifest.json');
 if(!response.ok)throw Error('Cannot load reference manifest (HTTP '+response.status+').');
 const bytes=new Uint8Array(await response.arrayBuffer());
 status.textContent='Loading game code...';
 loading.stage(status.textContent);
 const [{default:init,run,manifest_shell,source_render_failure,cpu_profile},module]=await code;
 const exports=await init({module_or_path:module});wasmMemory=exports.memory;renderFailure=source_render_failure;
 rustCpuProfile=cpu_profile;if(startupCpuProfile)rustCpuProfile(true);
 // Rust copies only the shell fields, so JS does not parse the world records.
 const text=manifest_shell(bytes),manifest=JSON.parse(text);
 const {manifest_sha256:manifestHash}=await manifestStamp(bytes,manifest);
 fetched({path:'assets/reference/manifest.json',requiredBytes:bytes.byteLength,receivedBytes:bytes.byteLength});
 startup.build({sourceBuildId:manifest.build_id,manifestSha256:manifestHash,wasm:'release'});
 configureAssetCatalog(manifest.content_base,manifest.file_catalog,fetched);worldAdmitted=true;
 startup.end('manifest');startup.begin('interface');
 input=createInput(canvas,()=>menu?.model.state==='playing' && document.pointerLockElement===canvas,manifest.driving.bindings,cpuProfile);
 bootstrap.manifestUtf16PayloadBytes=text.length*2;
 status.textContent='Loading interface assets...';
 loading.stage(status.textContent);
 await loadUiAssets(manifest.ui);
 survival=createSurvival(manifest.ui,manifest.sleep.cooldown_seconds);
 audio=createAudio(manifest.audio);installAudioBridge(window,audio);
 window.addEventListener('pagehide',()=>audio.dispose());
 menu=createMenu({canvas,ui:manifest.ui,drivingBindings:manifest.driving.bindings,saveSession,audio,clearInput:()=>input.clear(),queueAction:a=>{actions=a;}});
 const switchIconBytes=await readPackageAsset(manifest.lighting.switch_icon.path);
 const switchIconUrl=URL.createObjectURL(new Blob([switchIconBytes],{type:'image/png'}));
 const switchIcon=new Image();switchIcon.src=switchIconUrl;await switchIcon.decode();
 checkImageSize(assetReceipt(manifest.lighting.switch_icon.path),switchIcon.naturalWidth,switchIcon.naturalHeight);
 feedback=createFeedback(manifest.ui,switchIconUrl,manifest);
 startup.end('interface');
 if(failed)return;status.textContent='Loading the garage...';loading.stage(status.textContent);
 startup.begin('generation');startup.begin('decodeUpload');startup.begin('pipeline');
 run(bytes,manifestHash);
 cpuProfile.end('startup');
}
load().catch(error=>fail(error?.message??String(error)));
