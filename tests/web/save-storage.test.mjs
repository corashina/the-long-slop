import test from 'node:test';
import assert from 'node:assert/strict';
import {webcrypto as crypto} from 'node:crypto';
import {createSaveStorage, manifestStamp, verifyRecord} from '../../web/save-storage.js';

const payload={schema_version:9,saved_at:'2026-10-12',world:{seed:42},source:{build_id:11140890}};
const tick=()=>new Promise(resolve=>setImmediate(resolve));
// Inject request events only. Real database behavior belongs to the browser fixture.
function boundary(){
 const open={};const request={};const tx={abort(){tx.onabort?.();},objectStore(){return {get(){return request;},put(record,key){tx.record=record;tx.key=key;return request;}};}};
 const db={version:1,objectStoreNames:{contains:()=>true},closed:0,close(){this.closed++;},transaction(...args){db.args=args;return tx;}};
 const indexedDB={open(name,version){indexedDB.args=[name,version];return open;}};
 return {open,request,tx,db,indexedDB,storage:createSaveStorage({indexedDB,crypto})};
}
async function connect(b){while(!b.open.onsuccess)await tick();b.open.result=b.db;b.open.onsuccess();await tick();}

test('record_checksum_rejects_corruption_and_preserves_property_order',async()=>{
 const b=boundary();const promise=b.storage.write(payload);await connect(b);b.request.onsuccess?.();b.tx.oncomplete();await promise;
 assert.equal(typeof b.tx.record.payload,'object');assert.equal(JSON.stringify(b.tx.record.payload),JSON.stringify(payload));assert.deepEqual(await verifyRecord(b.tx.record,crypto),payload);
 await assert.rejects(verifyRecord({...b.tx.record,payload:{...payload,saved_at:'changed'}},crypto),/checksum/i);
 await assert.rejects(verifyRecord({...b.tx.record,payload:Object.fromEntries(Object.entries(payload).reverse())},crypto),/checksum/i);
 await assert.rejects(verifyRecord({...b.tx.record,payload:JSON.stringify(payload)},crypto),/record/i);
 await assert.rejects(verifyRecord({...b.tx.record,version:2},crypto),/record/i);
});
test('write_and_read_wait_for_transaction_completion',async()=>{
 const b=boundary();let done=false;const write=b.storage.write(payload).then(()=>{done=true;});await connect(b);
 b.request.onsuccess?.();await tick();assert.equal(done,false);
 assert.deepEqual(b.indexedDB.args,['tld-browser-save',1]);assert.deepEqual(b.db.args,['saves','readwrite',{durability:'strict'}]);assert.equal(b.tx.key,'manual');
 b.tx.oncomplete();await write;
 const read=b.storage.read();await tick();b.request.result=b.tx.record;b.request.onsuccess();let readDone=false;read.then(()=>{readDone=true;});await tick();assert.equal(readDone,false);b.tx.oncomplete();assert.deepEqual(await read,b.tx.record);
});
test('unavailable_storage_and_quota_errors_reject',async()=>{
 await assert.rejects(createSaveStorage({indexedDB:undefined,crypto}).read(),/unavailable/i);
 const b=boundary();const write=b.storage.write(payload);await connect(b);b.tx.error=new DOMException('Full','QuotaExceededError');b.tx.onabort();await assert.rejects(write,{name:'QuotaExceededError'});
});
test('incompatible_upgrade_aborts_without_creating_a_store',async()=>{
 const b=boundary();let aborted=false;let created=false;b.open.transaction={abort(){aborted=true;}};b.db.createObjectStore=()=>{created=true;};
 const read=b.storage.read();await tick();b.open.result=b.db;b.open.onupgradeneeded({oldVersion:2});await assert.rejects(read,/version/i);assert.equal(aborted,true);assert.equal(created,false);
});
test('cancel_during_delayed_hash_never_opens_a_transaction',async()=>{
 let finish;const delayedCrypto={subtle:{digest:()=>new Promise(resolve=>{finish=resolve;})}};
 const b=boundary();const storage=createSaveStorage({indexedDB:b.indexedDB,crypto:delayedCrypto});const controller=new AbortController();const write=storage.write(payload,{signal:controller.signal});controller.abort();finish(new ArrayBuffer(32));await assert.rejects(write,{name:'AbortError'});assert.equal(b.indexedDB.args,undefined);
});

test('schema_5_rejection_preserves_stored_bytes',async()=>{
 const {createSaveSession}=await import('../../web/save-session.js');
 const b=boundary(); const write=b.storage.write({...payload,schema_version:5}); await connect(b); b.tx.oncomplete(); await write;
 const before=JSON.stringify(b.tx.record); let commits=0;
 const session=createSaveSession({drawDistance:()=>2500,storage:{read:async()=>b.tx.record,write:async()=>{throw new Error('Unexpected write');}},runtime:{prepare:async()=>{throw new Error('Schema version');},commit:async()=>{commits++;},discard:()=>{}}});
 await assert.rejects(session.load(),/Schema version/); assert.equal(JSON.stringify(b.tx.record),before); assert.equal(commits,0);
});
