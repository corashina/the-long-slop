import test from 'node:test';
import assert from 'node:assert/strict';
import {installDrawDistance} from '../../web/draw-distance.js';

const range={minimum:500,maximum:25000,fallback:2500};
function memory(initial){const items=new Map(initial===undefined?[]:[['the-long-slop.draw-distance',initial]]);return {items,storage:()=>({getItem:key=>items.get(key)??null,setItem:(key,value)=>items.set(key,value),removeItem:key=>items.delete(key)})};}
function install(storage){
 const input=new EventTarget(),output={textContent:''};input.disabled=false;
 return {input,output,slider:installDrawDistance(storage,input,output,range)};
}

test('the slider stores an accepted value, restores it, and clears it',()=>{
 const store=memory(),first=install(store.storage);
 assert.equal(first.slider.value(),2500);assert.equal(first.output.textContent,'2500 m');
 assert.deepEqual([first.input.min,first.input.max,first.input.step],['500','25000','100']);
 first.input.value='10040';first.input.dispatchEvent(new Event('input'));first.input.dispatchEvent(new Event('change'));
 assert.equal(first.slider.value(),10000);assert.equal(first.output.textContent,'10000 m');
 assert.equal(store.items.size,0);
 first.slider.store();
 assert.equal(store.items.get('the-long-slop.draw-distance'),'10000');
 const next=install(store.storage);
 assert.equal(next.slider.value(),10000);assert.equal(next.input.value,'10000');
 next.slider.clear();
 assert.equal(install(store.storage).slider.value(),2500);
});

test('blocked storage and invalid values give the export default',()=>{
 const blocked=install(()=>{throw new Error('SecurityError');});
 assert.equal(blocked.slider.value(),2500);
 blocked.input.value='600';blocked.input.dispatchEvent(new Event('input'));blocked.slider.store();blocked.slider.clear();
 assert.equal(blocked.slider.value(),600);
 for(const value of ['abc','','Infinity','400','25100'])assert.equal(install(memory(value).storage).slider.value(),2500,value);
});

test('lock toggles disabled',()=>{
 const {input,slider}=install(memory().storage);
 slider.lock(true);assert.equal(input.disabled,true);
 slider.lock(false);assert.equal(input.disabled,false);
});
