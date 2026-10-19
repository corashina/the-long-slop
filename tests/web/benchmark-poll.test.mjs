import {test} from 'node:test';
import assert from 'node:assert/strict';
import {poll} from '../../scripts/benchmark-poll.mjs';

test('async false does not complete polling before the condition becomes true',async()=>{
 let calls=0;
 await poll(async()=>++calls===2,1000);
 assert.equal(calls,2);
 await assert.rejects(poll(async()=>false,20),/timed out/);
});

test('a stalled async condition cannot bypass the deadline',async()=>{
 await assert.rejects(poll(()=>new Promise(()=>{}),20),/timed out/);
});
