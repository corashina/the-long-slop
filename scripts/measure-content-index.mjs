/** Run in DevTools after starting and focusing the game. Hooks restore after each sample. */
export function contentIndexSampleFailures(sample) {
 const failures=[];
 if(sample.scenario.startsWith('startup-')){
  if(typeof sample.startupTrace?.hiddenDuringStartup!=='boolean'||typeof sample.startupTrace?.unfocusedDuringStartup!=='boolean')failures.push('startup-environment-missing');
  if(sample.startupTrace?.hiddenDuringStartup)failures.push('startup-hidden-tab');
  if(sample.startupTrace?.unfocusedDuringStartup)failures.push('startup-unfocused-tab');
  if(sample.startupTrace?.profileCpu!==sample.profileCpu)failures.push('startup-instrumentation-mismatch');
  if(!sample.startupTrace?.completed||!sample.startupTrace?.inputReady||sample.startupTrace?.failure)failures.push('startup-not-complete');
  if(sample.cacheState!==(sample.scenario==='startup-cold'?'cold':'warm'))failures.push('startup-cache-state-mismatch');
  return failures;
 }
 if(sample.hiddenDuringSample)failures.push('hidden-tab');
 if(sample.unfocusedDuringSample)failures.push('unfocused-tab');
 if(sample.auditBefore.state!=='Ready'||sample.auditAfter.state!=='Ready')failures.push('renderer-not-ready');
 if(sample.frames<2||sample.submissionDelta!==sample.frames)failures.push('submitted-frame-mismatch');
 if(!sample.stateBefore.started||!sample.stateAfter.started||sample.notStartedFrames)failures.push('game-not-started');
 if(sample.scenario!=='save-load'&&(sample.stateBefore.paused||sample.stateAfter.paused||sample.pausedFrames))failures.push('game-paused');
 if(!(sample.stateAfter.totalSteps>sample.stateBefore.totalSteps))failures.push('physics-did-not-step');
 if(sample.scenario==='garage'&&!sample.settled)failures.push('garage-work-not-settled');
 if(sample.scenario==='driving'&&(!sample.stateBefore.seated||!sample.stateAfter.seated||sample.unseatedFrames||!(sample.travelDeltaM>0)))failures.push('driving-not-observed');
 if(sample.scenario==='save-load'&&(!sample.saveOperations?.some(row=>row.kind==='Capture'&&!row.error)||!sample.saveOperations?.some(row=>row.kind==='Prepare'&&!row.error)||!sample.saveOperations?.some(row=>row.kind==='Commit'&&!row.error)))failures.push('save-load-not-observed');
 return failures;
}

export function assertCpuProfileMode(profileCpu,profileStreaming,state){
 if(profileCpu)return;
 if(state.browserEnabled||state.rustEnabled||state.startupRenderTraceActive||state.terrain.jobActive||state.terrain.handshakeActive||state.road.jobActive||state.road.handshakeActive||state.road.publicationActive||state.streamingEnabled&&!profileStreaming)throw Error('CPU or streaming profiling is active; stop it before this sample');
}

