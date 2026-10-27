import test from 'node:test';
import assert from 'node:assert/strict';
import {installSunglassesEffects,readSunglassesEffects} from '../../web/sunglasses.js';

function memory(initial){const items=new Map(initial===undefined?[]:[['the-long-slop.sunglasses-effects',initial]]);return {items,storage:()=>({getItem:key=>items.get(key)??null,setItem:(key,value)=>items.set(key,value)})};}

test('the setting is on by default and with bad or blocked storage',()=>{
 assert.equal(readSunglassesEffects(memory().storage),true);
 assert.equal(readSunglassesEffects(memory('bad').storage),true);
 assert.equal(readSunglassesEffects(()=>{throw new Error('SecurityError');}),true);
 assert.equal(readSunglassesEffects(memory('off').storage),false);
});

test('a change is stored and sent to Rust once',()=>{
 const store=memory(),target={},input=new EventTarget();
 installSunglassesEffects(target,store.storage,input);
 assert.equal(input.checked,true);
 assert.equal(target.gameSunglassesEffects(),true);
 assert.equal(target.gameSunglassesEffects(),undefined);
 input.checked=false;input.dispatchEvent(new Event('change'));
 assert.equal(store.items.get('the-long-slop.sunglasses-effects'),'off');
 assert.equal(target.gameSunglassesEffects(),false);
 assert.equal(target.gameSunglassesEffects(),undefined);
 const next={},reloaded=new EventTarget();
 installSunglassesEffects(next,store.storage,reloaded);
 assert.equal(reloaded.checked,false);
 assert.equal(next.gameSunglassesEffects(),false);
});
