import test from 'node:test';
import assert from 'node:assert/strict';

test('startup shows string errors returned by WASM',async()=>{
 const nodes=new Map();
 const node=selector=>{
  if(!nodes.has(selector))nodes.set(selector,{textContent:'',hidden:true,addEventListener(){},removeAttribute(){}});
  return nodes.get(selector);
 };
 globalThis.window={addEventListener(){}};
 globalThis.document={
  querySelector:node,
  addEventListener(){},
  createElement:()=>({getContext:()=>({getExtension:()=>null})}),
 };
 globalThis.fetch=async()=>{throw 'Invalid reference field: manifest size';};
 await import('../../web/main.js');
 await new Promise(resolve=>setImmediate(resolve));
 assert.equal(node('#runtime-error').hidden,false);
 assert.equal(node('#runtime-error').textContent,
  'Cannot start the game. Invalid reference field: manifest size Reload to try again.');
});