export async function measureContentIndex({durationMs=10000,instrument=false,profileStreaming=false,profileCpu=false,label='sample',scenario='garage',cacheState='unknown',route='unspecified',seed=null,browserTrace=null,traceRecording=false}={}) {
 if(!Number.isFinite(durationMs)||durationMs<1000)throw Error('durationMs must be at least 1000');
 if(typeof instrument!=='boolean')throw Error('instrument must be a boolean');
 if(typeof profileStreaming!=='boolean')throw Error('profileStreaming must be a boolean');
 if(typeof profileCpu!=='boolean')throw Error('profileCpu must be a boolean');
 if(typeof traceRecording!=='boolean')throw Error('traceRecording must be a boolean');
 if(Number(instrument)+Number(profileStreaming)+Number(profileCpu)>1)throw Error('Collect CPU stages, streaming stages, and WebGL calls separately');
 if(!['garage','driving','startup-cold','startup-warm','interaction','save-load'].includes(scenario))throw Error('Invalid scenario');
 if(!['cold','warm','unknown'].includes(cacheState))throw Error('Invalid cache state');
 if(seed!==null&&!Number.isInteger(seed))throw Error('Invalid benchmark seed');
 if(browserTrace!==null&&(!browserTrace.path||!/^[a-f0-9]{64}$/.test(browserTrace.sha256??'')))throw Error('A browser trace needs its saved path and SHA-256');
 if(!/^[a-z0-9][a-z0-9-]{0,95}$/.test(label))throw Error('Invalid receipt label');
 const configResponse=await fetch('/__benchmark-config');
 if(!configResponse.ok)throw Error('Benchmark server configuration is unavailable');
 const configuration=await configResponse.json();
 const canvas=document.querySelector('#game'),gl=canvas?.getContext('webgl2');
 if(!gl||typeof window.gamePerformanceFrame!=='function'||typeof window.measureWorld!=='function')throw Error('The game is unavailable');
 const {source_render_audit,streaming_profile,streaming_profile_audit,cpu_profile_audit}=await import('/pkg/my_summer_slop.js');
 const startupScenario=scenario.startsWith('startup-');
 if(!window.gameCpuInstrumentationState||!cpu_profile_audit||!streaming_profile_audit)throw Error('Instrumentation state is unavailable');
 const assertActiveMode=()=>assertCpuProfileMode(profileCpu,profileStreaming,{...window.gameCpuInstrumentationState(),rustEnabled:JSON.parse(cpu_profile_audit()).enabled,streamingEnabled:JSON.parse(streaming_profile_audit()).enabled});
 assertActiveMode();
 if(profileCpu&&(!window.gameCpuProfile||!cpu_profile_audit))throw Error('CPU profiler is unavailable');
 if(startupScenario&&profileCpu&&!window.gameCpuProfileSnapshot?.().enabled)throw Error('Reload with ?profileCpu=1 before startup profiling');
 if(profileStreaming&&(!streaming_profile||!streaming_profile_audit))throw Error('Streaming profiler is unavailable');
 const snapshot=()=>({...window.gameState,fluidVolumes:undefined,inventory:undefined});
 const before=await window.measureWorld();
 const stateBefore=snapshot(),auditBefore=JSON.parse(source_render_audit());
 const originalFrame=window.gamePerformanceFrame,originals=new Map(),counts={};
 const intervals=[],input={trustedEvents:0,syntheticEvents:0,trustedEventsWithPointerLock:0,codes:[],pointerLockSeen:document.pointerLockElement===canvas};
 const codes=new Set();
 let frames=0,last=null,pausedFrames=0,notStartedFrames=0,unseatedFrames=0;
 let hiddenDuringSample=document.hidden,unfocusedDuringSample=!document.hasFocus();
 const visibilityChanged=()=>{hiddenDuringSample||=document.hidden;};
 const blurred=()=>{unfocusedDuringSample=true;};
 const pointerLockChanged=()=>{input.pointerLockSeen||=document.pointerLockElement===canvas;};
 const inputEvent=event=>{
  if(event.type==='mousemove'&&!event.movementX&&!event.movementY)return;
  if(event.isTrusted){input.trustedEvents++;if(document.pointerLockElement===canvas)input.trustedEventsWithPointerLock++;}
  else input.syntheticEvents++;
  if(event.code)codes.add(event.code);
 };
 const eventTypes=['keydown','keyup','mousedown','mouseup','mousemove'];
 const started=performance.now();
 const saveOperations=[],originalSaveResult=window.gameSaveResult;
 let elapsed,streamingProfile=null,cpuProfile=null,renderMainThreadTrace=null,renderTrace=null;
 try {
  if(profileCpu){if(!startupScenario)window.gameCpuProfile(true);const {observeRenderMainThread}=await import('/cpu-profile.js');renderTrace=observeRenderMainThread();}
  if(profileStreaming)streaming_profile(true);
  if(scenario==='save-load')window.gameSaveResult=function(id,kind,payload,error){saveOperations.push({id,kind,error:error||null,at:performance.now()});return originalSaveResult.call(this,id,kind,payload,error);};
  document.addEventListener('visibilitychange',visibilityChanged);
  document.addEventListener('pointerlockchange',pointerLockChanged);
  window.addEventListener('blur',blurred);
  for(const type of eventTypes)document.addEventListener(type,inputEvent,true);
  window.gamePerformanceFrame=function(physicsMs,worldMs){
   const now=performance.now();if(last!==null)intervals.push(now-last);last=now;frames++;
   if(window.gameState.paused)pausedFrames++;
   if(!window.gameState.started)notStartedFrames++;
   if(!window.gameState.seated)unseatedFrames++;
   return originalFrame.call(this,physicsMs,worldMs);
  };
  if(instrument)for(const name of ['drawArrays','drawElements','drawArraysInstanced','drawElementsInstanced','useProgram','bindBuffer']){
   const original=gl[name];originals.set(name,original);counts[name]=0;
   gl[name]=function(...args){counts[name]++;return original.apply(this,args);};
  }
  await new Promise(resolve=>setTimeout(resolve,durationMs));
  elapsed=performance.now()-started;
 } finally {
  if(profileCpu){cpuProfile={browser:window.gameCpuProfileSnapshot(),rust:JSON.parse(cpu_profile_audit())};renderMainThreadTrace={sample:renderTrace?.stop()??null,startup:startupScenario?window.gameCpuStartupRenderTrace():null};window.gameCpuProfile(false);}
  if(scenario==='save-load')window.gameSaveResult=originalSaveResult;
  if(profileStreaming){streamingProfile=JSON.parse(streaming_profile_audit());streaming_profile(false);}
  window.gamePerformanceFrame=originalFrame;
  for(const [name,original] of originals)gl[name]=original;
  for(const type of eventTypes)document.removeEventListener(type,inputEvent,true);
  document.removeEventListener('visibilitychange',visibilityChanged);
  document.removeEventListener('pointerlockchange',pointerLockChanged);
  window.removeEventListener('blur',blurred);
 }
 const auditAfter=JSON.parse(source_render_audit()),stateAfter=snapshot();
 assertActiveMode();
 const after=await window.measureWorld();
 intervals.sort((a,b)=>a-b);
 const percentile=p=>intervals.length?intervals[Math.ceil(intervals.length*p)-1]:null;
 const info=gl.getExtension('WEBGL_debug_renderer_info');
 input.codes=[...codes].sort();
 const result={schemaVersion:2,label,scenario,cacheState,route,seed,instrument,profileStreaming,profileCpu,cpuProfile,renderMainThreadTrace,streamingProfile,configuration,recordedAt:new Date().toISOString(),
  instrumentationMode:profileCpu?'cpu':profileStreaming?'streaming':instrument?'webgl':traceRecording||browserTrace?'browser-trace':'none',browser:navigator.userAgent,browserTrace,traceRecording,
  startupTrace:startupScenario?window.gameStartupTrace():null,saveOperations,
  elapsedMs:elapsed,requestedDurationMs:durationMs,frames,fps:frames*1000/elapsed,
  intervals:{count:intervals.length,meanMs:intervals.length?intervals.reduce((sum,value)=>sum+value,0)/intervals.length:null,p95Ms:percentile(.95),p99Ms:percentile(.99),maxMs:intervals.at(-1)??null},
  submissionDelta:auditAfter.graph.effects.submitted_token-auditBefore.graph.effects.submitted_token,
  pausedFrames,notStartedFrames,unseatedFrames,hiddenDuringSample,unfocusedDuringSample,
  settled:[before,after].every(value=>value.world.pending_work===0&&value.world.generation_jobs===0),
  travelDeltaM:stateAfter.travelM-stateBefore.travelM,
  input,realInputObserved:input.trustedEventsWithPointerLock>0,
  counts,perFrame:Object.fromEntries(Object.entries(counts).map(([name,count])=>[name,count/frames])),
  cacheCounters:null,cacheCountersUnavailableReason:'No content-index counter export exists in this build.',
  canvas:[canvas.width,canvas.height],dpr:devicePixelRatio,visibility:document.visibilityState,
  gpu:info?gl.getParameter(info.UNMASKED_RENDERER_WEBGL):gl.getParameter(gl.RENDERER),
  stateBefore,stateAfter,auditBefore,auditAfter,before,after};
 result.failures=contentIndexSampleFailures(result);
 result.valid=result.failures.length===0;
 result.performanceSample=result.valid&&!instrument&&!profileStreaming&&!profileCpu&&!traceRecording&&!browserTrace;
 const saved=await fetch(`/__benchmark-result/${label}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(result)});
 if(!saved.ok)throw Error(`Benchmark receipt could not be saved: HTTP ${saved.status}`);
 return {label,scenario,valid:result.valid,failures:result.failures,instrument,profileStreaming,profileCpu,cpuProfile,streamingProfile,performanceSample:result.performanceSample,
  realInputObserved:result.realInputObserved,elapsedMs:elapsed,frames,submissionDelta:result.submissionDelta,
  fps:result.fps,intervals:result.intervals,steps:[stateBefore.totalSteps,stateAfter.totalSteps],travelDeltaM:result.travelDeltaM,
  settled:result.settled,counts,receipt:await saved.json()};
}
