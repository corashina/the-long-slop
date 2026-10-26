import test from 'node:test';
import assert from 'node:assert/strict';
import {movedAxis,pressedButton} from '../../web/steering-wheel.js';

test('axis bind takes the first axis past 0.25 from rest',()=>{
 const rest=new Float32Array(32);rest[3]=-1;
 const axes=Float32Array.from(rest);axes[3]=-0.8;axes[9]=0.2;
 assert.equal(movedAxis(axes,rest),null);
 axes[9]=0.3;axes[20]=-0.9;
 assert.deepEqual(movedAxis(axes,rest),{pad:1,axis:1});
});

test('button bind takes a newly pressed button only',()=>{
 const held=Uint32Array.of(1,0,0,0);
 assert.equal(pressedButton(Uint32Array.of(1,0,0,0),held),null);
 assert.equal(pressedButton(Uint32Array.of(0,0,0,0),held),null);
 assert.deepEqual(pressedButton(Uint32Array.of(1,0,2**31+8,0),held),{pad:0,button:0});
 held.fill(0);held[2]=8;
 assert.deepEqual(pressedButton(Uint32Array.of(0,0,2**31+8,0),held),{pad:2,button:31});
});
