import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve} from 'node:path';
import {registerHooks} from 'node:module';
import {contentIndexSampleFailures,assertCpuProfileMode,measureContentIndex} from '../../scripts/measure-content-index.mjs';
import {createCpuProfile} from '../../web/cpu-profile.js';
import {createContentIndexBenchmarkServer} from '../../scripts/serve-content-index-benchmark.mjs';
import {summarizeWorkerComparison,attachBrowserTrace} from '../../scripts/summarize-streaming-profile.mjs';

function receipt(){const identity={web:{'main.js':'a'.repeat(64)},scripts:{'measure-content-index.mjs':'b'.repeat(64)},packages:{'main.js':'c'.repeat(64),'worker.wasm':'d'.repeat(64)}};const configuration={identity,commit:'a'.repeat(40),manifestSha256:'f'.repeat(64),buildProfile:'release'};return {...sample(),schemaVersion:2,valid:true,failures:[],instrument:false,profileCpu:false,profileStreaming:false,performanceSample:true,cacheState:'warm',instrumentationMode:'none',configuration,server:configuration,canvas:[1280,720],dpr:1,gpu:'fixture',browser:'fixture',route:'fixed-route',seed:1,intervals:{count:59,meanMs:16,p95Ms:18,p99Ms:20,maxMs:30}};}
test('receipt_rejects_mixed_packages_and_invalid_samples',()=>{
 const a=receipt(),b=receipt(),c=receipt();c.intervals.p95Ms=19;
 assert.equal(summarizeWorkerComparison(a,b,c).metrics['frame.p95Ms'].variation,1);
 c.configuration.identity.packages['worker.wasm']='changed';
 assert.throws(()=>summarizeWorkerComparison(a,b,c),/identity/);
 for(const [field,value] of [['hiddenDuringSample',true],['unfocusedDuringSample',true],['pausedFrames',1],['submissionDelta',0]]){
  assert.throws(()=>summarizeWorkerComparison(a,{...receipt(),[field]:value},receipt()),/invalid sample/);
 }
 assert.throws(()=>summarizeWorkerComparison(a,{...receipt(),scenario:'driving',travelDeltaM:0},receipt()),/invalid sample/);
 const missing=receipt();delete missing.intervals.p99Ms;assert.throws(()=>summarizeWorkerComparison(a,missing,receipt()),/metric/);
 const missingMetadata=receipt();delete missingMetadata.seed;assert.throws(()=>summarizeWorkerComparison(a,missingMetadata,receipt()),/metadata/);
});

test('invalid worker timing cannot qualify a saved comparison',()=>{
 const invalid={...receipt(),profileCpu:true,performanceSample:false,instrumentationMode:'cpu',cpuProfile:{browser:{stages:{},inputLatencies:{},workers:[{kind:'terrain',id:1,timing:{valid:false,submitDelivery:{valid:false,reason:'below-zero-within-arithmetic-uncertainty',rawMs:-0.00005},returnDelivery:{valid:true}},submitDeliveryMs:null,returnDeliveryMs:3,totalJobMs:12,worker:{evaluateWithInputCopyMs:2,extractionMs:-1}}]},rust:{stages:[],measurements:[],streaming:{stages:[],measurements:[]}}}};
 const summary=summarizeWorkerComparison(invalid,invalid,invalid);
 assert.equal(summary.workerCosts[0].terrain['main.submitDeliveryMs'],undefined);
 assert.equal(summary.workerCosts[0].terrain['worker.extractionMs'],undefined);
 assert.equal(summary.workerCosts[0].terrain['main.totalJobMs'].meanMs,12);
 assert.equal(summary.workerCosts[0].terrain['worker.evaluateWithInputCopyMs'].meanMs,2);
 assert.equal(summary.workerCosts[0].terrain['main.returnDeliveryMs'].meanMs,3);
 assert.equal(summary.workerTimingExcluded[0][0].reason,'below-zero-within-arithmetic-uncertainty');
 assert.equal(summary.workerRecords[0][0].timing.submitDelivery.rawMs,-0.00005);
});

