import init,{evaluate} from './pkg/terrain_worker.js';
const ready=init();
self.addEventListener('message',async({data})=>{
 try{
  await ready;
  const surface=evaluate(data.input);
  const out={id:data.id,vertices:surface.vertices(),normals:surface.normals(),tangents:surface.tangents(),
   uv:surface.uv(),colors:surface.colors(),triangles:surface.triangles()};
  surface.free();
  self.postMessage(out,[out.vertices,out.normals,out.tangents,out.uv,out.colors,out.triangles].map(a=>a.buffer));
 }catch(error){self.postMessage({id:data.id,error:error?.message??String(error)});}
});
