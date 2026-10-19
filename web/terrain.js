// Terrain tile jobs run in a Web Worker with its own WASM. One job is in flight at a time.
export function createTerrain(worker){
 let next=0,job=null,result=null,error=null;
 worker.addEventListener('message',({data})=>{
  if(data.id!==job)return;
  job=null;
  if(data.error!==undefined)error=data.error;else result=data;
 });
 worker.addEventListener('error',event=>{error=event.message||'The terrain worker stopped.';});
 const ready=()=>{if(!result)throw Error('The terrain worker has no result.');return result;};
 return {
  // The input is a view of WASM memory, so copy it before transfer.
  submit(input){
   if(job!==null||result)throw Error('The terrain worker is busy.');
   next=next+1&0x7fffffff;job=next;
   const copy=input.slice();
   worker.postMessage({id:job,input:copy},[copy.buffer]);
   return job;
  },
  poll(){if(error!==null)throw Error(error);return result?result.id:job===null?-1:-2;},
  vertexCount:()=>ready().vertices.length/3,
  triangleCount:()=>ready().triangles.length/3,
  read(vertices,normals,tangents,uv,colors,triangles){
   const r=ready();
   vertices.set(r.vertices);normals.set(r.normals);tangents.set(r.tangents);
   uv.set(r.uv);colors.set(r.colors);triangles.set(r.triangles);
   result=null;
  },
  discard(){result=null;},
 };
}

export function installTerrainBridge(target,terrain){
 target.gameTerrainSubmit=terrain.submit;
 target.gameTerrainPoll=terrain.poll;
 target.gameTerrainVertexCount=terrain.vertexCount;
 target.gameTerrainTriangleCount=terrain.triangleCount;
 target.gameTerrainRead=terrain.read;
 target.gameTerrainDiscard=terrain.discard;
}
