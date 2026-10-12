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
if(!document.querySelector('#start').disabled)fixtureReady();
for(const [button,work] of [[buttons[0],checks],[buttons[1],continueCheck],[buttons[2],roundtrip],[buttons[3],async()=>{await roundtrip();await missingHandleCheck();}]])button.addEventListener('click',()=>{buttons.forEach(b=>b.disabled=true);work().catch(error=>log('FAIL '+error.stack)).finally(enable);});