test('saved browser traces attach to the same receipt with their content hash',async t=>{
 const root=await mkdtemp(resolve(tmpdir(),'cpu-trace-'));t.after(()=>rm(root,{recursive:true,force:true}));
 const receiptPath=resolve(root,'receipt.json'),tracePath=resolve(root,'trace.json');
 await writeFile(receiptPath,JSON.stringify(receipt()));await writeFile(tracePath,'{"traceEvents":[]}');
 const trace=await attachBrowserTrace(receiptPath,tracePath);assert.match(trace.sha256,/^[a-f0-9]{64}$/);assert.equal(trace.coverageVerified,false);
 assert.deepEqual(JSON.parse(await readFile(receiptPath,'utf8')).browserTrace,trace);
});

function sample(){return {scenario:'garage',auditBefore:{state:'Ready'},auditAfter:{state:'Ready'},frames:60,submissionDelta:60,
 hiddenDuringSample:false,unfocusedDuringSample:false,notStartedFrames:0,pausedFrames:0,unseatedFrames:0,settled:true,travelDeltaM:10,
 stateBefore:{started:true,paused:false,totalSteps:10,seated:true},stateAfter:{started:true,paused:false,totalSteps:70,seated:true}};}

test('sample validity requires submitted frames, active physics, and a visible focused game',()=>{
 assert.deepEqual(contentIndexSampleFailures(sample()),[]);
 for(const [field,value,failure] of [['submissionDelta',59,'submitted-frame-mismatch'],['hiddenDuringSample',true,'hidden-tab'],
  ['unfocusedDuringSample',true,'unfocused-tab'],['pausedFrames',1,'game-paused'],['notStartedFrames',1,'game-not-started']]){
  assert.ok(contentIndexSampleFailures({...sample(),[field]:value}).includes(failure));
 }
 const stopped=sample();stopped.stateAfter.totalSteps=10;
 assert.ok(contentIndexSampleFailures(stopped).includes('physics-did-not-step'));
});

test('driving permits pending work and requires seated movement',()=>{
 const driving={...sample(),scenario:'driving',settled:false};
 assert.deepEqual(contentIndexSampleFailures(driving),[]);
 assert.ok(contentIndexSampleFailures({...driving,travelDeltaM:0}).includes('driving-not-observed'));
 assert.ok(contentIndexSampleFailures({...driving,unseatedFrames:1}).includes('driving-not-observed'));
 assert.ok(contentIndexSampleFailures({...driving,scenario:'garage'}).includes('garage-work-not-settled'));
});

test('scene scale receipts reject resize, scale changes, and wrong target extents',()=>{
 const scaled={...sample(),sceneResolution:{scale:.75,native:[1280,720],changedDuringSample:false},
  auditBefore:{state:'Ready',graph:{presented:true,extent:[960,540]}},auditAfter:{state:'Ready',graph:{presented:true,extent:[960,540]}}};
 assert.deepEqual(contentIndexSampleFailures(scaled),[]);
 assert.ok(contentIndexSampleFailures({...scaled,sceneResolution:{...scaled.sceneResolution,changedDuringSample:true}}).includes('scene-resolution-mismatch'));
 assert.ok(contentIndexSampleFailures({...scaled,auditAfter:{state:'Ready',graph:{presented:true,extent:[1280,720]}}}).includes('scene-resolution-mismatch'));
 assert.ok(contentIndexSampleFailures({...scaled,auditAfter:{state:'Ready',graph:{presented:false,extent:[960,540]}}}).includes('scene-resolution-mismatch'));
});

test('startup and save-load receipts require their own scenario evidence',()=>{
 const startup={...sample(),scenario:'startup-cold',cacheState:'cold',profileCpu:false,startupTrace:{completed:true,inputReady:true,failure:null,hiddenDuringStartup:false,unfocusedDuringStartup:false,profileCpu:false}};
 startup.stateBefore.started=false;startup.stateAfter.started=false;
 assert.deepEqual(contentIndexSampleFailures(startup),[]);
 assert.ok(contentIndexSampleFailures({...startup,cacheState:'warm'}).includes('startup-cache-state-mismatch'));
 assert.ok(contentIndexSampleFailures({...startup,startupTrace:{completed:false}}).includes('startup-not-complete'));
 const save={...sample(),scenario:'save-load',pausedFrames:2,saveOperations:[{kind:'Capture'},{kind:'Prepare'},{kind:'Commit'}]};
 assert.deepEqual(contentIndexSampleFailures(save),[]);
 assert.ok(contentIndexSampleFailures({...save,saveOperations:[]}).includes('save-load-not-observed'));
});

