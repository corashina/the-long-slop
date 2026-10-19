import test from 'node:test';
import assert from 'node:assert/strict';
import {createTerrain} from '../../web/terrain.js';

function fakeWorker(){
 const listeners={},posted=[];
 return {posted,addEventListener:(type,fn)=>{listeners[type]=fn;},postMessage:(data,transfer)=>posted.push({data,transfer}),
  reply:data=>listeners.message({data}),fail:message=>listeners.error({message})};
}
const result=id=>({id,vertices:new Float32Array([1,2,3,4,5,6]),normals:new Float32Array(6),tangents:new Float32Array(8),
 uv:new Float32Array(4),colors:new Float32Array(8),triangles:new Uint32Array([0,1,1])});

test('one job is in flight, its result is read once, and a discarded result frees the worker',()=>{
 const worker=fakeWorker(),terrain=createTerrain(worker),memory=new Uint8Array([7,8,9]);
 assert.equal(terrain.poll(),-1);
 const id=terrain.submit(memory.subarray(1));
 memory[1]=0;
 assert.deepEqual([...worker.posted[0].data.input],[8,9]);
 assert.equal(worker.posted[0].transfer[0],worker.posted[0].data.input.buffer);
 assert.equal(terrain.poll(),-2);
 assert.throws(()=>terrain.submit(memory));
 worker.reply(result(id));
 assert.equal(terrain.poll(),id);
 assert.equal(terrain.vertexCount(),2);
 assert.equal(terrain.triangleCount(),1);
 const vertices=new Float32Array(6),triangles=new Uint32Array(3),f=n=>new Float32Array(n);
 terrain.read(vertices,f(6),f(8),f(4),f(8),triangles);
 assert.deepEqual([...vertices],[1,2,3,4,5,6]);
 assert.deepEqual([...triangles],[0,1,1]);
 assert.equal(terrain.poll(),-1);
 const second=terrain.submit(memory);
 assert.notEqual(second,id);
 worker.reply(result(second));
 terrain.discard();
 assert.equal(terrain.poll(),-1);
});

test('worker failures reach the next poll',()=>{
 const worker=fakeWorker(),terrain=createTerrain(worker);
 const id=terrain.submit(new Uint8Array(1));
 worker.reply({id,error:'Terrain job input is truncated'});
 assert.throws(()=>terrain.poll(),/truncated/);
 const other=fakeWorker(),stopped=createTerrain(other);
 other.fail('Cannot load terrain code');
 assert.throws(()=>stopped.poll(),/Cannot load/);
});
