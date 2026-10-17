/** Run through DevTools on a ready, focused game tab. Every hook is restored. */
export async function measureShadowCache({durationMs=10000,instrument=false,label='sample',resume=false}={}) {
 if(resume){
  if(!window.gameState.started){const {saveRuntime}=await import("/main.js");const token=await saveRuntime.prepareNewGame();await saveRuntime.commit(token);}
  const original=window.gameActions;
  try{window.gameActions=()=>4;while(window.gameState.paused)await new Promise(resolve=>requestAnimationFrame(resolve));}
  finally{window.gameActions=original;}
  const start=performance.now();let settled=0;
  while(settled<2){
   await new Promise(resolve=>setTimeout(resolve,1000));const w=await window.measureWorld();
   settled=w.world.pending_work===0&&w.world.generation_jobs===0?settled+1:0;
   if(performance.now()-start>60000)throw Error('World work did not settle');
  }
 }
 const canvas=document.querySelector('#game');
 const gl=canvas.getContext('webgl2');
 const originals=new Map();
 const counts={};
 let currentProgram=instrument?gl.getParameter(gl.CURRENT_PROGRAM):null;
 if(instrument){counts.programChanges=0;counts.shaderSwitches=0;}
 const intervals=[];
 const originalFrame=window.gamePerformanceFrame;
 let frames=0,last=null;
 const snapshot=()=>({...window.gameState,fluidVolumes:undefined,inventory:undefined});
 const before=await window.measureWorld();
 const stateBefore=snapshot();
 const {source_render_audit}=await import('/pkg/my_summer_slop.js');
 const auditBefore=JSON.parse(source_render_audit());
 const started=performance.now();
 window.gamePerformanceFrame=function(physicsMs,worldMs){
  const now=performance.now();if(last!==null)intervals.push(now-last);last=now;frames++;
  return originalFrame.call(this,physicsMs,worldMs);
 };
 if(instrument)for(const name of ['useProgram','drawArrays','drawElements','drawArraysInstanced','drawElementsInstanced','stencilFuncSeparate','clear','blitFramebuffer']){
  originals.set(name,gl[name]);counts[name]=0;
  gl[name]=function(...args){counts[name]++;if(name==='useProgram'&&args[0]!==currentProgram){counts.programChanges++;if(args[0]!==null)counts.shaderSwitches++;currentProgram=args[0];}return originals.get(name).apply(this,args);};
 }
 let elapsed;
 try {await new Promise(resolve=>setTimeout(resolve,durationMs));elapsed=performance.now()-started;}
 finally {window.gamePerformanceFrame=originalFrame;for(const [name,original] of originals)gl[name]=original;}
 const auditAfter=JSON.parse(source_render_audit());
 const stateAfter=snapshot();
 const after=await window.measureWorld();
 const sorted=intervals.sort((a,b)=>a-b);
 const percentile=p=>sorted[Math.min(sorted.length-1,Math.floor(sorted.length*p))];
 const info=gl.getExtension('WEBGL_debug_renderer_info');
 const submissionDelta=auditAfter.graph.effects.submitted_token-auditBefore.graph.effects.submitted_token;
 const settled=[before,after].every(w=>w.world.pending_work===0&&w.world.generation_jobs===0);
 const playing=stateBefore.started&&stateAfter.started&&!stateBefore.paused&&!stateAfter.paused;
 const valid=auditBefore.state==='Ready'&&auditAfter.state==='Ready'&&submissionDelta===frames&&(!resume||(playing&&settled));
 const result={valid,submissionDelta,elapsedMs:elapsed,frames,fps:frames*1000/elapsed,
  intervals:{meanMs:sorted.reduce((a,b)=>a+b,0)/sorted.length,p50Ms:percentile(.5),p95Ms:percentile(.95),maxMs:sorted.at(-1)},
  instrument,counts,perFrame:Object.fromEntries(Object.entries(counts).map(([name,count])=>[name,count/frames])),
  canvas:[canvas.width,canvas.height],dpr:devicePixelRatio,visibility:document.visibilityState,
  gpu:info?gl.getParameter(info.UNMASKED_RENDERER_WEBGL):gl.getParameter(gl.RENDERER),
  stateBefore,stateAfter,auditBefore,auditAfter,before,after};
 const saved=await fetch(`/__benchmark-result/${label}`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(result)});
 if(!saved.ok)throw Error("Benchmark receipt could not be saved");
 return {valid:result.valid,submissionDelta,label,elapsedMs:elapsed,frames,fps:result.fps,intervals:result.intervals,counts,perFrame:result.perFrame,canvas:result.canvas,visibility:result.visibility,states:[auditBefore.state,auditAfter.state],steps:[stateBefore.totalSteps,stateAfter.totalSteps]};
}
