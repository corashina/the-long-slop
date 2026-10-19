import test from 'node:test';
import assert from 'node:assert/strict';
import {createCpuProfile,workerClockHandshake,workerDeliveryInterval} from '../../web/cpu-profile.js';

test('disabled_profile_has_no_clock_reads',()=>{
 let calls=0,throwClock=false;
 const p=createCpuProfile(()=>{if(throwClock)throw Error('disabled clock');calls++;return calls;},1000);
 calls=0;throwClock=true;
 p.begin('physics');p.end('physics');p.input(1,0,true);p.consume(1,2);p.submitFrame(3);
 const clockCallsAfterReset=calls;assert.equal(clockCallsAfterReset,0);
 assert.equal(p.snapshot().inputs.length,0);
});

test('input_requires_a_fixed_step',()=>{
 let clock=10;const p=createCpuProfile(()=>clock,1000);p.enable(true);
 p.input(1,5,true);clock=20;p.submitFrame(1);
 assert.equal(p.snapshot().inputs[0].eventToConsumeMs,null);
 clock=25;p.consume(1,2);clock=30;p.submitFrame(3);p.consume(1,3);p.submitFrame(4);
 assert.deepEqual(p.snapshot().inputs[0],{sequence:1,eventAt:5,trusted:true,step:2,token:3,eventToConsumeMs:20,consumeToSubmitMs:5});
});

test('storage_is_bounded_and_pending_input_can_be_cleared',()=>{
 const p=createCpuProfile(()=>10,1000);p.enable(true);
 for(let i=1;i<=4100;i++)p.input(i,0,false);
 assert.equal(p.snapshot().inputs.length,4096);assert.equal(p.snapshot().dropped.inputs,4);
 p.clearInput();p.consume(4100,2);p.submitFrame(3);
 assert.ok(p.snapshot().inputs.every(row=>row.step===null));
 for(let i=0;i<4100;i++)p.worker({id:i});
 assert.equal(p.snapshot().workers.length,4096);assert.equal(p.snapshot().dropped.workers,4);
});

test('clock_handshake_keeps_context_origins_separate',()=>{
 const clock=workerClockHandshake({windowOrigin:1000,windowSent:100,windowReceived:120,workerOrigin:900,workerReceived:205,workerSent:210});
 assert.equal(clock.roundTripMs,15);
 assert.equal(workerDeliveryInterval(clock,{workerSent:215,windowReceived:130}),15);
 assert.throws(()=>workerDeliveryInterval(clock,{workerSent:500,windowReceived:130}),/negative/);
});

test('stage_statistics_and_session_clear_keep_their_measurement_boundaries',()=>{
 let clock=0;const p=createCpuProfile(()=>clock,1000);p.enable(true);
 for(const duration of [1,2,3,4,10]){p.begin('physics');clock+=duration;p.end('physics');}
 assert.deepEqual(p.snapshot().stages.physics,{count:5,meanMs:4,p95Ms:10,p99Ms:10,maxMs:10,dropped:0});
 p.input(1,0,true);p.consume(1,1);p.clearInput();p.submitFrame(1);
 assert.equal(p.snapshot().inputs[0].token,null);
 p.input(2,clock,true);p.consume(2,2);p.submitFrame(1);
 // The old token already submitted before event 2 was consumed.
 assert.equal(p.snapshot().inputs[1].token,null);p.submitFrame(2);assert.equal(p.snapshot().inputs[1].token,2);
});
