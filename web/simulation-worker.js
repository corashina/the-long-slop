// Runs the simulation WASM. The terrain and road workers start here, next to the simulation that uses them.
import init,{start,tick,cpu_profile} from './pkg/simulation_worker.js';
import {createTerrain,installTerrainBridge} from './terrain.js';
import {installRoadBridge} from './road.js';
self.gameNow=()=>performance.now();
let inputConsumed=false,clearInput=false,memory;
self.gameCpuInputConsumed=()=>{inputConsumed=true;};
self.gameCpuClearInput=()=>{clearInput=true;};
installTerrainBridge(self,createTerrain(new Worker(new URL('./terrain-worker.js',import.meta.url),{type:'module'}),null));
installRoadBridge(self,null);
const ready=init().then(exports=>{memory=exports.memory;});
self.addEventListener('message',async({data})=>{
 try{
  await ready;
  if(data.cpuProfile!==undefined){cpu_profile(data.cpuProfile);return;}
  if(data.start){
   const {manifest,hash,drawDistance,biome,startCar,seed,options}=data.start;
   const startup=start(manifest,hash,drawDistance,biome,startCar,seed,options);
   self.postMessage({startup},[startup.buffer]);return;
  }
  const started=performance.now(),frame=tick(data.command),tickMs=performance.now()-started;
  self.postMessage({frame,tickMs,wasmBytes:memory.buffer.byteLength,inputConsumed,clearInput},[frame.buffer]);
  inputConsumed=clearInput=false;
 }catch(error){self.postMessage({error:error?.message??String(error)});}
});
