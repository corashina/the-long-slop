import test from 'node:test';
import assert from 'node:assert/strict';
import {webcrypto as crypto} from 'node:crypto';
import {createSaveStorage, manifestStamp, verifyRecord} from '../../web/save-storage.js';

const payload={schema_version:1,saved_at:'2026-10-12',world:{seed:'42'},source:{build_id:11140890}};
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
 assert.equal(b.tx.record.payload,JSON.stringify(payload));assert.deepEqual(await verifyRecord(b.tx.record,crypto),payload);
 await assert.rejects(verifyRecord({...b.tx.record,payload:b.tx.record.payload+' '},crypto),/checksum/i);
 await assert.rejects(verifyRecord({...b.tx.record,version:2},crypto),/record/i);
});
test('manifest_stamp_hashes_original_bytes_and_copies_reference_fields',async()=>{
 const bytes=new TextEncoder().encode('{ "build_id":11140890 }');
 const reference={app_id:1017180,build_id:11140890,source_hashes:[{file:'source',sha256:'a'.repeat(64)}]};
 const stamp=await manifestStamp(bytes,reference,crypto);
 assert.deepEqual(Object.keys(stamp),['app_id','build_id','manifest_sha256','source_hashes']);
 assert.equal(stamp.manifest_sha256.length,64);assert.deepEqual(stamp.source_hashes,reference.source_hashes);
 reference.source_hashes[0].file='changed';assert.equal(stamp.source_hashes[0].file,'source');
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
test('blocked_open_rejects_and_closes_late_connection',async()=>{
 const b=boundary();const read=b.storage.read();await tick();b.open.onblocked();await assert.rejects(read,/blocked/i);b.open.result=b.db;b.open.onsuccess();assert.equal(b.db.closed,1);
});
test('incompatible_upgrade_aborts_without_creating_a_store',async()=>{
 const b=boundary();let aborted=false;let created=false;b.open.transaction={abort(){aborted=true;}};b.db.createObjectStore=()=>{created=true;};
 const read=b.storage.read();await tick();b.open.result=b.db;b.open.onupgradeneeded({oldVersion:2});await assert.rejects(read,/version/i);assert.equal(aborted,true);assert.equal(created,false);
});
test('version_change_closes_connection_and_cancel_aborts_write',async()=>{
 const b=boundary();const controller=new AbortController();const write=b.storage.write(payload,{signal:controller.signal});await connect(b);controller.abort();await assert.rejects(write,{name:'AbortError'});
 b.db.onversionchange();assert.equal(b.db.closed,1);
});
test('cancel_during_delayed_hash_never_opens_a_transaction',async()=>{
 let finish;const delayedCrypto={subtle:{digest:()=>new Promise(resolve=>{finish=resolve;})}};
 const b=boundary();const storage=createSaveStorage({indexedDB:b.indexedDB,crypto:delayedCrypto});const controller=new AbortController();const write=storage.write(payload,{signal:controller.signal});controller.abort();finish(new ArrayBuffer(32));await assert.rejects(write,{name:'AbortError'});assert.equal(b.indexedDB.args,undefined);
});
test('close_rejects_pending_open_and_closes_late_connection',async()=>{
 const b=boundary();const read=b.storage.read();await tick();b.storage.close();await assert.rejects(read,/closed/i);b.open.result=b.db;b.open.onsuccess();assert.equal(b.db.closed,1);await assert.rejects(b.storage.read(),/closed/i);
});
test('cancel_during_open_does_not_start_a_transaction',async()=>{
 const b=boundary();const controller=new AbortController();const write=b.storage.write(payload,{signal:controller.signal});while(!b.open.onsuccess)await tick();controller.abort();b.open.result=b.db;b.open.onsuccess();await assert.rejects(write,{name:'AbortError'});assert.equal(b.db.args,undefined);
});
