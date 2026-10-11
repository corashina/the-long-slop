import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { resolve, basename } from 'node:path';
test('GET and HEAD preserve media types and lengths', async () => {
 const folder=await mkdtemp(resolve('web/server-test-'));
 const server=spawn(process.execPath,['scripts/serve.mjs'],{env:{...process.env,PORT:'4199'}});
 try {
  await once(server.stdout,'data');
  for(const [ext,type] of [['wav','audio/wav'],['ttf','font/ttf'],['otf','font/otf'],['glb','model/gltf-binary']]) {
   await writeFile(resolve(folder,`fixture.${ext}`),Buffer.from([0,1,2,3,4]));
   for(const method of ['GET','HEAD']) {
    const response=await fetch(`http://127.0.0.1:4199/${basename(folder)}/fixture.${ext}`,{method});
    assert.equal(response.status,200); assert.equal(response.headers.get('content-type'),type);
    assert.equal(response.headers.get('content-length'),'5');
    assert.equal((await response.arrayBuffer()).byteLength,method==='HEAD'?0:5);
   }
  }
 } finally {server.kill();await once(server,'exit');await rm(folder,{recursive:true});}
});
