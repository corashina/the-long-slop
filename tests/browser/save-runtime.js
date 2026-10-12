import {normalizeSnapshot as normalize} from './save-normalize.js';
import {saveRuntime as runtime} from '../../web/main.js';
import {createSaveSession} from '../../web/save-session.js';
const parentDocument=parent.document,output=parentDocument.querySelector('#results');
const buttons=[parentDocument.querySelector('#runtime'),parentDocument.querySelector('#continue'),parentDocument.querySelector('#roundtrip'),parentDocument.querySelector('#handle')];
let fixtureEnabled=false;
function enable(){buttons.forEach(button=>button.disabled=(button===buttons[0]||button===buttons[3])&&!fixtureEnabled);}
function log(text){output.textContent+='\n'+text;}
function assert(value,message){if(!value)throw Error(message);}
function equal(a,b,message){
 function difference(left,right,path){
  if(JSON.stringify(left)===JSON.stringify(right))return '';
  if(left&&right&&typeof left==='object'&&typeof right==='object'){
   for(const key of new Set([...Object.keys(left),...Object.keys(right)])){const found=difference(left[key],right[key],path+'.'+key);if(found)return found;}
  }
  return path+': '+JSON.stringify(left)+' != '+JSON.stringify(right);
 }
 const detail=difference(a,b,'snapshot');assert(!detail,message+' '+detail);
}

function reconstructed(actual,expected){
 let positionError=0,rotationError=0;
 function compare(left,right,path){
  if(typeof left==='number'&&typeof right==='number'){
   const error=Math.abs(left-right);
   const pose=/\.(pose|local_pose|local_grip)\.(position|rotation)\.\d+$/.exec(path);
   if(pose){
    if(pose[2]==='position'){positionError=Math.max(positionError,error);assert(error<=1e-3,path+' position error '+error);}
    else{rotationError=Math.max(rotationError,error);assert(error<=1e-5,path+' rotation error '+error);}
    return;
   }
  }
  if(left===right)return;
  if(left&&right&&typeof left==='object'&&typeof right==='object'){
   assert(Array.isArray(left)===Array.isArray(right),path+' type changed.');
   equal(Object.keys(left).sort(),Object.keys(right).sort(),path+' fields changed.');
   for(const key of Object.keys(left))compare(left[key],right[key],path+'.'+key);
   return;
  }
  assert(false,path+': '+JSON.stringify(left)+' != '+JSON.stringify(right));
 }
 compare(actual,expected,'snapshot');
 return {positionError,rotationError};
}

