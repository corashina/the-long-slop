const MAX_FILE_BYTES=32*1024*1024;
const inputSizes=new Map(),prototypeStaging=new Map();
let dependencyReserve=0,inputMetadataBytes=0;
export function worldInputBytes(path){const bytes=inputSizes.get(path);if(!bytes)throw Error("Missing checked world input size: "+path);return bytes;}
export function worldPrototypeStaging(path){const bytes=prototypeStaging.get(path);if(!bytes)throw Error("Missing checked prototype size: "+path);return bytes;}
export function worldDependencyReserve(){if(!dependencyReserve)throw Error("Missing world dependency reservation.");return dependencyReserve;}
export function worldInputMetadataBytes(){return inputMetadataBytes;}
export const worldBootstrapMeasurements={};

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

// Read world input sizes from the manifest. The asset reader checks each file hash when it loads.
export function admitWorldAssets(manifest){
 if(!manifest.world||!manifest.world.prototypes?.length)throw Error('Missing required world contract.');
 const sizes=manifest.file_bytes,prototypeInputs=new Map();
 const size=file=>{const bytes=sizes[file];if(!Number.isSafeInteger(bytes)||bytes<=0)throw Error('Missing world file size: '+file);if(bytes>MAX_FILE_BYTES)throw Error('World file exceeds 32 MiB: '+file);return bytes;};
 const dynamic=new Set(manifest.assets.filter(a=>a.usage==='world_prototype'||a.usage==='world_material').map(a=>a.id));
 for(const asset of manifest.assets.filter(asset=>dynamic.has(asset.id))){
  for(const entry of [{file:asset.path},...asset.dependencies]){
   if(!/^reference\/[a-zA-Z0-9_.-]+$/.test(entry.file))throw Error('Invalid world dependency receipt.');
   inputSizes.set(entry.file,size(entry.file));
  }
  if(asset.usage==='world_prototype')prototypeInputs.set(asset.path,asset.dependencies.filter(d=>d.file.endsWith('.bin')).map(d=>d.file));
 }
 if(!inputSizes.size)throw Error('Missing required world prototype assets.');
 // One dependency can always advance while generation retains its own bounded share.
 dependencyReserve=5*(1024*1024+16);
 for(const [path,buffers] of prototypeInputs){
  let bytes=worldInputBytes(path);
  for(const buffer of buffers)bytes+=worldInputBytes(buffer);
  prototypeStaging.set(path,2*bytes);
  dependencyReserve=Math.max(dependencyReserve,2*bytes);
 }
 for(const material of manifest.rendering.materials.filter(m=>dynamic.has(m.asset)))for(const texture of material.properties.textures)for(const mip of texture.mips){
  // Fetch buffer, VecReader, loader input; bounded PNG decoder growth and scanlines.
  dependencyReserve=Math.max(dependencyReserve,3*worldInputBytes(mip.file)+2*1024*1024+128*mip.width);
 }
 if(dependencyReserve>=MAX_FILE_BYTES)throw Error('World input overlap leaves no generation capacity.');
 inputMetadataBytes=0;
 for(const path of inputSizes.keys())inputMetadataBytes+=256+path.length*2;
 for(const path of prototypeStaging.keys())inputMetadataBytes+=256+path.length*2;
 worldBootstrapMeasurements.inputMetadataBytes=inputMetadataBytes;
 worldBootstrapMeasurements.dependencyStagingReserve=dependencyReserve;
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
 const bytes=await checkedBytes(response,worldInputBytes(path));
 // Input chunks, contiguous input, decoded text, parser nodes, and returned metadata overlap.
 if(bytes.byteLength!==worldInputBytes(path)||bytes.byteLength*18>budget)throw Error('Prototype metadata exceeds its staging reservation.');
 // The glTF loader hashes these bytes before it uses them.
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
  const submesh=primitive.extras?.submesh;
  if(!Number.isSafeInteger(submesh)||submesh<0)throw Error('Missing source submesh identity.');
  primitives.push({mesh:meshIndex,primitive:primitiveIndex,submesh,vertices,indices:count,bytes:vertices*stride+count*4,
   attributes:Object.keys(attributes),source_material:identity??null,has_source_material:Object.hasOwn(primitive.extras??{},'world_source_material'),material:material??null});
 }
 const nodes=(gltf.nodes??[]).map((node,index)=>({index,mesh:node.mesh??null,children:node.children??[],
  skin:node.skin??null,matrix:node.matrix??null,translation:node.translation??[0,0,0],rotation:node.rotation??[0,0,0,1],scale:node.scale??[1,1,1],extras:node.extras??null}));
 meshBytes=nodes.reduce((total,node)=>total+primitives.filter(p=>p.mesh===node.mesh).reduce((sum,p)=>sum+p.bytes+p.vertices*32,0),0);
 const skinBytes=(gltf.skins??[]).reduce((bytes,skin)=>bytes+skin.joints.length*128,0);
 const sceneBytes=nodes.length*2048+primitives.length*2048+skinBytes;
 return JSON.stringify({json_bytes:bytes.byteLength,buffer_bytes:bufferBytes,mesh_bytes:meshBytes,scene_bytes:sceneBytes,nodes,primitives});
 }finally{prototypeRequests.delete(key);}
}
