import test from 'node:test';
import assert from 'node:assert/strict';
import {createStartupTrace} from '../../web/startup-trace.js';

test('phase duration and failure snapshot stay fixed',()=>{
 let now=10;
 const trace=createStartupTrace(()=>now);
 trace.begin('wasm');
 now=35;trace.end('wasm');
 trace.fetched({path:'pkg/game.wasm',requiredBytes:7,receivedBytes:7,cache:'miss',downloadMs:4});
 trace.reconcile([{name:'http://localhost/pkg/game.wasm',transferSize:7,requestStart:30,responseEnd:35}]);
 trace.build({sourceBuildId:11140890});
 const snapshot=trace.snapshot();
 assert.equal(snapshot.phases.wasm.durationMs,25);
 assert.equal(snapshot.requiredBytes,7);
 assert.equal(snapshot.receivedBytes,7);
 assert.equal(snapshot.downloadMs,5);
 assert.equal(snapshot.cache.miss,1);
 assert.equal(snapshot.buildIdentity.sourceBuildId,11140890);
 trace.fail({message:'light admission failed',requiredLights:14});
 const frozenFailureSnapshot=trace.snapshot();
 now=80;trace.begin('generation');trace.fetched({path:'late',requiredBytes:1,receivedBytes:1});
 trace.reconcile([{name:'http://localhost/pkg/game.wasm',transferSize:0,requestStart:50,responseEnd:80}]);
 assert.deepEqual(trace.snapshot(),frozenFailureSnapshot);
});

test('finish requires two presented frames and input readiness',()=>{
 const trace=createStartupTrace(()=>10);
 assert.throws(()=>trace.finish({presentedFrames:0,inputReady:false}));
 assert.throws(()=>trace.finish({presentedFrames:1,inputReady:true}));
 assert.throws(()=>trace.finish({presentedFrames:2,inputReady:false}));
 trace.finish({presentedFrames:2,inputReady:true});
 assert.equal(trace.snapshot().completed,true);
});
