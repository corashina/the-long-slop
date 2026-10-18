import {saveRuntime} from '/main.js';
import {source_render_audit} from '/pkg/my_summer_slop.js';

const next=()=>new Promise(resolve=>requestAnimationFrame(resolve));
const audit=()=>JSON.parse(source_render_audit()??'null');
const originals=Object.fromEntries(['gameActions','gameKeys','gamePressed','gameReleased','gameMouseX','gameMouseY','gameWheel'].map(key=>[key,window[key]]));
const overlays=['#menu','#performance'].map(selector=>document.querySelector(selector));
const displays=overlays.map(node=>node.style.display);
for(const node of overlays)node.style.display='none';
let action=0,held=0,pressed=0,released=0,mouseX=0,mouseY=0,autoResume=false;
const frame={pressed:0,released:0,mouseX:0,mouseY:0};
function setKeys(value){pressed=(pressed|(value&~held))>>>0;released=(released|(held&~value))>>>0;held=value>>>0;}
// Control inputs only. FPS samples never wrap graphics or frame callbacks.
window.gameActions=()=>{
 const queued=originals.gameActions(),explicit=action;action=0;
 if(explicit)return explicit;
 return autoResume&&window.gameState.started?(window.gameState.paused?4:queued&~2):queued;
};
window.gameKeys=()=>{
 originals.gameKeys();frame.pressed=pressed;frame.released=released;frame.mouseX=mouseX;frame.mouseY=mouseY;
 pressed=released=mouseX=mouseY=0;return held;
};
window.gamePressed=()=>frame.pressed;window.gameReleased=()=>frame.released;
window.gameMouseX=()=>frame.mouseX;window.gameMouseY=()=>frame.mouseY;window.gameWheel=()=>0;

async function wait(test,timeout=180000){
 const start=performance.now();while(!test()){
  const failure=document.querySelector('#runtime-error');
  if(failure&&!failure.hidden&&failure.textContent)throw Error(failure.textContent);
  if(performance.now()-start>timeout)throw Error('Benchmark wait timed out');await next();
 }
}
async function delay(ms){const start=performance.now();while(performance.now()-start<ms)await next();}
async function pause(){autoResume=false;setKeys(0);if(!window.gameState.paused){action=2;await wait(()=>window.gameState.paused);}}
async function resume(){autoResume=true;action=4;await wait(()=>window.gameState.started&&!window.gameState.paused);}
async function ready(){
 await wait(()=>window.gameStartupDone()&&!document.querySelector('#new-game').disabled
  &&audit()?.state==='Ready'&&audit().graph.presented);
}
async function load(saved){await pause();await saveRuntime.commit(await saveRuntime.prepare(saved));await ready();}
async function saveReport(data){
 const response=await fetch('/__report',{method:'POST',body:JSON.stringify(data)});
 if(!response.ok)throw Error('Cannot save benchmark report');return data;
}
function countCalls(){
 const gl=document.querySelector('#game').getContext('webgl2');
 const names=['activeTexture','bindTexture','texParameteri','bindSampler','bindBuffer','bindBufferRange',
  'vertexAttribPointer','vertexAttribIPointer','vertexAttribDivisor','enableVertexAttribArray','disableVertexAttribArray',
  'useProgram','drawArrays','drawElements','drawArraysInstanced','drawElementsInstanced','bufferData','bufferSubData'];
 const proto=WebGL2RenderingContext.prototype,methods=new Map(),counts=Object.fromEntries(names.map(name=>[name,0]));
 const samplers=new Map();let repeatedSamplers=0;
 for(const name of names){
  const original=proto[name];methods.set(name,original);
  proto[name]=function(...args){
   if(this===gl){counts[name]++;if(name==='bindSampler'){
    if(samplers.has(args[0])&&samplers.get(args[0])===args[1])repeatedSamplers++;
    samplers.set(args[0],args[1]);
   }}
   return original.apply(this,args);
  };
 }
 return {stop(){for(const [name,original] of methods)proto[name]=original;return {counts,repeatedSamplers};}};
}

