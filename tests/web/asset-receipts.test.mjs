import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {assetUrl, readAsset, configureAssetCatalog,checkImageSize} from '../../web/world-assets.js';

const receipt=(file,bytes)=>({file,bytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex')});
test('selected reads use checked bytes; unused catalog entries cause no requests',async t=>{
 const bytes=new TextEncoder().encode('selected payload');
 const selectedReceipt=receipt('reference/selected.bin',bytes),unusedReceipt=receipt('reference/unused.bin',bytes);
 const fetchCounts=new Map();
 t.mock.method(globalThis,'fetch',async url=>{const path=String(url).split('/package/')[1];fetchCounts.set(path,(fetchCounts.get(path)??0)+1);return new Response(bytes);});
 configureAssetCatalog('./assets/runtime/package/',{files:[selectedReceipt,unusedReceipt],plans:[],dependency_staging_reserve:5242960});
 const decoded=await readAsset('./assets/runtime/package/',selectedReceipt);
 assert.deepEqual(decoded,bytes);
 assert.equal(fetchCounts.get(unusedReceipt.file) ?? 0, 0);
 assert.equal(fetchCounts.get(selectedReceipt.file), 1);
 let decoderCallsAfterInvalidReceipt=0;
 await assert.rejects(async()=>{await readAsset('./assets/runtime/package/',{...selectedReceipt,sha256:'0'.repeat(64)});decoderCallsAfterInvalidReceipt++;},/SHA-256/);
 assert.equal(decoderCallsAfterInvalidReceipt, 0);
});
test('bounded reads reject overflow, truncation and bad PNG dimensions',async t=>{
 const bytes=new Uint8Array(60),entry=receipt('reference/input.bin',bytes);
 t.mock.method(globalThis,'fetch',async()=>new Response(bytes));
 await assert.rejects(readAsset('./assets/runtime/package/',{...entry,bytes:59}),/size|length/);
 await assert.rejects(readAsset('./assets/runtime/package/',{...entry,bytes:61}),/size|length/);
 await assert.rejects(readAsset('./assets/runtime/package/',{...entry,bytes:2**32}),/receipt|size/);
 await assert.rejects(readAsset('./assets/runtime/package/',{...entry,file:'reference/input.png',image_size:[2,3]}),/PNG/);
 assert.throws(()=>assetUrl('./assets/runtime/package/','../escape'),/path/);
 assert.throws(()=>checkImageSize({image_size:[2,3]},3,2),/decoded image dimensions/);
});
test('declared input overflow fails before reading and cleanup waits for physical cancellation',async t=>{
 const entry=receipt('reference/selected.bin',new Uint8Array(2));
 let finishCancel,settled=false;
 const cancelled=new Promise(resolve=>{finishCancel=resolve;});
 t.mock.method(globalThis,'fetch',async()=>new Response(new ReadableStream({
  start(controller){controller.enqueue(new Uint8Array(3));},
  cancel(){return cancelled;},
 })));
 const pending=readAsset('./assets/runtime/package/',entry).finally(()=>{settled=true;});
 const failure=assert.rejects(pending,/length/);
 await new Promise(resolve=>setImmediate(resolve));
 assert.equal(settled,false,'the caller must retain ownership while physical cancellation is pending');
 finishCancel();await failure;assert.equal(settled,true);
 t.mock.method(globalThis,'fetch',async()=>new Response(new Uint8Array(2),{headers:{'Content-Length':String(2**32)}}));
 await assert.rejects(readAsset('./assets/runtime/package/',entry),/declared size/);
});

test('large audio receipts keep their declared bound outside world staging',async t=>{
 const large={file:'reference/radio.wav',bytes:57418028,sha256:'a'.repeat(64)};
 configureAssetCatalog('./assets/runtime/package/',{files:[large],plans:[],dependency_staging_reserve:5242960});
 t.mock.method(globalThis,'fetch',async()=>new Response(new Uint8Array(2),{headers:{'Content-Length':'2'}}));
 await assert.rejects(readAsset('./assets/runtime/package/',large),/declared size differs/);
});

test('response failures retain ownership until physical cancellation completes',async t=>{
 const entry=receipt('reference/selected.bin',new Uint8Array(2));
 for(const [options,message] of [[{status:503},/HTTP 503/],[{headers:{'Content-Length':'3'}},/declared size/]]){
  let finishCancel,cancelStarted=false,settled=false;
  const cancelled=new Promise(resolve=>{finishCancel=resolve;});
  t.mock.method(globalThis,'fetch',async()=>new Response(new ReadableStream({
   start(controller){controller.enqueue(new Uint8Array(2));},
   cancel(){cancelStarted=true;return cancelled;},
  }),options));
  const pending=readAsset('./assets/runtime/package/',entry).finally(()=>{settled=true;});
  const failure=assert.rejects(pending,message);
  try{
   await new Promise(resolve=>setImmediate(resolve));
   assert.equal(cancelStarted,true,'failure must cancel the response body');
   assert.equal(settled,false,'ownership must remain until physical cancellation completes');
  }finally{finishCancel();await failure;}
  assert.equal(settled,true);
 }
});
