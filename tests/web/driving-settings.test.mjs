import test from 'node:test';
import assert from 'node:assert/strict';
import {installDrivingSettings} from '../../web/driving-settings.js';

const mouse={sensitivity_x:{default:1,minimum:0.1,maximum:4},sensitivity_y:{default:2,minimum:0.5,maximum:3}};
function memory(initial){const items=new Map(initial===undefined?[]:[['the-long-slop.mouse-driving',initial]]);return {items,storage:()=>({getItem:key=>items.get(key)??null,setItem:(key,value)=>items.set(key,value)})};}
function install(storage){
 const inputs=[new EventTarget(),new EventTarget()],outputs=[{textContent:''},{textContent:''}];
 return {inputs,outputs,settings:installDrivingSettings(storage,inputs,outputs,mouse)};
}

test('settings are sent at start and after a change, and stored',()=>{
 const store=memory(),first=install(store.storage);
 assert.equal(first.settings.take(),'{"mouse":[1,2]}');assert.equal(first.settings.take(),undefined);
 first.inputs[1].value='2.5';first.inputs[1].dispatchEvent(new Event('input'));
 assert.equal(first.outputs[1].textContent,'2.50');
 assert.equal(first.settings.take(),'{"mouse":[1,2.5]}');
 assert.equal(install(store.storage).settings.take(),'{"mouse":[1,2.5]}');
});

test('invalid or blocked storage gives the export defaults',()=>{
 assert.equal(install(memory('{"mouse":[9,2]}').storage).settings.take(),'{"mouse":[1,2]}');
 assert.equal(install(memory('not json').storage).settings.take(),'{"mouse":[1,2]}');
 assert.equal(install(()=>{throw new Error('SecurityError');}).settings.take(),'{"mouse":[1,2]}');
});
