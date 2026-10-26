import test from 'node:test';
import assert from 'node:assert/strict';
import {installDrivingSettings} from '../../web/driving-settings.js';

const range=value=>({default:value,minimum:0,maximum:5});
const axis={enabled:true,invert:false,middle:null,dead:range(0.1),sens:range(1),max:range(1)};
const driving={mouse_drive:{sensitivity_x:{default:1,minimum:0.1,maximum:4},sensitivity_y:{default:2,minimum:0.5,maximum:3}},
 steering_wheel:{h_shifter:false,axes:[{...axis,middle:range(0)},axis,axis,axis,axis]}};
const wheelAxis={bound:null,enabled:true,invert:false,middle:0,dead:0.1,sens:1,max:1};
// The full Rust `DrivingSettings` JSON with the given mouse values.
const full=mouse=>({mouse,h_shifter:false,axes:Array.from({length:5},()=>({...wheelAxis})),buttons:Array(11).fill(null)});
const sent=mouse=>JSON.stringify(full(mouse));
function memory(initial){const items=new Map(initial===undefined?[]:[['the-long-slop.driving-settings',initial]]);return {items,storage:()=>({getItem:key=>items.get(key)??null,setItem:(key,value)=>items.set(key,value)})};}
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

test('steering wheel binds round trip through storage',()=>{
 const store=memory(),first=install(store.storage),{settings}=first.settings;first.settings.take();
 settings.h_shifter=true;settings.axes[0].bound={pad:1,axis:7};settings.axes[0].middle=0.5;settings.buttons[10]={pad:3,button:31};
 first.settings.changed();
 const expected=JSON.stringify(settings);
 assert.equal(first.settings.take(),expected);
 assert.equal(install(store.storage).settings.take(),expected);
});

test('invalid or blocked storage gives the export defaults',()=>{
 assert.equal(install(memory(sent([9,2])).storage).settings.take(),sent([1,2]));
 assert.equal(install(memory('not json').storage).settings.take(),sent([1,2]));
 const bad=[value=>{value.extra=1;},value=>{value.axes[1].bound={pad:4,axis:0};},value=>{value.axes[1].middle=0.5;},
  value=>{value.buttons[0]={pad:0,button:32};},value=>{value.buttons.pop();},value=>{value.axes[2].dead=9;}];
 for(const change of bad){const value=full([1,2]);change(value);assert.equal(install(memory(JSON.stringify(value)).storage).settings.take(),sent([1,2]));}
 assert.equal(install(()=>{throw new Error('SecurityError');}).settings.take(),sent([1,2]));
});