export const bench={
 ready,pause,resume,load,audit,
 async hold(value,ms){setKeys(value);try{await delay(ms);}finally{setKeys(0);}},
 async tap(value){setKeys(value);await next();await next();setKeys(0);},
 async look(x,y){mouseX+=x;mouseY+=y;await next();await next();},
 async capture(name){await pause();const saved=await saveRuntime.capture();await saveReport({name,saved});return saved;},
 async fixture(name){const response=await fetch('/__fixtures/'+name+'.json');if(!response.ok)throw Error('Missing fixture');const data=await response.json();return data.saved??data;},
 async sample(name,{keys=0,seconds=10,warmup=10000,instrumented=false}={}){
  if(document.hidden)throw Error('Benchmark tab is hidden');
  await ready();await resume();
  await delay(warmup);setKeys(keys);
  const canvas=document.querySelector('#game');
  const before={state:structuredClone(window.gameState),audit:audit(),menuHidden:document.querySelector('#menu').hidden,menuDisplay:getComputedStyle(document.querySelector('#menu')).display,
   pointerLock:document.pointerLockElement===canvas};
  const probe=instrumented?countCalls():null;
  const intervals=[];let last=await next(),start=last,callCounts;
  try{while(last-start<seconds*1000){const now=await next();intervals.push(now-last);last=now;}}
  finally{callCounts=probe?.stop();setKeys(0);}
  const after={state:structuredClone(window.gameState),audit:audit(),menuHidden:document.querySelector('#menu').hidden,menuDisplay:getComputedStyle(document.querySelector('#menu')).display,
   pointerLock:document.pointerLockElement===canvas};
  if(document.hidden)throw Error('Benchmark tab became hidden');
  const sorted=intervals.slice().sort((a,b)=>a-b),elapsed=last-start;
  const mean=intervals.reduce((a,b)=>a+b,0)/intervals.length;
  const submitted=after.audit.graph.token-before.audit.graph.token;
  const renderCadence=submitted>=intervals.length-2;
  const data={name,recordedAt:new Date().toISOString(),instrumented,seconds:elapsed/1000,frames:intervals.length,fps:1000/mean,
   submittedFrames:submitted,submittedFps:submitted*1000/elapsed,renderCadence,
   meanMs:mean,medianMs:sorted[Math.floor(sorted.length*.5)],p95Ms:sorted[Math.floor(sorted.length*.95)],
   p99Ms:sorted[Math.floor(sorted.length*.99)],maxMs:sorted.at(-1),intervals,
   viewport:[canvas.width,canvas.height],dpr:devicePixelRatio,visibility:document.visibilityState,
   before,after,...callCounts};
  await saveReport(data);
  return {name,frames:data.frames,fps:data.fps,meanMs:data.meanMs,p95Ms:data.p95Ms,
   submitted,renderCadence,seated:after.state.seated,
   speed:after.state.speedKmh,travel:after.state.travelM-before.state.travelM,
   ready:after.audit.state,counts:callCounts?.counts};
 },
 async observe(name,seconds=20){
  const snapshots=[];const start=performance.now();while(performance.now()-start<seconds*1000){snapshots.push(audit());await delay(500);}
  return saveReport({name,snapshots,state:structuredClone(window.gameState)});
 },
 async contextLoss(){
  const gl=document.querySelector('#game').getContext('webgl2');
  gl.getExtension('WEBGL_lose_context').loseContext();await delay(300);
  return {lost:gl.isContextLost(),error:document.querySelector('#capture-error')?.textContent,
   runtime:document.querySelector('#runtime-error')?.textContent};
 },
 cleanup(){for(const key in originals)window[key]=originals[key];overlays.forEach((node,index)=>{node.style.display=displays[index];});}
};
window.samplerBench=bench;
