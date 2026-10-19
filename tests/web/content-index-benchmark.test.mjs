import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve} from 'node:path';
import {contentIndexSampleFailures} from '../../scripts/measure-content-index.mjs';
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

test('startup and save-load receipts require their own scenario evidence',()=>{
 const startup={...sample(),scenario:'startup-cold',cacheState:'cold',startupTrace:{completed:true,inputReady:true,failure:null}};
 startup.stateBefore.started=false;startup.stateAfter.started=false;
 assert.deepEqual(contentIndexSampleFailures(startup),[]);
 assert.ok(contentIndexSampleFailures({...startup,cacheState:'warm'}).includes('startup-cache-state-mismatch'));
 assert.ok(contentIndexSampleFailures({...startup,startupTrace:{completed:false}}).includes('startup-not-complete'));
 const save={...sample(),scenario:'save-load',pausedFrames:2,saveOperations:[{kind:'Capture'},{kind:'Prepare'},{kind:'Commit'}]};
 assert.deepEqual(contentIndexSampleFailures(save),[]);
 assert.ok(contentIndexSampleFailures({...save,saveOperations:[]}).includes('save-load-not-observed'));
});

test('server selects frozen inputs, confines paths, and preserves separate receipts',async t=>{
 const root=await mkdtemp(resolve(tmpdir(),'content-index-benchmark-'));
 t.after(()=>rm(root,{recursive:true,force:true}));
 const webRoot=resolve(root,'web'),scriptsRoot=resolve(root,'scripts'),assetRoot=resolve(root,'assets'),packageRoot=resolve(root,'pkg'),resultsRoot=resolve(root,'results'),commit='a'.repeat(40);
 await Promise.all([mkdir(webRoot),mkdir(scriptsRoot),mkdir(resolve(assetRoot,'reference'),{recursive:true}),mkdir(packageRoot)]);
 await Promise.all([writeFile(resolve(webRoot,'index.html'),'game'),writeFile(resolve(assetRoot,'reference/manifest.json'),'{}'),
  writeFile(resolve(packageRoot,'my_summer_slop.js'),'frozen-package'),writeFile(resolve(packageRoot,'my_summer_slop_bg.wasm'),'wasm'),
  writeFile(resolve(root,'private.txt'),'private'),writeFile(resolve(webRoot,'main.js'),'frozen-main'),writeFile(resolve(scriptsRoot,'measure-content-index.mjs'),'frozen measureContentIndex'),writeFile(resolve(packageRoot,'terrain_worker.js'),'frozen-worker'),writeFile(resolve(packageRoot,'terrain_worker_bg.wasm'),'worker-wasm')]);
 await assert.rejects(createContentIndexBenchmarkServer({webRoot,scriptsRoot,commit,assetRoot,packageRoot,resultsRoot:resolve(webRoot,'receipts'),variant:'candidate'}),/outside served directories/);
 const server=await createContentIndexBenchmarkServer({webRoot,scriptsRoot,commit,assetRoot,packageRoot,resultsRoot,variant:'baseline-recheck'});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 t.after(()=>new Promise((resolve,reject)=>{server.closeAllConnections();server.close(error=>error?reject(error):resolve());}));
 const base=`http://127.0.0.1:${server.address().port}`;
 assert.equal(await(await fetch(base+'/pkg/my_summer_slop.js')).text(),'frozen-package');
 assert.equal(await(await fetch(base+'/assets/reference/manifest.json')).text(),'{}');
 assert.match(await(await fetch(base+'/__measure.mjs')).text(),/measureContentIndex/);
 assert.equal((await fetch(base+'/assets/%2e%2e%5cprivate.txt')).status,400);
 assert.equal((await fetch(base+'/pkg/%2e%2e%5cprivate.txt')).status,400);
 const configuration=await(await fetch(base+'/__benchmark-config')).json();
 assert.equal(configuration.variant,'baseline-recheck');assert.match(configuration.identity.packages['my_summer_slop_bg.wasm'],/^[a-f0-9]{64}$/);
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
