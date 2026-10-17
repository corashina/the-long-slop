const WINDOW_MS=2000,CAPACITY=4096;

export class FrameTimings {
 constructor(){
  this.times=new Float64Array(CAPACITY);this.intervals=new Float64Array(CAPACITY);
  this.physics=new Float64Array(CAPACITY);this.world=new Float64Array(CAPACITY);this.reset();
 }
 reset(){this.head=0;this.count=0;this.previous=null;}
 sample(now,physics,world){
  const previous=this.previous;this.previous=now;
  if(previous===null||now<=previous)return;
  const i=this.head;
  this.times[i]=now;this.intervals[i]=now-previous;this.physics[i]=physics;this.world[i]=world;
  this.head=(i+1)%CAPACITY;this.count=Math.min(this.count+1,CAPACITY);
  while(this.count&&this.times[(this.head-this.count+CAPACITY)%CAPACITY]<now-WINDOW_MS)this.count--;
 }
 summary(){
  if(!this.count)return null;
  let elapsed=0,worst=0,physics=0,world=0;
  for(let n=0;n<this.count;n++){
   const i=(this.head-this.count+n+CAPACITY)%CAPACITY;
   elapsed+=this.intervals[i];worst=Math.max(worst,this.intervals[i]);physics+=this.physics[i];world+=this.world[i];
  }
  return {fps:1000*this.count/elapsed,average:elapsed/this.count,worst,physics:physics/this.count,world:world/this.count};
 }
}

export function createPerformance(readWasmBytes){
 const root=document.querySelector('#performance'),values=document.querySelector('#performance-values');
 const timings=new FrameTimings();let lastPaint=-Infinity,stopped=false;
 const ms=value=>value.toFixed(1),mib=bytes=>Number.isFinite(bytes)?(bytes/1048576).toFixed(1)+' MiB':'unavailable';
 function reset(){timings.reset();lastPaint=-Infinity;values.textContent='Waiting for game frames...';}
 const key=event=>{
  if(event.code!=='F3'||event.repeat||event.ctrlKey||event.altKey||event.metaKey)return;
  event.preventDefault();root.hidden=!root.hidden;lastPaint=-Infinity;
 };
 window.addEventListener('keydown',key);document.addEventListener('visibilitychange',reset);
 return {
  frame(now,physicsMs,worldMs){
   if(stopped||document.hidden)return false;
   timings.sample(now,physicsMs,worldMs);
   if(root.hidden||now-lastPaint<250)return false;
   lastPaint=now;return true;
  },
  counters(steps,wheelContacts,bodies,colliders,roads,terrain,objects,jobs,pending,cpuBytes,gpuBytes){
   const sample=timings.summary();
   values.textContent=(sample?
    'FPS: '+sample.fps.toFixed(0)+'\nFrame avg / worst: '+ms(sample.average)+' / '+ms(sample.worst)+' ms'+
    '\nPhysics / world CPU avg: '+ms(sample.physics)+' / '+ms(sample.world)+' ms':
    'FPS: --\nFrame avg / worst: --\nPhysics / world CPU avg: --')+
    '\nPhysics steps/frame: '+steps+'\nWheel contacts: '+wheelContacts+'\nBodies / colliders: '+bodies+' / '+colliders+
    '\nRoads / terrain tiles: '+roads+' / '+terrain+'\nActive generated objects: '+objects+'\nJobs / queued work: '+jobs+' / '+pending+
    '\nWASM: '+mib(readWasmBytes())+'\nJS heap: '+mib(performance.memory?.usedJSHeapSize)+
    '\nWorld render CPU reserve: '+mib(cpuBytes)+'\nWorld render GPU reserve: '+mib(gpuBytes);
  },
  stop(){stopped=true;timings.reset();values.textContent='Performance counters stopped.';document.removeEventListener('visibilitychange',reset);},
 };
}
