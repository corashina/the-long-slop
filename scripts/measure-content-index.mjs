/** Run in DevTools after starting and focusing the game. Hooks restore after each sample. */
export function contentIndexSampleFailures(sample) {
 const failures=[];
 if(sample.auditBefore.state!=='Ready'||sample.auditAfter.state!=='Ready')failures.push('renderer-not-ready');
 if(sample.frames<2||sample.submissionDelta!==sample.frames)failures.push('submitted-frame-mismatch');
 if(sample.hiddenDuringSample)failures.push('hidden-tab');
 if(sample.unfocusedDuringSample)failures.push('unfocused-tab');
 if(!sample.stateBefore.started||!sample.stateAfter.started||sample.notStartedFrames)failures.push('game-not-started');
 if(sample.stateBefore.paused||sample.stateAfter.paused||sample.pausedFrames)failures.push('game-paused');
 if(!(sample.stateAfter.totalSteps>sample.stateBefore.totalSteps))failures.push('physics-did-not-step');
 if(sample.scenario==='garage'&&!sample.settled)failures.push('garage-work-not-settled');
 if(sample.scenario==='driving'&&(!sample.stateBefore.seated||!sample.stateAfter.seated||sample.unseatedFrames||!(sample.travelDeltaM>0)))failures.push('driving-not-observed');
 return failures;
}

export async function measureContentIndex({durationMs=10000,instrument=false,profileStreaming=false,label='sample',scenario='garage'}={}) {
 if(!Number.isFinite(durationMs)||durationMs<1000)throw Error('durationMs must be at least 1000');
 if(typeof instrument!=='boolean')throw Error('instrument must be a boolean');
 if(typeof profileStreaming!=='boolean')throw Error('profileStreaming must be a boolean');
 if(instrument&&profileStreaming)throw Error('Collect streaming stages and WebGL calls separately');
 if(!['garage','driving'].includes(scenario))throw Error('scenario must be garage or driving');
 if(!/^[a-z0-9][a-z0-9-]{0,95}$/.test(label))throw Error('Invalid receipt label');
 const configResponse=await fetch('/__benchmark-config');
 if(!configResponse.ok)throw Error('Benchmark server configuration is unavailable');
 const configuration=await configResponse.json();
 const canvas=document.querySelector('#game'),gl=canvas?.getContext('webgl2');
 if(!gl||typeof window.gamePerformanceFrame!=='function'||typeof window.measureWorld!=='function')throw Error('The game is unavailable');
 const {source_render_audit,streaming_profile,streaming_profile_audit}=await import('/pkg/my_summer_slop.js');
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
 let elapsed,streamingProfile=null;
 try {
  if(profileStreaming)streaming_profile(true);
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
  if(profileStreaming){streamingProfile=JSON.parse(streaming_profile_audit());streaming_profile(false);}
  window.gamePerformanceFrame=originalFrame;
  for(const [name,original] of originals)gl[name]=original;
  for(const type of eventTypes)document.removeEventListener(type,inputEvent,true);
  document.removeEventListener('visibilitychange',visibilityChanged);
  document.removeEventListener('pointerlockchange',pointerLockChanged);
  window.removeEventListener('blur',blurred);
 }
 const auditAfter=JSON.parse(source_render_audit()),stateAfter=snapshot();
 const after=await window.measureWorld();
 intervals.sort((a,b)=>a-b);
 const percentile=p=>intervals.length?intervals[Math.ceil(intervals.length*p)-1]:null;
 const info=gl.getExtension('WEBGL_debug_renderer_info');
 input.codes=[...codes].sort();
 const result={label,scenario,instrument,profileStreaming,streamingProfile,configuration,recordedAt:new Date().toISOString(),
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
 result.performanceSample=result.valid&&!instrument&&!profileStreaming;
 const saved=await fetch(`/__benchmark-result/${label}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(result)});
 if(!saved.ok)throw Error(`Benchmark receipt could not be saved: HTTP ${saved.status}`);
 return {label,scenario,valid:result.valid,failures:result.failures,instrument,profileStreaming,streamingProfile,performanceSample:result.performanceSample,
  realInputObserved:result.realInputObserved,elapsedMs:elapsed,frames,submissionDelta:result.submissionDelta,
  fps:result.fps,intervals:result.intervals,steps:[stateBefore.totalSteps,stateAfter.totalSteps],travelDeltaM:result.travelDeltaM,
  settled:result.settled,counts,receipt:await saved.json()};
}
