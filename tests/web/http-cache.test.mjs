import test from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {mkdir,writeFile,unlink,rmdir,rename} from 'node:fs/promises';
import {randomUUID} from 'node:crypto';

test('immutable packages cache; mutable code revalidates with ETags',async()=>{
 const id='cache-test-'+randomUUID(),folder='web/assets/runtime/'+id,nextFolder=folder+'-new';
 let path=folder+'/receipt.txt',moved=false;
 await mkdir(folder,{recursive:true});await writeFile(path,'owned cache fixture');
 const server=spawn(process.execPath,['scripts/serve.mjs'],{env:{...process.env,PORT:'0'},stdio:['ignore','pipe','pipe']});
 try{
  const [line]=await Promise.race([once(server.stdout,'data'),once(server,'exit').then(()=>{throw Error('Child server exited');})]);
  const base=line.toString().match(/http:\/\/127\.0\.0\.1:\d+/)[0];
  const response=await fetch(base+'/assets/runtime/'+id+'/receipt.txt');
  assert.equal(response.status,200);assert.equal(await response.text(),'owned cache fixture');
  assert.equal(response.headers.get('cache-control'),'public, max-age=31536000, immutable');
  for(const path of ['/','/main.js']){
   const first=await fetch(base+path);await first.arrayBuffer();
   assert.equal(first.headers.get('cache-control'),'no-cache');
   assert.ok(first.headers.get('etag'));
   const next=await fetch(base+path,{headers:{'If-None-Match':first.headers.get('etag')}});
   assert.equal(next.status,304);assert.equal((await next.arrayBuffer()).byteLength,0);
  }
  await mkdir(nextFolder);await rename(path,nextFolder+'/receipt.txt');path=nextFolder+'/receipt.txt';moved=true;
  await writeFile(path,'new package fixture');
  const changed=await fetch(base+'/assets/runtime/'+id+'-new/receipt.txt');
  assert.equal(changed.status,200);assert.equal(await changed.text(),'new package fixture');
  assert.notEqual(changed.url,response.url);
 }finally{
  if(server.exitCode===null){const ended=once(server,'exit');server.kill();await ended;}
  await unlink(path);if(moved)await rmdir(nextFolder);await rmdir(folder);
 }
});