const frame=()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
let action=0,fixtureSequence=0,fixtureQueue=[],fixturePending=new Map(),commitCalls=0,discardedCandidate,delayResult=null;
const actions=window.gameActions,hasRequest=window.gameHasSaveRequest,takeRequest=window.gameSaveRequest,result=window.gameSaveResult;
window.gameActions=()=>{const value=actions()|action;action=0;return value;};
window.gameHasSaveRequest=()=>fixtureQueue.length!==0||hasRequest();
window.gameSaveRequest=()=>{
 const text=fixtureQueue.length?JSON.stringify(fixtureQueue.shift()):takeRequest();
 const request=JSON.parse(text);
 if(request.kind==='Commit')commitCalls++;
 if(request.kind==='Discard')discardedCandidate=request.payload;
 return text;
};
window.gameSaveResult=(id,kind,payload,error)=>{
 if(fixturePending.has(id)){
  const {resolve,reject}=fixturePending.get(id);fixturePending.delete(id);
  error?reject(Error(error)):resolve(payload?JSON.parse(payload):undefined);return;
 }
 if(delayResult&&kind==='PrepareNewGame'&&!error){delayResult.receive([id,kind,payload,error]);return;}
 result(id,kind,payload,error);
};
function fixture(kind){const id=0xf0000000+(++fixtureSequence);return new Promise((resolve,reject)=>{fixturePending.set(id,{resolve,reject});fixtureQueue.push({id,kind});});}
function state(){const s=window.gameState;return {camera:s.camera,origin:[s.originX,s.originY,s.originZ]};}
async function rejects(work,path){let error;try{await work();}catch(value){error=value;}assert(error,'Expected rejection.');if(path)assert(error.message.includes(path),'Missing asset path absent: '+error.message);}
async function continueCheck(){
 const before=window.gameState.totalSteps;action=4;await frame();await frame();
 assert(!window.gameState.paused&&window.gameState.totalSteps>before,'Continue did not resume simulation.');
 action=2;await frame();assert(window.gameState.paused,'Pause did not stop simulation.');
 log('PASS usable_continue');
}
async function roundtrip(){
 const fresh=await runtime.prepareNewGame();await runtime.commit(fresh);await frame();
 assert(window.gameState.paused&&window.gameState.started,'New game is not paused.');
 const saved=await runtime.capture();const token=await runtime.prepare(saved);await runtime.commit(token);await frame();
 const error=reconstructed(normalize(await runtime.capture()),normalize(saved));
 assert(window.gameState.paused,'Load resumed simulation.');log('PASS paused_runtime_roundtrip max_position_error_m='+error.positionError+' max_rotation_component_error='+error.rotationError);
}
async function missingHandleCheck(){
 const before=await runtime.capture(),presentation=state(),commits=commitCalls;
 const label=await fixture('FixtureRemoveMaterial');let candidate;
 try{await rejects(async()=>{candidate=await runtime.prepare(before);},label);}
 finally{if(candidate!==undefined)runtime.discard(candidate);await fixture('FixtureRestoreAsset');}
 equal(normalize(await runtime.capture()),normalize(before),'Missing material changed the runtime.');equal(state(),presentation,'Missing material changed camera or origin.');assert(commitCalls===commits,'Missing material preparation committed.');
 log('PASS missing_handle_rejects_before_swap '+label);await continueCheck();
}
async function checks(){
 await roundtrip();
 await missingHandleCheck();
 let before=await runtime.capture(),presentation=state(),commits=commitCalls;
 const path=await fixture('FixtureRemoveAsset');
 try{await rejects(()=>runtime.prepare(before),path);}finally{await fixture('FixtureRestoreAsset');}
 equal(normalize(await runtime.capture()),normalize(before),'Missing asset changed the runtime.');equal(state(),presentation,'Missing asset changed camera or origin.');assert(commitCalls===commits,'Failed preparation committed.');
 log('PASS missing_asset_rejects_before_swap '+path);await continueCheck();
 before=await runtime.capture();presentation=state();commits=commitCalls;
 let release;const arrived=new Promise(resolve=>{release=resolve;});delayResult={receive:release};
 const session=createSaveSession({runtime,storage:{}}),pending=session.newGame();
 const response=await arrived;const staleCandidate=JSON.parse(response[2]);session.cancel();delayResult=null;result(...response);await pending;await frame();
 equal(normalize(await runtime.capture()),normalize(before),'Canceled preparation changed the runtime.');equal(state(),presentation,'Canceled preparation changed camera or origin.');assert(commitCalls===commits,'Canceled preparation committed.');assert(discardedCandidate===staleCandidate,'Stale candidate was not discarded.');
 log('PASS late_prepare_after_cancel_is_discarded');await continueCheck();
 log('Runtime checks complete.');
}
async function fixtureReady(){
 try{await fixture('FixtureRestoreAsset');fixtureEnabled=true;}catch{}
 enable();parentDocument.querySelector('#state').textContent=fixtureEnabled?'Runtime ready. Fault checks enabled.':'Runtime ready. Fault checks require the save-fixture build.';
}
const ready=window.gameReady;window.gameReady=()=>{ready();fixtureReady();};
if(!document.querySelector('#new-game').disabled)fixtureReady();
for(const [button,work] of [[buttons[0],checks],[buttons[1],continueCheck],[buttons[2],roundtrip],[buttons[3],async()=>{await roundtrip();await missingHandleCheck();}]])button.addEventListener('click',()=>{buttons.forEach(b=>b.disabled=true);work().catch(error=>log('FAIL '+error.stack)).finally(enable);});
import {createSaveStorage,verifyRecord} from '../../web/save-storage.js';
const storage=createSaveStorage(),session=createSaveSession({runtime,storage});
const proofKey='tld-assembled-save-proof';
const physicalButtons=[...parentDocument.querySelectorAll('[data-physical]')];
let heldKeys=0,pressedKeys=0,releasedKeys=0;
const originalKeys=window.gameKeys,originalPressed=window.gamePressed,originalReleased=window.gameReleased;
window.gameKeys=()=>originalKeys()|heldKeys;
window.gamePressed=()=>{const value=originalPressed()|pressedKeys;pressedKeys=0;return value;};
window.gameReleased=()=>{const value=originalReleased()|releasedKeys;releasedKeys=0;return value;};
async function inputSteps(held,steps,pressed=0){
 action=4;await frame();heldKeys=held;pressedKeys=pressed;const start=window.gameState.totalSteps;
 const deadline=performance.now()+90000;
 while(window.gameState.totalSteps-start<steps){await frame();assert(performance.now()<deadline,'Simulation step timeout.');}
 action=2;heldKeys=0;releasedKeys=held;await frame();await frame();
 assert(window.gameState.paused,'Fixture did not pause.');
}
async function releaseHeld(){
 async function steps(count){const start=window.gameState.totalSteps,deadline=performance.now()+90000;while(window.gameState.totalSteps-start<count){await frame();assert(performance.now()<deadline,'Release input step timeout.');}}
 heldKeys=0;pressedKeys=0;releasedKeys=0;action=4;await frame();await steps(2);heldKeys=256;pressedKeys=256;await steps(2);
 heldKeys=0;releasedKeys=256;await steps(2);action=2;await frame();await frame();
 log('CARRY_RELEASE_INPUT '+JSON.stringify({carrying:window.gameState.carrying,throwCharge:window.gameState.throwCharge,throwActive:window.gameState.throwActive,steps:window.gameState.totalSteps}));
}
async function topology(){
 const value=await fixture('FixtureInspect');
 assert(value.duplicateAssemblyBodies===0,'Duplicate active assembly bodies: '+value.duplicateAssemblyBodies);
 assert(value.badColliderOwners===0,'Invalid collider owners.');return value;
}
async function saveReload(kind,evidence){
 const snapshot=await runtime.capture(),inspection=await topology();
 await session.save();const record=await storage.read(),stored=await verifyRecord(record);
 assert(stored.schema_version===1,'Stored schema is not 1.');
 equal(normalize(stored),normalize(snapshot),'Stored state differs.');
 sessionStorage.setItem(proofKey,JSON.stringify({kind,snapshot:stored,inspection,evidence}));
 log('PASS '+kind+'_manual_slot_saved '+JSON.stringify({inspection,evidence}));
 storage.close();parent.location.reload();
}
async function driveSave(){
 await session.newGame();await frame();
 const setup=await fixture('FixtureSetupDriving');equal(setup.velocity,[0,0,0],'Setup injected velocity.');
 const before=await runtime.capture(),initial=await topology();
 await inputSteps(1024,240,1024);
 assert(window.gameState.running,'Normal ignition input did not start the engine.');
 await inputSteps(1,360);
 const after=await runtime.capture(),inspection=await topology();
 const displacement=Math.hypot(...inspection.vehiclePosition.map((v,i)=>v-initial.vehiclePosition[i]));
 assert(displacement>4,'Throttle did not move the chassis four metres.');
 assert(inspection.rebases>initial.rebases,'Normal motion did not trigger a rebase.');
 assert(JSON.stringify(inspection.origin)!==JSON.stringify(initial.origin),'Origin did not shift.');
 assert(inspection.wheelContacts.some(Boolean),'Driven wheels lack ground contact.');
 assert(after.attachments.wheels.some((w,i)=>w.rotation!==before.attachments.wheels[i].rotation),'Wheel rotation did not advance.');
 const fuel=s=>s.items.filter(i=>i.fluids).reduce((sum,i)=>sum+i.fluids[0],0);
 assert(fuel(after)<fuel(before),'Running engine did not consume fuel.');
 await saveReload('driving_rebase',{setup,displacement,fuelBefore:fuel(before),fuelAfter:fuel(after),tripBefore:before.progress.trip_m,tripAfter:after.progress.trip_m});
}
async function interactionSave(){
 await session.newGame();await frame();await inputSteps(0,120);
 const removed=await fixture('FixtureDetachAndCarry');assert(removed.supportCount===3,'Removal did not reduce wheel support.');
 await inputSteps(0,12);const stable=await topology();
 assert(stable.carryJoints===1&&stable.held===removed.removedPart,'Original carry did not survive twelve steps.');
 log('PASS original_carry_survives_resume '+JSON.stringify(stable));
 const fluid=await fixture('FixtureTransferFluid');assert(fluid.transferred===0.25,'Fluid transfer failed.');
 const cycle=await fixture('FixtureSetupCycle');
 const snapshot=await runtime.capture();assert(snapshot.player.carry,'No held item.');
 assert(snapshot.lighting.controllers.some(c=>(c.energy>0&&c.energy<1)||c.switch_motion?.kind==='moving'),'No active fade.');
 await saveReload('parts_fluid_carry_cycle',{removed,fluid,cycle});
}
async function resumeReloadProof(){
 const text=sessionStorage.getItem(proofKey);if(!text)return;
 const proof=JSON.parse(text);sessionStorage.removeItem(proofKey);
 await session.load();await frame();
 const current=await runtime.capture(),errors=reconstructed(normalize(current),normalize(proof.snapshot));
 if(proof.kind==='default_manual_slot'){
  assert(window.gameState.paused,'Default load resumed simulation.');log('PASS default_manual_slot_page_reload '+JSON.stringify(errors));await continueCheck();return;
 }
 const initial=await topology();
 assert(window.gameState.paused,'Reload resumed without Continue.');
 equal(initial.wheelSupport,proof.inspection.wheelSupport,'Wheel support changed.');
 equal(initial.carryJoints,proof.inspection.carryJoints,'Carry joint count changed.');
 log('PASS '+proof.kind+'_page_reload_snapshot '+JSON.stringify({errors,inspection:initial,evidence:proof.evidence}));
 if(proof.kind==='driving_rebase'){
  await inputSteps(1,60);
  const after=await runtime.capture(),inspection=await topology();
  assert(after.progress.trip_m>current.progress.trip_m,'Resumed throttle did not advance trip.');
  assert(Math.hypot(...inspection.vehiclePosition.map((v,i)=>v-initial.vehiclePosition[i]))>0.1,'Resumed chassis did not move.');
  assert(inspection.wheelContacts.some(Boolean),'Resumed wheel queries found no support.');
  assert(Math.abs(inspection.vehiclePosition[1]-initial.vehiclePosition[1])<1,'Resumed chassis fell through road.');
  log('PASS resumed_driving_collision_wheel_support '+JSON.stringify(inspection));
 }else{
  await inputSteps(0,12);
  const after=await runtime.capture(),inspection=await topology();
  assert(inspection.wheelContacts.some(Boolean),'Resumed wheel queries found no support.');
  assert(inspection.carryJoints===1&&after.player.carry,'Resumed carry joint is missing.');
  assert(after.lighting.cycle.cycle_seconds<current.lighting.cycle.cycle_seconds,'Cycle did not wrap.');
  assert(after.lighting.controllers.some((c,i)=>c.energy!==current.lighting.controllers[i].energy||JSON.stringify(c.switch_motion)!==JSON.stringify(current.lighting.controllers[i].switch_motion)),'Lamp fade did not resume.');
  assert(Math.abs(inspection.vehiclePosition[1]-initial.vehiclePosition[1])<1,'Chassis fell through garage.');
  const fluid=await fixture('FixtureTransferFluid');assert(fluid.transferred===0.25,'Resumed fluid transfer failed.');
  log('PASS resumed_parts_collision_carry_fluid_cycle_fade '+JSON.stringify({inspection,fluid,cycleBefore:current.lighting.cycle,cycleAfter:after.lighting.cycle}));
  await releaseHeld();
  const dropped=await runtime.capture();assert(dropped.player.carry===null,'Normal grab input did not drop the restored item.');
  log('PASS restored_carry_normal_drop');
 }
 log('Assembled reload checks complete.');
}
async function rawRecord(record,abort=false){
 const db=await new Promise((resolve,reject)=>{const r=indexedDB.open('tld-browser-save',1);r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);});
 try{await new Promise((resolve,reject)=>{const tx=db.transaction('saves','readwrite');tx.oncomplete=resolve;tx.onabort=()=>abort?resolve():reject(tx.error);const r=tx.objectStore('saves').put(record,'manual');if(abort)r.onsuccess=()=>tx.abort();});}finally{db.close();}
}
async function activeFailureChecks(){
 await session.newGame();await frame();await session.save();
 const original=await storage.read(),before=normalize(await runtime.capture()),presentation=state();
 const preserved=async(label,work)=>{await rejects(work);equal(normalize(await runtime.capture()),before,label+' changed active state.');equal(state(),presentation,label+' changed camera/origin.');log('PASS active_session_preserved_'+label);};
 await rawRecord({...original,payload:'pending replacement'},true);equal(await storage.read(),original,'Abort changed slot.');equal(normalize(await runtime.capture()),before,'Abort changed runtime.');log('PASS abort_preserves_slot_and_active_session');
 await rawRecord({...original,payload:original.payload+' '});const corrupt=await storage.read();await preserved('corrupt',()=>session.load());equal(await storage.read(),corrupt,'Corrupt load changed record.');
 await rawRecord(original);
 const source=JSON.parse(original.payload);source.source.manifest_sha256='0'.repeat(64);await storage.write(source);const incompatible=await storage.read();await preserved('source_mismatch',()=>session.load());equal(await storage.read(),incompatible,'Source mismatch changed record.');
 await rawRecord(original);
 const unavailable=createSaveSession({runtime,storage:createSaveStorage({indexedDB:null})});await preserved('storage_unavailable',()=>unavailable.save());equal(await storage.read(),original,'Storage failure changed slot.');
 const path=await fixture('FixtureRemoveAsset');try{await preserved('missing_asset',()=>session.load());equal(await storage.read(),original,'Missing asset changed slot.');}finally{await fixture('FixtureRestoreAsset');}
 log('Active storage failure checks complete.');
}
for(const [id,work] of [['stored-release',async()=>{await session.load();await frame();assert((await runtime.capture()).player.carry,'Saved item is not held.');await releaseHeld();assert((await runtime.capture()).player.carry===null,'Normal grab input did not drop restored item.');log('PASS restored_carry_normal_drop');}],['original-carry',async()=>{await session.newGame();await frame();await inputSteps(0,120);await fixture('FixtureDetachAndCarry');const before=await topology(),saved=await runtime.capture();await inputSteps(0,12);const after=await topology();log('ORIGINAL_CARRY '+JSON.stringify({before,after,player:saved.player,heldBody:saved.bodies.find(b=>JSON.stringify(b.identity.root)===JSON.stringify(saved.player.carry?.item))}));assert(after.carryJoints===1&&after.held!==null,'Original carry also drops before any save/load.');log('PASS original_carry_survives_resume');}],['inspect-topology',async()=>log('TOPOLOGY '+JSON.stringify(await fixture('FixtureInspect')))],['drive-save',driveSave],['interaction-save',interactionSave],['active-failures',activeFailureChecks]])parentDocument.querySelector('#'+id).addEventListener('click',()=>{physicalButtons.forEach(b=>b.disabled=true);work().catch(e=>log('FAIL '+e.stack)).finally(()=>physicalButtons.forEach(b=>b.disabled=!fixtureEnabled));});
const previousFixtureReady=fixtureReady;
fixtureReady=async()=>{await previousFixtureReady();physicalButtons.forEach(b=>b.disabled=!fixtureEnabled);parentDocument.querySelector('#manual-reload').disabled=false;await resumeReloadProof().catch(e=>log('FAIL '+e.stack));};
parentDocument.querySelector('#manual-reload').addEventListener('click',async()=>{
 const button=parentDocument.querySelector('#manual-reload');button.disabled=true;
 try{
  await session.newGame();await frame();await session.save();const record=await storage.read(),snapshot=await verifyRecord(record);
  assert(snapshot.schema_version===1,'Stored schema is not 1.');
  sessionStorage.setItem(proofKey,JSON.stringify({kind:'default_manual_slot',snapshot}));storage.close();parent.location.reload();
 }catch(error){log('FAIL '+error.stack);button.disabled=false;}
});