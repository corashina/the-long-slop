import {normalizeSnapshot} from './save-normalize.js';
export {normalizeSnapshot} from './save-normalize.js';
import {createSaveStorage,verifyRecord} from '../../web/save-storage.js';
const DATABASE='tld-browser-save',STORE='saves',KEY='manual';
const channel=new BroadcastChannel('tld-save-fixture');
const output=document.querySelector('#results');
const mode=new URL(location.href).searchParams.get('mode');
function log(message){output.textContent+=(output.textContent==='Ready.'?'\n':'')+message+'\n';}
function assert(condition,message){if(!condition)throw new Error(message);}
function equal(actual,expected){assert(JSON.stringify(actual)===JSON.stringify(expected),'Values differ.');}
async function pass(name,work){await work();log('PASS '+name);}
function open(version=1){return new Promise((resolve,reject)=>{const request=indexedDB.open(DATABASE,version);request.onupgradeneeded=()=>request.result.createObjectStore(STORE);request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);request.onblocked=()=>reject(new Error('Fixture database is blocked. Release the helper tab.'));});}
function removeDatabase(){return new Promise((resolve,reject)=>{const request=indexedDB.deleteDatabase(DATABASE);request.onsuccess=resolve;request.onerror=()=>reject(request.error);request.onblocked=()=>reject(new Error('Fixture reset is blocked. Release the helper tab.'));});}
function replace(db,record,abort=false){return new Promise((resolve,reject)=>{const tx=db.transaction(STORE,'readwrite',{durability:'strict'});tx.oncomplete=resolve;tx.onabort=()=>abort?resolve():reject(tx.error);tx.onerror=()=>{};const request=tx.objectStore(STORE).put(record,KEY);if(abort)request.onsuccess=()=>tx.abort();});}
function source(path_id){return {file:'fixture.assets',path_id};}
function instance(path_id){return {kind:'initial',source:source(path_id)};}
const snapshotA={schema_version:2,driving_assists:{automatic_clutch:true,automatic_gears:true},saved_at:'A',source:{app_id:1017180,build_id:11140890,manifest_sha256:'a'.repeat(64),source_hashes:[]},world:{seed:'42',records:[{kind:'pump',instance:{kind:'station',seed:'42',index:'1',role:'pump'},valve:0}]},player:{pose:{position:[1048577.25,-4,2048.5],rotation:[0,0,0,1]},yaw:0,pitch:0,seat:null,crouched:false,vertical_velocity:0,recoil_velocity:[0,0,0],running:false,carry:null},vehicles:[],bodies:[],items:[],attachments:{parts:[],wheels:[],tires:[]},mechanisms:{controls:[],openings:[]},progress:{selected_vehicle:{kind:'initial',source:{file:'fixture',path_id:'1'}},start_attempts:'0'},lighting:{cycle:{elapsed_seconds:0,cycle_seconds:1800,evaluated_cycle_seconds:1800,cloud_coordinate:0,star_rotation:0},controllers:[]},view:{camera:{kind:'first_person'}}};
async function storageChecks(){
 await removeDatabase();const storage=createSaveStorage({indexedDB,crypto});let db;
 try{
  await storage.write(snapshotA);const recordA=await storage.read();
  await pass('stored_object_property_order_and_checksum',async()=>{assert(recordA.payload&&typeof recordA.payload==='object'&&!Array.isArray(recordA.payload),'Payload is not an object.');assert(JSON.stringify(recordA.payload)===JSON.stringify(snapshotA),'Stored property order changed.');equal(await verifyRecord(recordA,crypto),snapshotA);});
  db=await open();
  await pass('aborted_write_keeps_previous_record',async()=>{await replace(db,{...recordA,payload:{...snapshotA,saved_at:'B'}},true);equal(await storage.read(),recordA);});
  await pass('corrupt_record_rejected',async()=>{await replace(db,{...recordA,payload:{...recordA.payload,saved_at:'changed'}});let rejected=false;try{await verifyRecord(await storage.read(),crypto);}catch(error){rejected=/checksum/.test(error.message);}assert(rejected,'Corrupt record passed verification.');await storage.write(snapshotA);});
  await pass('string_payload_rejected',async()=>{await replace(db,{...recordA,payload:JSON.stringify(snapshotA)});let rejected=false;try{await verifyRecord(await storage.read(),crypto);}catch(error){rejected=/record/.test(error.message);}assert(rejected,'String payload passed verification.');await storage.write(snapshotA);});
  await pass('normalize_preserves_absolute_coordinates',async()=>{const a=structuredClone(snapshotA),b=structuredClone(snapshotA);a.items=[{instance:instance('2'),pose:{position:[1048577.25,0,0]}},{instance:instance('1'),pose:{position:[-4096.5,0,0]}}];b.items=a.items.toReversed();b.saved_at='B';equal(normalizeSnapshot(a),normalizeSnapshot(b));assert(normalizeSnapshot(a).player.pose.position[0]===1048577.25,'Coordinates changed.');equal(a.items[0].instance,instance('2'));});
 }finally{db?.close();storage.close();}
 log('Storage checks complete.');
}
function helperMessage(type,timeout=10000){return new Promise((resolve,reject)=>{
 const timer=setTimeout(()=>{channel.removeEventListener('message',receive);reject(new Error('Helper tab did not respond: '+type));},timeout);
 function receive(event){if(event.data.type!==type)return;clearTimeout(timer);channel.removeEventListener('message',receive);resolve(event.data);}
 channel.addEventListener('message',receive);
});}
async function interruptedWrite(child){
 const storage=createSaveStorage({indexedDB,crypto});
 try{
  const ready=helperMessage('interrupt_ready');channel.postMessage({type:'probe'});await ready;
  await storage.write(snapshotA);const recordA=await storage.read();
  const pending=helperMessage('write_pending');channel.postMessage({type:'start_interruption',record:{...recordA,payload:{...snapshotA,saved_at:'incomplete B'}}});await pending;
  assert(child,'Open the interruption tab from this button.');child.close();
  await pass('interrupted_page_reads_complete_record',async()=>equal(await storage.read(),recordA));
 }finally{storage.close();child?.close();}
}
async function blockedOpen(child){
 const ready=helperMessage('blocker_ready');channel.postMessage({type:'probe'});await ready;
 let lateClosed=false;let lateResolve;const late=new Promise(resolve=>{lateResolve=resolve;});
 // A fixture-only upgrade triggers a real blocked event. Version 1 opens cannot trigger that event.
 const boundary={open(){
  const request=indexedDB.open(DATABASE,2),facade={};
  for(const name of ['onblocked','onerror'])Object.defineProperty(facade,name,{set(handler){request[name]=handler;}});
  Object.defineProperty(facade,'onsuccess',{set(handler){request.onsuccess=event=>{handler(event);lateResolve();};}});
  Object.defineProperty(facade,'onupgradeneeded',{set(){request.onupgradeneeded=()=>{};}});
  Object.defineProperty(facade,'error',{get:()=>request.error});
  Object.defineProperty(facade,'result',{get:()=>({close(){request.result.close();lateClosed=true;}})});
  return facade;
 }};
 const storage=createSaveStorage({indexedDB:boundary,crypto});
 try{
  await pass('second_tab_blocks_open',async()=>{let rejected=false;try{await storage.read();}catch(error){rejected=/blocked/.test(error.message);}assert(rejected,'Blocked open did not reject.');});
  channel.postMessage({type:'release'});await late;
  await pass('late_successful_open_closes_connection',async()=>assert(lateClosed,'Late connection stayed open.'));
  child?.close();
  await pass('incompatible_database_rejected_without_upgrade',async()=>{const production=createSaveStorage({indexedDB,crypto});try{let rejected=false;try{await production.read();}catch(error){rejected=error.name==='VersionError';}assert(rejected,'Version 2 database passed.');}finally{production.close();}const db=await open(2);assert(db.version===2,'Database version changed.');db.close();});
 }finally{storage.close();channel.postMessage({type:'release'});child?.close();}
}
let helperDb=null,helperTx=null;
async function startHelper(){
 if(mode==='blocker'){
  helperDb=await open();helperDb.onversionchange=()=>log('Holding version 1 until release.');
  channel.postMessage({type:'blocker_ready'});log('Blocker holds version 1.');
 }else if(mode==='interrupt'){channel.postMessage({type:'interrupt_ready'});log('Interruption helper ready.');}
 channel.onmessage=async event=>{
  if(event.data.type==='probe')channel.postMessage({type:mode==='blocker'?'blocker_ready':'interrupt_ready'});
  if(event.data.type==='release'){helperTx?.abort();helperDb?.close();helperDb=null;log('Helper released.');}
  if(mode==='interrupt'&&event.data.type==='start_interruption'){
   helperDb=await open();helperTx=helperDb.transaction(STORE,'readwrite',{durability:'strict'});
   helperTx.onerror=()=>{};helperTx.onabort=()=>log('Interrupted write aborted.');
   const store=helperTx.objectStore(STORE),request=store.put(event.data.record,KEY);
   request.onsuccess=()=>{function hold(){const request=store.get(KEY);request.onsuccess=hold;}hold();channel.postMessage({type:'write_pending'});log('B is pending. Close this tab.');};
  }
 };
}
function run(button,work){button.addEventListener('click',()=>{const child=work.childMode?window.open('?mode='+work.childMode,'save-fixture-'+work.childMode):null;button.disabled=true;work(child).catch(error=>log('FAIL '+error.message)).finally(()=>{button.disabled=false;});});}
if(mode){document.querySelectorAll('#storage,#interrupt,#blocked').forEach(button=>{button.hidden=true;});startHelper().catch(error=>log('FAIL '+error.message));}
else{run(document.querySelector('#storage'),storageChecks);interruptedWrite.childMode='interrupt';run(document.querySelector('#interrupt'),interruptedWrite);blockedOpen.childMode='blocker';run(document.querySelector('#blocked'),blockedOpen);}
document.querySelector('#release').addEventListener('click',()=>{channel.postMessage({type:'release'});helperTx?.abort();helperDb?.close();log('Helper released.');});
