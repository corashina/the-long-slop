import test from 'node:test';
import assert from 'node:assert/strict';
import {installDrivingSettings} from '../../web/driving-settings.js';

const range=value=>({default:value,minimum:0,maximum:5});
const axis={enabled:true,invert:false,middle:null,dead:range(0.1),sens:range(1),max:range(1)};
const driving={mouse_drive:{sensitivity_x:{default:1,minimum:0.1,maximum:4},sensitivity_y:{default:2,minimum:0.5,maximum:3}},
 steering_wheel:{h_shifter:false,axes:[{...axis,middle:range(0)},axis,axis,axis,axis]}};
const wheelAxis={bound:null,enabled:true,invert:false,middle:0,dead:0.1,sens:1,max:1};
// The full Rust `DrivingSettings` JSON with the given mouse values.
const sent=mouse=>JSON.stringify({mouse,h_shifter:false,axes:Array(5).fill(wheelAxis),buttons:Array(11).fill(null)});
function memory(initial){const items=new Map(initial===undefined?[]:[['the-long-slop.mouse-driving',initial]]);return {items,storage:()=>({getItem:key=>items.get(key)??null,setItem:(key,value)=>items.set(key,value)})};}
function install(storage){
 const inputs=[new EventTarget(),new EventTarget()],outputs=[{textContent:''},{textContent:''}];
 return {inputs,outputs,settings:installDrivingSettings(storage,inputs,outputs,driving)};
}

test('settings are sent at start and after a change, and stored',()=>{
 const store=memory(),first=install(store.storage);
 assert.equal(first.settings.take(),sent([1,2]));assert.equal(first.settings.take(),undefined);
 first.inputs[1].value='2.5';first.inputs[1].dispatchEvent(new Event('input'));
 assert.equal(first.outputs[1].textContent,'2.50');
 assert.equal(first.settings.take(),sent([1,2.5]));
 assert.equal(install(store.storage).settings.take(),sent([1,2.5]));
});

test('invalid or blocked storage gives the export defaults',()=>{
 assert.equal(install(memory(sent([9,2])).storage).settings.take(),sent([1,2]));
 assert.equal(install(memory('not json').storage).settings.take(),sent([1,2]));
 assert.equal(install(()=>{throw new Error('SecurityError');}).settings.take(),sent([1,2]));
});
