import test from 'node:test';
import assert from 'node:assert/strict';
import {createLoadingGuard} from '../../web/loading-guard.js';
function fixture(){
 let now=0,next=0,revision=0;const timers=new Map(),failures=[];
 const clock={now:()=>now,setTimeout(callback,delay){const id=++next;timers.set(id,{at:now+delay,callback});return id;},clearTimeout(id){timers.delete(id);}};
 const guard=createLoadingGuard({clock,progress:()=>revision,fail:message=>failures.push(message)});
 return {guard,failures,progress(){revision++;},advance(ms){const end=now+ms;while(true){const entry=[...timers].sort((a,b)=>a[1].at-b[1].at)[0];if(!entry||entry[1].at>end)break;now=entry[1].at;timers.delete(entry[0]);entry[1].callback();}now=end;}};
}
test('new readiness work renews the 120 second stall deadline',()=>{
 const f=fixture();f.guard.start();f.advance(119000);f.progress();f.advance(119000);assert.equal(f.failures.length,0);f.advance(1000);assert.equal(f.failures.length,0);f.advance(1000);assert.deepEqual(f.failures,['Loading made no progress for 120 seconds.']);
});
test('stalled loading fails once at 120 seconds',()=>{
 const f=fixture();f.guard.start();f.advance(119999);assert.equal(f.failures.length,0);f.advance(1);assert.equal(f.failures.length,1);f.advance(240000);assert.equal(f.failures.length,1);
});
test('repeated frames without a new readiness revision do not renew the guard',()=>{
 const f=fixture();f.guard.start();for(let frame=0;frame<7200;frame++)f.advance(1000/60);f.advance(1);assert.equal(f.failures.length,1);
});
test('Ready stops the guard and a new Load starts a fresh deadline',()=>{
 const f=fixture();f.guard.start();f.advance(119000);f.guard.stop();f.advance(240000);assert.equal(f.failures.length,0);f.guard.start();f.advance(119000);assert.equal(f.failures.length,0);f.progress();f.advance(1000);f.guard.stop();f.advance(240000);assert.equal(f.failures.length,0);f.guard.start();f.advance(120000);assert.equal(f.failures.length,1);
});

test('default clock calls browser timers with a valid receiver',()=>{
 const set=globalThis.setTimeout,clear=globalThis.clearTimeout;let canceled=false;
 try{
  globalThis.setTimeout=function(){assert(this===undefined||this===globalThis);return 7;};
  globalThis.clearTimeout=function(id){assert(this===undefined||this===globalThis);assert.equal(id,7);canceled=true;};
  const guard=createLoadingGuard({progress:()=>0,fail:()=>{}});guard.start();guard.stop();assert(canceled);
 }finally{globalThis.setTimeout=set;globalThis.clearTimeout=clear;}
});
