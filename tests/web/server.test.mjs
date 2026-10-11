import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { resolve, basename } from 'node:path';
test('GET and HEAD preserve media types and lengths', async () => {
 const folder=await mkdtemp(resolve('web/server-test-'));
 const server=spawn(process.execPath,['scripts/serve.mjs'],{env:{...process.env,PORT:'0'}});
 try {
  const [address]=await Promise.race([once(server.stdout,'data'),once(server,'exit').then(()=>{throw Error('Test server exited before listening.');})]);
  const url=address.toString().match(/http:\/\/127\.0\.0\.1:\d+/)[0];
  for(const [ext,type] of [['wav','audio/wav'],['ttf','font/ttf'],['otf','font/otf'],['glb','model/gltf-binary']]) {
   await writeFile(resolve(folder,`fixture.${ext}`),Buffer.from([0,1,2,3,4]));
   for(const method of ['GET','HEAD']) {
    const response=await fetch(`${url}/${basename(folder)}/fixture.${ext}`,{method});
    assert.equal(response.status,200); assert.equal(response.headers.get('content-type'),type);
    assert.equal(response.headers.get('content-length'),'5');
    assert.equal((await response.arrayBuffer()).byteLength,method==='HEAD'?0:5);
   }
  }
 } finally {if(server.exitCode===null&&server.signalCode===null){server.kill();await once(server,'exit');}await rm(folder,{recursive:true});}
});