test('uninstrumented sampling rejects active startup CPU, Rust, terrain, and render instrumentation',()=>{
 const profile=createCpuProfile(()=>10,1000);profile.enable(true);
 const inactive={browserEnabled:false,rustEnabled:false,streamingEnabled:false,terrain:{jobActive:false,handshakeActive:false},road:{jobActive:false,handshakeActive:false,publicationActive:false},grass:{jobActive:false,handshakeActive:false,publicationActive:false},startupRenderTraceActive:false};
 assert.throws(()=>assertCpuProfileMode(false,false,{...inactive,browserEnabled:profile.enabled}),/profiling is active/);
 profile.enable(false);
 assert.doesNotThrow(()=>assertCpuProfileMode(false,false,{...inactive,browserEnabled:profile.enabled}));
 for(const active of [{rustEnabled:true},{streamingEnabled:true},{terrain:{jobActive:true,handshakeActive:false}},{terrain:{jobActive:false,handshakeActive:true}},{startupRenderTraceActive:true}])assert.throws(()=>assertCpuProfileMode(false,false,{...inactive,...active}),/profiling is active/);
 assert.doesNotThrow(()=>assertCpuProfileMode(false,true,{...inactive,streamingEnabled:true}));
});

test('the real sampler rejects a false-profile request before measuring or writing a receipt',async t=>{
 const previous={window:globalThis.window,document:globalThis.document,fetch:globalThis.fetch,fixture:globalThis.cpuReceiptFixture};
 t.after(()=>{globalThis.window=previous.window;globalThis.document=previous.document;globalThis.fetch=previous.fetch;globalThis.cpuReceiptFixture=previous.fixture;});
 const wasm='data:text/javascript,'+encodeURIComponent('export const cpu_profile_audit=()=>JSON.stringify({enabled:globalThis.cpuReceiptFixture.rustEnabled}); export const streaming_profile_audit=()=>JSON.stringify({enabled:globalThis.cpuReceiptFixture.streamingEnabled});');
 const hooks=registerHooks({resolve(specifier,context,next){return specifier==='/pkg/the_long_slop.js'?{url:wasm,shortCircuit:true}:next(specifier,context);}});t.after(()=>hooks.deregister());
 const profile=createCpuProfile(()=>10,1000);profile.enable(true);
 let measurements=0,writes=0;
 globalThis.cpuReceiptFixture={rustEnabled:true,streamingEnabled:true};
 globalThis.window={gameCpuInstrumentationState:()=>({browserEnabled:profile.enabled,terrain:{jobActive:false,handshakeActive:false},road:{jobActive:false,handshakeActive:false,publicationActive:false},grass:{jobActive:false,handshakeActive:false,publicationActive:false},startupRenderTraceActive:false}),gamePerformanceFrame(){},measureWorld(){measurements++;throw Error('unexpected measurement');}};
 globalThis.document={querySelector:()=>({getContext:()=>({})})};
 globalThis.fetch=async path=>{if(path!=='/__benchmark-config')writes++;return {ok:true,json:async()=>({})};};
 await assert.rejects(measureContentIndex({profileCpu:false}),/profiling is active/);
 profile.enable(false);globalThis.cpuReceiptFixture.streamingEnabled=false;
 await assert.rejects(measureContentIndex({profileCpu:false}),/profiling is active/);
 assert.equal(measurements,0);assert.equal(writes,0);
});

test('startup validity uses stored startup visibility and focus after the page becomes visible and focused',()=>{
 const startup={...sample(),scenario:'startup-warm',cacheState:'warm',profileCpu:false,startupTrace:{completed:true,inputReady:true,failure:null,hiddenDuringStartup:false,unfocusedDuringStartup:false,profileCpu:false}};
 assert.deepEqual(contentIndexSampleFailures(startup),[]);
 for(const [key,failure] of [['hiddenDuringStartup','startup-hidden-tab'],['unfocusedDuringStartup','startup-unfocused-tab']])assert.ok(contentIndexSampleFailures({...startup,startupTrace:{...startup.startupTrace,[key]:true}}).includes(failure));
 assert.ok(contentIndexSampleFailures({...startup,startupTrace:{...startup.startupTrace,profileCpu:true}}).includes('startup-instrumentation-mismatch'));
 const missing={...startup,startupTrace:{completed:true,inputReady:true,failure:null}};
 assert.ok(contentIndexSampleFailures(missing).includes('startup-environment-missing'));
});

