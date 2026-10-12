const MAX_FILE_BYTES=32*1024*1024;

async function checkedBytes(response){
 const reader=response.body.getReader(),chunks=[];
 let length=0;
 try{
  for(;;){
   const {done,value}=await reader.read();if(done)break;
   length+=value.byteLength;
   if(length>MAX_FILE_BYTES)throw Error('World dependency exceeds 32 MiB.');
   chunks.push(value);
  }
 }finally{await reader.cancel();reader.releaseLock();}
 const bytes=new Uint8Array(length);let offset=0;
 for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}
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
    const digest=await crypto.subtle.digest('SHA-256',bytes);
    const hash=Array.from(new Uint8Array(digest),v=>v.toString(16).padStart(2,'0')).join('');
    if(hash!==entry.sha256)throw Error('SHA-256 mismatch');
   }catch(error){throw Error('Cannot load dependencies for '+entry.owner+': '+entry.file+' ('+error.message+').');}
  }
 }
 try{await Promise.all([worker(),worker()]);}finally{controller.abort();}
}
