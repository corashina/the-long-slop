const MAX_FILE_BYTES=32*1024*1024;
export const worldBootstrapMeasurements={dependencyWorkers:1,maxDependencyBytes:0,dependencyPayloadCopyBytes:0};

async function checkedBytes(response,maxBytes=MAX_FILE_BYTES){
 const reader=response.body.getReader(),chunks=[];
 let length=0;
 try{
  for(;;){
   const {done,value}=await reader.read();if(done)break;
   length+=value.byteLength;
   if(length>maxBytes)throw Error('World dependency exceeds 32 MiB.');
   chunks.push(value);
  }
 }finally{await reader.cancel();reader.releaseLock();}
 const bytes=new Uint8Array(length);let offset=0;
 for(let i=0;i<chunks.length;i++){const chunk=chunks[i];bytes.set(chunk,offset);offset+=chunk.byteLength;chunks[i]=null;}
 chunks.length=0;
 return bytes;
}

// Check catalog bytes without decoding meshes or textures into resident assets.
export async function admitWorldAssets(manifest){
 if(!manifest.world||!manifest.world.prototypes?.length)throw Error('Missing required world contract.');
 const files=new Map();
 for(const asset of manifest.assets.filter(asset=>asset.usage==='world_prototype')){
  for(const entry of [{file:asset.path,sha256:asset.sha256},...asset.dependencies]){
   if(!/^reference\/[a-zA-Z0-9_.-]+$/.test(entry.file)||!/^[a-f0-9]{64}$/.test(entry.sha256))throw Error('Invalid world dependency receipt.');
   const previous=files.get(entry.file);
   if(previous&&previous.sha256!==entry.sha256)throw Error('Conflicting world dependency receipts.');
   files.set(entry.file,{...entry,owner:previous?.owner??asset.path});
  }
 }
 if(!files.size)throw Error('Missing required world prototype assets.');
 const pending=files.values(),controller=new AbortController();
 async function worker(){
  for(let next=pending.next();!next.done;next=pending.next()){
   const entry=next.value;
   try{
    const response=await fetch('./assets/'+entry.file,{signal:controller.signal});
    if(!response.ok)throw Error('HTTP '+response.status);
    const bytes=await checkedBytes(response);
    worldBootstrapMeasurements.maxDependencyBytes=Math.max(worldBootstrapMeasurements.maxDependencyBytes,bytes.byteLength);
    worldBootstrapMeasurements.dependencyPayloadCopyBytes=2*worldBootstrapMeasurements.maxDependencyBytes;
    const digest=await crypto.subtle.digest('SHA-256',bytes);
    const hash=Array.from(new Uint8Array(digest),v=>v.toString(16).padStart(2,'0')).join('');
    if(hash!==entry.sha256)throw Error('SHA-256 mismatch');
   }catch(error){throw Error('Cannot load dependencies for '+entry.owner+': '+entry.file+' ('+error.message+').');}
  }
 }
 try{await worker();}finally{controller.abort();}
}

export async function verifyWorldBytes(bytes,expected){
 const digest=await crypto.subtle.digest('SHA-256',bytes);
 const actual=Array.from(new Uint8Array(digest),v=>v.toString(16).padStart(2,'0')).join('');
 if(actual!==expected)throw Error('Source asset SHA-256 mismatch.');
}
const prototypeRequests=new Map();
export function cancelWorldPrototype(key){prototypeRequests.get(key)?.abort();}
// Called only after the source stream reserves this selected prototype's staging budget.
export async function preflightWorldPrototype(path,sha256,budget,key){
 const controller=new AbortController();prototypeRequests.set(key,controller);
 try{
 if(!/^reference\/[a-zA-Z0-9_.-]+\.gltf$/.test(path)||!/^[a-f0-9]{64}$/.test(sha256))throw Error('Invalid prototype receipt.');
 const response=await fetch('./assets/'+path,{signal:controller.signal});if(!response.ok)throw Error('Prototype HTTP '+response.status);
 const bytes=await checkedBytes(response,Math.min(MAX_FILE_BYTES,Math.floor(budget/18)));
 // Input chunks, contiguous input, decoded text, parser nodes, and returned metadata overlap.
 if(bytes.byteLength*18>budget)throw Error('Prototype metadata exceeds its staging reservation.');
 const digest=await crypto.subtle.digest('SHA-256',bytes);
 const hash=Array.from(new Uint8Array(digest),v=>v.toString(16).padStart(2,'0')).join('');
 if(hash!==sha256)throw Error('Prototype SHA-256 mismatch.');
 const gltf=JSON.parse(new TextDecoder().decode(bytes));
 const accessors=gltf.accessors??[];let meshBytes=0,bufferBytes=0;
 const primitives=[];
 for(const buffer of gltf.buffers??[]){
  if(!Number.isSafeInteger(buffer.byteLength)||buffer.byteLength<0)throw Error('Invalid prototype buffer size.');
  bufferBytes+=buffer.byteLength;
 }
 for(const [meshIndex,mesh] of (gltf.meshes??[]).entries())for(const [primitiveIndex,primitive] of mesh.primitives.entries()){
  if((primitive.mode??4)!==4)throw Error('Unsupported source primitive topology.');
  const attributes=primitive.attributes,position=accessors[attributes.POSITION];
  if(!position||!Number.isSafeInteger(position.count))throw Error('Missing prototype positions.');
  const count=primitive.indices===undefined?position.count:accessors[primitive.indices].count;
  const vertices=position.count;
  let stride=0;
  for(const [name,accessor] of Object.entries(attributes)){
   const width={SCALAR:1,VEC2:2,VEC3:3,VEC4:4,MAT4:16}[accessors[accessor].type];
   if(!width)throw Error('Unsupported source vertex attribute.');
   stride+=width*4;
  }
  // Bevy mesh, GPU upload, vertex packing and index widening retain independent allocations.
  meshBytes+=vertices*stride+count*4;
  const identity=primitive.extras?.world_source_material;
  const material=primitive.material===undefined?null:gltf.materials[primitive.material].extras;
  if(identity===undefined&&material===undefined)throw Error('Missing source material identity.');
  primitives.push({mesh:meshIndex,primitive:primitiveIndex,vertices,indices:count,bytes:vertices*stride+count*4,
   attributes:Object.keys(attributes),source_material:identity??null,has_source_material:Object.hasOwn(primitive.extras??{},'world_source_material'),material:material??null});
 }
 const nodes=(gltf.nodes??[]).map((node,index)=>({index,mesh:node.mesh??null,children:node.children??[],
  skin:node.skin??null,matrix:node.matrix??null,translation:node.translation??[0,0,0],rotation:node.rotation??[0,0,0,1],scale:node.scale??[1,1,1],extras:node.extras??null}));
 const skinBytes=(gltf.skins??[]).reduce((bytes,skin)=>bytes+skin.joints.length*128,0);
 const sceneBytes=nodes.length*2048+primitives.length*2048+skinBytes;
 return JSON.stringify({json_bytes:bytes.byteLength,buffer_bytes:bufferBytes,mesh_bytes:meshBytes,scene_bytes:sceneBytes,nodes,primitives});
 }finally{prototypeRequests.delete(key);}
}