test('server selects frozen inputs, confines paths, and preserves separate receipts',async t=>{
 const root=await mkdtemp(resolve(tmpdir(),'content-index-benchmark-'));
 t.after(()=>rm(root,{recursive:true,force:true}));
 const webRoot=resolve(root,'web'),scriptsRoot=resolve(root,'scripts'),assetRoot=resolve(root,'assets'),packageRoot=resolve(root,'pkg'),resultsRoot=resolve(root,'results'),commit='a'.repeat(40);
 await Promise.all([mkdir(webRoot),mkdir(scriptsRoot),mkdir(resolve(assetRoot,'reference'),{recursive:true}),mkdir(packageRoot)]);
 await Promise.all([writeFile(resolve(webRoot,'index.html'),'game'),writeFile(resolve(assetRoot,'reference/manifest.json'),'{}'),
  writeFile(resolve(packageRoot,'the_long_slop.js'),'frozen-package'),writeFile(resolve(packageRoot,'the_long_slop_bg.wasm'),'wasm'),
  writeFile(resolve(root,'private.txt'),'private'),writeFile(resolve(webRoot,'main.js'),'frozen-main'),writeFile(resolve(scriptsRoot,'measure-content-index.mjs'),'frozen measureContentIndex'),writeFile(resolve(packageRoot,'terrain_worker.js'),'frozen-worker'),writeFile(resolve(packageRoot,'terrain_worker_bg.wasm'),'worker-wasm')]);
 await assert.rejects(createContentIndexBenchmarkServer({webRoot,scriptsRoot,commit,assetRoot,packageRoot,resultsRoot:resolve(webRoot,'receipts'),variant:'candidate'}),/outside served directories/);
 const server=await createContentIndexBenchmarkServer({webRoot,scriptsRoot,commit,assetRoot,packageRoot,resultsRoot,variant:'baseline-recheck'});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 t.after(()=>new Promise((resolve,reject)=>{server.closeAllConnections();server.close(error=>error?reject(error):resolve());}));
 const base=`http://127.0.0.1:${server.address().port}`;
 assert.equal(await(await fetch(base+'/pkg/the_long_slop.js')).text(),'frozen-package');
 assert.equal(await(await fetch(base+'/assets/reference/manifest.json')).text(),'{}');
 assert.match(await(await fetch(base+'/__measure.mjs')).text(),/measureContentIndex/);
 assert.match(await(await fetch(base+'/__scene-measure.mjs')).text(),/measureSceneResolution/);
 assert.match(await(await fetch(base+'/measure-content-index.mjs')).text(),/measureContentIndex/);
 assert.equal((await fetch(base+'/assets/%2e%2e%5cprivate.txt')).status,400);
 assert.equal((await fetch(base+'/pkg/%2e%2e%5cprivate.txt')).status,400);
 const configuration=await(await fetch(base+'/__benchmark-config')).json();
 assert.equal(configuration.variant,'baseline-recheck');assert.match(configuration.identity.packages['the_long_slop_bg.wasm'],/^[a-f0-9]{64}$/);
 assert.match(configuration.identity.packages['terrain_worker_bg.wasm'],/^[a-f0-9]{64}$/);assert.match(configuration.identity.web['main.js'],/^[a-f0-9]{64}$/);assert.match(configuration.identity.scripts['measure-content-index.mjs'],/^[a-f0-9]{64}$/);
 const body=JSON.stringify({schemaVersion:2,label:'garage-fps',scenario:'garage',instrument:false,profileCpu:false,profileStreaming:false,cacheState:'warm',configuration,valid:false});
 const post=()=>fetch(base+'/__benchmark-result/garage-fps',{method:'POST',body});
 const saved=await post();assert.equal(saved.status,201);
 const receipt=JSON.parse(await readFile((await saved.json()).path,'utf8'));
 assert.deepEqual(receipt.server.identity,configuration.identity);
 assert.equal(receipt.server.variant,'baseline-recheck');assert.equal(receipt.valid,false);
 assert.equal((await post()).status,409);
 assert.equal((await fetch(base+'/__benchmark-result/other',{method:'POST',body:'not-json'})).status,400);
 assert.equal((await fetch(base+'/__benchmark-result/other',{method:'POST',body})).status,400);
 assert.equal((await fetch(base+'/index.html',{method:'DELETE'})).status,405);
 assert.equal((await fetch(base+'/private.txt')).status,404);
});
