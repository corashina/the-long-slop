// Load this module in the active game tab after the canvas reaches 1280x720.
// FPS samples run without WebGL wrappers. Capture draws in a separate sample.
import {saveRuntime} from '/main.js';
import {ACTIONS} from '/menu.js';
import {source_render_audit} from '/pkg/the_long_slop.js';
const next=()=>new Promise(resolve=>requestAnimationFrame(resolve));
const audit=()=>JSON.parse(source_render_audit());
const read=()=>structuredClone(window.gameState);
const original={},overlays=[];
let actions=0,held=0,pressed=0,released=0,mouseX=0,mouseY=0,installed=false,autoResume=false;
export function installInput(){
 if(installed)return;installed=true;
 for(const selector of ['#menu','#performance']){const node=document.querySelector(selector);overlays.push([node,node.style.display]);node.style.display='none';}
 for(const key of ['gameActions','gameKeys','gamePressed','gameReleased','gameMouseX','gameMouseY','gameWheel'])original[key]=window[key];
 window.gameActions=()=>{const explicit=actions,queued=original.gameActions();actions=0;return explicit||(autoResume?(window.gameState.paused?ACTIONS.CONTINUE:queued&~ACTIONS.PAUSE):queued);};
 window.gameKeys=()=>{original.gameKeys();return held;};
 window.gamePressed=()=>{const value=pressed;pressed=0;return value;};
 window.gameReleased=()=>{const value=released;released=0;return value;};
 window.gameMouseX=()=>{const value=mouseX;mouseX=0;return value;};
 window.gameMouseY=()=>{const value=mouseY;mouseY=0;return value;};
 window.gameWheel=()=>0;
}
export function input(keys=0,press=keys&~held){installInput();pressed|=press;released|=held&~keys;held=keys;}
export async function resume(){installInput();autoResume=true;actions=ACTIONS.CONTINUE;await wait(()=>window.gameState.started&&!window.gameState.paused);}
export function look(x,y){installInput();mouseX+=x;mouseY+=y;}
export function action(value){installInput();actions|=value;}
export async function steps(count){const start=window.gameState.totalSteps;await wait(()=>window.gameState.totalSteps>=start+count);return read();}
async function bounded(work,timeout=120000){
 let timer;
 try{return await Promise.race([work,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Save operation timed out')),timeout);})]);}
 finally{clearTimeout(timer);}
}
async function wait(test,timeout=240000){
 const start=performance.now();while(!test()){const status=audit()?.state;if(status?.startsWith('Failed'))throw Error(status);if(performance.now()-start>timeout)throw Error('Batch benchmark timeout');await next();}
}
function assertReady(){
 const a=audit(),s=read(),canvas=document.querySelector('#game');
 if(a.state!=='Ready'||!s.started||s.paused||document.hidden)throw Error('Benchmark requires active gameplay');
 if(canvas.width!==1280||canvas.height!==720||devicePixelRatio!==1)throw Error('Benchmark resolution mismatch');
 return a;
}
export async function pause(){autoResume=false;input(0);if(!window.gameState.paused){actions=ACTIONS.PAUSE;await wait(()=>window.gameState.paused);}}
export async function capture(){await pause();return bounded(saveRuntime.capture(),30000);}
export async function load(saved){
 await pause();const previous=await bounded(window.measureWorld(),30000),controller=new AbortController();
 const timer=setTimeout(()=>controller.abort(),120000);let token;
 try{token=await saveRuntime.prepare(structuredClone(saved),controller.signal);}
 finally{clearTimeout(timer);}
 await bounded(saveRuntime.commit(token));
 await wait(()=>audit()?.state==='Ready');
 const committed=await bounded(saveRuntime.capture(),30000),current=await bounded(window.measureWorld(),30000);
 if(current.session===previous.session)throw Error('Load did not commit a replacement session');
 function compare(expected,actual,path){
  if(typeof expected==='number'&&typeof actual==='number'&&Number.isFinite(actual)&&Math.abs(expected-actual)<=0.0001)return;
  if(expected===actual)return;
  if(expected===null||actual===null||typeof expected!=='object'||typeof actual!=='object'||Array.isArray(expected)!==Array.isArray(actual))throw Error(`Load readback mismatch: ${path}`);
  const keys=Object.keys(expected).filter(key=>path!=='save'||key!=='saved_at').sort();
  const actualKeys=Object.keys(actual).filter(key=>path!=='save'||key!=='saved_at').sort();
  if(JSON.stringify(keys)!==JSON.stringify(actualKeys))throw Error(`Load readback keys mismatch: ${path}`);
  for(const key of keys)compare(expected[key],actual[key],`${path}.${key}`);
 }
 compare(saved,committed,'save');
 const before=read(),beforeAudit=audit();await resume();
 await wait(()=>window.gameState.totalSteps>=before.totalSteps+2&&audit().graph.views[0].frame>beforeAudit.graph.views[0].frame);
 const after=read(),afterAudit=audit();
 return {previousSession:previous.session,committedSession:current.session,committed,before,after,beforeAudit,afterAudit};
}
export async function sample(label,seconds=10,keys=0,scene='garage'){
 assertReady();input(keys);
 const beforeWorld=await bounded(window.measureWorld(),30000);
 if(beforeWorld.world.generation_jobs||beforeWorld.world.pending_work)throw Error('World work must settle before sampling');
 const before=read(),beforeAudit=audit(),buffer=new Float64Array(8192);let count=0,overflow=false;
 const originalFrame=window.gamePerformanceFrame;
 let previous=null,frames=0,hidden=false,paused=false;
 const visibility=()=>{hidden||=document.hidden;};
 document.addEventListener('visibilitychange',visibility);
 window.gamePerformanceFrame=function(physics,world){
  paused||=window.gameState.paused;const now=performance.now();if(previous!==null){if(count<buffer.length)buffer[count++]=now-previous;else overflow=true;}previous=now;frames++;
  return originalFrame.call(this,physics,world);
 };
 const start=performance.now();let elapsed;
 try {await new Promise(resolve=>setTimeout(resolve,seconds*1000));elapsed=performance.now()-start;}
 finally {window.gamePerformanceFrame=originalFrame;document.removeEventListener('visibilitychange',visibility);input(0);}
 assertReady();const after=read(),afterAudit=audit(),afterWorld=await bounded(window.measureWorld(),30000);
 if(hidden||paused)throw Error('Gameplay paused or became hidden during sampling');
 if(scene==='garage'&&(afterWorld.world.generation_jobs||afterWorld.world.pending_work||beforeWorld.world.generated_objects!==afterWorld.world.generated_objects))throw Error('Garage world work changed during sampling');
 if(scene==='driving'&&(!before.seated||!after.seated||Math.hypot(after.absoluteX-before.absoluteX,after.absoluteZ-before.absoluteZ)<.25))throw Error('Driving sample requires a seated player and vehicle movement');
 if(after.totalSteps<=before.totalSteps||count===0)throw Error('Game frames or physics did not advance');
 if(afterAudit.graph.views[0].frame<=beforeAudit.graph.views[0].frame)throw Error('Main render submissions did not advance');
 if(overflow)throw Error('Timing buffer capacity exceeded');
 const times=buffer.subarray(0,count),measuredMs=times.reduce((sum,value)=>sum+value,0);times.sort();
 const gl=document.querySelector('#game').getContext('webgl2'),info=gl.getExtension('WEBGL_debug_renderer_info');
 const report={label,scene,world:{before:beforeWorld.world,after:afterWorld.world},seconds:measuredMs/1000,wallSeconds:elapsed/1000,frames:count,frameCallbacks:frames,fps:count*1000/measuredMs,
  mean:measuredMs/count,median:times[Math.floor(times.length*.5)],p95:times[Math.floor(times.length*.95)],worst:times.at(-1),
  before,after,beforeAudit,afterAudit,canvas:[1280,720],dpr:devicePixelRatio,gpu:info?gl.getParameter(info.UNMASKED_RENDERER_WEBGL):gl.getParameter(gl.RENDERER)};
 return report;
}
export async function draws(targetFrames=120){
 const beforeAudit=assertReady(),before=read();const prototype=WebGL2RenderingContext.prototype,names=['drawArrays','drawElements','drawArraysInstanced','drawElementsInstanced'];
 const counts=Object.fromEntries(names.map(name=>[name,{calls:0,instances:0}])),saved={};
 const originalFrame=window.gamePerformanceFrame;let sampleFrames=0;
 window.gamePerformanceFrame=function(...args){sampleFrames++;return originalFrame.apply(this,args);};
 for(const name of names){saved[name]=prototype[name];prototype[name]=function(...args){counts[name].calls++;counts[name].instances+=name.endsWith('Instanced')?args.at(-1):1;return saved[name].apply(this,args);};}
 try {
  await next();for(const value of Object.values(counts)){value.calls=0;value.instances=0;}sampleFrames=0;
  await wait(()=>sampleFrames>=targetFrames);
  const afterAudit=assertReady(),after=read();
  if(after.totalSteps<=before.totalSteps||afterAudit.graph.views[0].frame<=beforeAudit.graph.views[0].frame)throw Error('Draw sample requires advancing physics and render submissions');
  return {sampleFrames,before,after,beforeAudit,afterAudit,counts,perFrame:Object.fromEntries(Object.entries(counts).map(([name,value])=>[name,{calls:value.calls/sampleFrames,instances:value.instances/sampleFrames}]))};
 }
 finally {window.gamePerformanceFrame=originalFrame;for(const name of names)prototype[name]=saved[name];}
}
export function cleanup(){autoResume=false;held=pressed=released=mouseX=mouseY=actions=0;installed=false;for(const [key,value]of Object.entries(original))window[key]=value;for(const [node,display]of overlays.splice(0))node.style.display=display;}
