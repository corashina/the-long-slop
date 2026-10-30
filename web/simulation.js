// The simulation runs in a Web Worker. One command is in flight; the renderer reads each frame once.
export function createSimulation(worker,onFailure){
 let frame=null,failed=false,starting=null,tickMs=0,ticks=0,wasmBytes=0,hooks={};
 const fail=message=>{if(failed)return;failed=true;frame=null;starting?.reject(Error(message));starting=null;onFailure(message);};
 worker.addEventListener('message',({data})=>{
  if(failed)return;
  if(data.error!==undefined){fail(data.error);return;}
  if(data.startup){starting?.resolve(data.startup);starting=null;return;}
  frame=data.frame;tickMs+=data.tickMs;ticks++;wasmBytes=data.wasmBytes;
  if(data.clearInput)hooks.clearInput?.();
  if(data.inputConsumed)hooks.inputConsumed?.();
 });
 worker.addEventListener('error',event=>fail(event.message||'The simulation worker stopped.'));
 worker.addEventListener('messageerror',()=>fail('The simulation worker message could not be read.'));
 return {
  // Hooks of the CPU profile: the worker reports input use with the frame.
  hooks(value){hooks=value;},
  profile(enabled){worker.postMessage({cpuProfile:Boolean(enabled)});},
  // `manifest` moves to the worker. Resolves with the encoded startup values.
  start(manifest,hash,drawDistance,biome,startCar){
   return new Promise((resolve,reject)=>{starting={resolve,reject};worker.postMessage({start:{manifest,hash,drawDistance,biome,startCar}},[manifest.buffer]);});
  },
  // The command is a view of WASM memory, so copy it before transfer.
  send(command){if(failed)return;const copy=command.slice();worker.postMessage({command:copy},[copy.buffer]);},
  frameLength:()=>frame?frame.length:-1,
  read(out){out.set(frame);frame=null;},
  // Average tick time since the last read, and the worker WASM memory.
  stats(){const value={tickMs:ticks?tickMs/ticks:null,wasmBytes};tickMs=0;ticks=0;return value;},
 };
}

export function installSimulationBridge(target,simulation){
 target.gameSimSend=simulation.send;
 target.gameSimFrameLength=simulation.frameLength;
 target.gameSimRead=simulation.read;
}
