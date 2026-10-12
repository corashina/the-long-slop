import {saveRuntime as runtime} from '../../web/main.js';
import {createSaveStorage} from '../../web/save-storage.js';
const host=parent.document,output=host.querySelector('#results'),storage=createSaveStorage();
let holdKind='',heldResult=null,action=0,pointerLockRequests=0;
const result=window.gameSaveResult,actions=window.gameActions,open=IDBFactory.prototype.open,transaction=IDBDatabase.prototype.transaction;
const capture=HTMLCanvasElement.prototype.requestPointerLock;
HTMLCanvasElement.prototype.requestPointerLock=function(...args){pointerLockRequests++;return capture.apply(this,args);};
window.gameSaveResult=(...args)=>{if(holdKind&&args[1]===holdKind&&!args[3]){heldResult=args;holdKind='';host.querySelector('#release').disabled=false;output.textContent='Operation pending. Use Release operation.';}else result(...args);};
window.gameActions=()=>{const value=actions()|action;action=0;return value;};
const frame=()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
function log(text){output.textContent=text;}
function assert(value,message){if(!value)throw Error(message);}
function loadButton(){return document.querySelector(document.querySelector('#start-actions').hidden?'#load':'#start-load');}
function button(id,work){host.querySelector(id).addEventListener('click',()=>Promise.resolve().then(work).catch(error=>log('FAIL '+error.stack)));}
async function clearSave(){
 const db=await new Promise((resolve,reject)=>{const request=indexedDB.open('tld-browser-save',1);request.onupgradeneeded=()=>request.result.createObjectStore('saves');request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);});
 try{await new Promise((resolve,reject)=>{const tx=db.transaction('saves','readwrite');tx.objectStore('saves').delete('manual');tx.oncomplete=resolve;tx.onabort=()=>reject(tx.error);});}finally{db.close();}
}
for(const [id,kind,selector] of [['#pending-save','Capture','#save'],['#pending-load','Prepare','#load'],['#pending-new-game','PrepareNewGame','#pause-new-game']])button(id,()=>{holdKind=kind;const target=kind==='Prepare'?loadButton():document.querySelector(selector);if(target.disabled||target.closest('[hidden]')){holdKind='';throw Error('Create a paused game first. Save before Hold Load.');}target.click();log('Waiting for operation result...');});
button('#release',async()=>{const response=heldResult;heldResult=null;host.querySelector('#release').disabled=true;if(response)result(...response);await frame();log('Operation released.');});
button('#missing',async()=>{await clearSave();loadButton().click();log('Missing save removed. Inspect the menu alert.');});
button('#incompatible',async()=>{const save=await runtime.capture();save.schema_version++;await storage.write(save);loadButton().click();log('Incompatible record written. Inspect the menu alert.');});
button('#storage-failure',()=>{const fail=()=>{throw new DOMException('Save storage is unavailable.','InvalidStateError');};IDBFactory.prototype.open=fail;IDBDatabase.prototype.transaction=fail;log('Storage failure enabled. Use Save or Load in the menu.');});
button('#storage-restore',()=>{IDBFactory.prototype.open=open;IDBDatabase.prototype.transaction=transaction;log('Storage restored.');});
button('#continue',async()=>{const before=window.gameState.totalSteps;action=4;await frame();await frame();assert(!window.gameState.paused&&window.gameState.totalSteps>before,'Continue did not advance simulation.');action=2;await frame();assert(window.gameState.paused,'Pause did not stop simulation.');log('PASS usable_continue (simulation action; mouse capture checked separately).');});
button('#preserve',async()=>{const before=await storage.read();assert(before!==null,'Use Save first.');const requests=pointerLockRequests;document.querySelector('#pause-new-game').click();for(let i=0;i<120&&document.querySelector('#pause-new-game').disabled;i++)await frame();assert(window.gameState.started&&window.gameState.paused,'New game did not stay paused.');assert(JSON.stringify(await storage.read())===JSON.stringify(before),'New game changed the saved record.');assert(pointerLockRequests===requests,'New game requested mouse capture.');log('PASS new_game_keeps_saved_record and paused state; zero mouse capture requests.');});
function ready(){host.querySelectorAll('header button').forEach(button=>button.disabled=button.id==='release');log('Runtime ready. Use New game in the menu.');}
const gameReady=window.gameReady;window.gameReady=()=>{gameReady();ready();};if(!document.querySelector('#new-game').disabled)ready();
