import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve} from 'node:path';
import {contentIndexSampleFailures} from '../../scripts/measure-content-index.mjs';
import {createContentIndexBenchmarkServer} from '../../scripts/serve-content-index-benchmark.mjs';

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

test('server selects frozen inputs, confines paths, and preserves separate receipts',async t=>{
 const root=await mkdtemp(resolve(tmpdir(),'content-index-benchmark-'));
 t.after(()=>rm(root,{recursive:true,force:true}));
 const webRoot=resolve(root,'web'),assetRoot=resolve(root,'assets'),packageRoot=resolve(root,'pkg'),resultsRoot=resolve(root,'results');
 await Promise.all([mkdir(webRoot),mkdir(resolve(assetRoot,'reference'),{recursive:true}),mkdir(packageRoot)]);
 await Promise.all([writeFile(resolve(webRoot,'index.html'),'game'),writeFile(resolve(assetRoot,'reference/manifest.json'),'{}'),
  writeFile(resolve(packageRoot,'my_summer_slop.js'),'frozen-package'),writeFile(resolve(packageRoot,'my_summer_slop_bg.wasm'),'wasm'),
  writeFile(resolve(root,'private.txt'),'private')]);
 await assert.rejects(createContentIndexBenchmarkServer({webRoot,assetRoot,packageRoot,resultsRoot:resolve(webRoot,'receipts'),variant:'candidate'}),/outside served directories/);
 const server=await createContentIndexBenchmarkServer({webRoot,assetRoot,packageRoot,resultsRoot,variant:'baseline-recheck'});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 t.after(()=>new Promise((resolve,reject)=>{server.closeAllConnections();server.close(error=>error?reject(error):resolve());}));
 const base=`http://127.0.0.1:${server.address().port}`;
 assert.equal(await(await fetch(base+'/pkg/my_summer_slop.js')).text(),'frozen-package');
 assert.equal(await(await fetch(base+'/assets/reference/manifest.json')).text(),'{}');
 assert.match(await(await fetch(base+'/__measure.mjs')).text(),/measureContentIndex/);
 assert.match(await(await fetch(base+'/__scene-measure.mjs')).text(),/measureSceneResolution/);
 assert.match(await(await fetch(base+'/measure-content-index.mjs')).text(),/measureContentIndex/);
 assert.equal((await fetch(base+'/assets/%2e%2e%5cprivate.txt')).status,400);
 assert.equal((await fetch(base+'/pkg/%2e%2e%5cprivate.txt')).status,400);
 const configuration=await(await fetch(base+'/__benchmark-config')).json();
 assert.equal(configuration.variant,'baseline-recheck');assert.match(configuration.packageWasmSha256,/^[a-f0-9]{64}$/);
 const body=JSON.stringify({label:'garage-fps',scenario:'garage',instrument:false,valid:false});
 const post=()=>fetch(base+'/__benchmark-result/garage-fps',{method:'POST',body});
 const saved=await post();assert.equal(saved.status,201);
 const receipt=JSON.parse(await readFile((await saved.json()).path,'utf8'));
 assert.equal(receipt.server.packageWasmSha256,configuration.packageWasmSha256);
 assert.equal(receipt.server.variant,'baseline-recheck');assert.equal(receipt.valid,false);
 assert.equal((await post()).status,409);
 assert.equal((await fetch(base+'/__benchmark-result/other',{method:'POST',body:'not-json'})).status,400);
 assert.equal((await fetch(base+'/__benchmark-result/other',{method:'POST',body})).status,400);
 assert.equal((await fetch(base+'/index.html',{method:'DELETE'})).status,405);
 assert.equal((await fetch(base+'/private.txt')).status,404);
});
