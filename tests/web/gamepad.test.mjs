import test from 'node:test';
import assert from 'node:assert/strict';
import {createGamepad} from '../../web/gamepad.js';

const button=pressed=>({pressed});
test('pads pack into reused arrays and missing pads read zero',()=>{
 const buttons=Array.from({length:33},(_,i)=>button(i===0||i===31||i===32));
 let pads=[null,{connected:true,axes:[0.5,-1,0,0,0,0,0,0,0.75],buttons}];
 const gamepad=createGamepad(()=>pads),{axes,buttons:bits}=gamepad;
 gamepad.poll();
 assert.equal(axes[8],0.5);assert.equal(axes[9],-1);assert.equal(axes[16],0);
 assert.deepEqual([...bits],[0,2**31+1,0,0]);
 pads=[];gamepad.poll();
 assert.equal(gamepad.axes,axes);assert.equal(axes[8],0);assert.deepEqual([...bits],[0,0,0,0]);
 const blocked=createGamepad(()=>{throw new Error('SecurityError');});blocked.poll();
 assert.deepEqual([...blocked.buttons],[0,0,0,0]);
});
