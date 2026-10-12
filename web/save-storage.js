const DATABASE='tld-browser-save',VERSION=1,STORE='saves',KEY='manual';
const encoder=new TextEncoder();
function aborted(){return new DOMException('Save operation canceled.','AbortError');}
function checkSignal(signal){if(signal?.aborted)throw aborted();}
async function sha256(bytes,crypto){
 if(!crypto?.subtle)throw new Error('Save checksum is unavailable.');
 const hash=await crypto.subtle.digest('SHA-256',bytes);
 return Array.from(new Uint8Array(hash),byte=>byte.toString(16).padStart(2,'0')).join('');
}
export async function manifestStamp(manifestBytes,reference,crypto=globalThis.crypto){
 return {app_id:reference.app_id,build_id:reference.build_id,manifest_sha256:await sha256(manifestBytes,crypto),source_hashes:reference.source_hashes.map(({file,sha256})=>({file,sha256}))};
}
export async function verifyRecord(record,crypto=globalThis.crypto){
 if(!record||record.version!==VERSION||!record.payload||typeof record.payload!=='object'||Array.isArray(record.payload)||typeof record.checksum!=='string')throw new Error('Invalid save record.');
 const json=JSON.stringify(record.payload),checksum=record.checksum;
 if(await sha256(encoder.encode(json),crypto)!==checksum)throw new Error('Save checksum does not match.');
 return JSON.parse(json);
}
export function createSaveStorage({indexedDB=globalThis.indexedDB,crypto=globalThis.crypto}={}){
 let connection=null,opening=null,closed=false;
 const transactions=new Set();
 function open(){
  if(closed)return Promise.reject(new Error('Save storage is closed.'));
  if(!indexedDB)return Promise.reject(new Error('Save storage is unavailable.'));
  if(connection)return Promise.resolve(connection);
  if(opening)return opening.promise;
  const state={settled:false};
  state.promise=new Promise((resolve,reject)=>{
   const fail=error=>{if(state.settled)return;state.settled=true;if(opening===state)opening=null;reject(error);};
   state.fail=fail;
   let request;
   try{request=indexedDB.open(DATABASE,VERSION);}catch(error){fail(error);return;}
   request.onblocked=()=>fail(new Error('Save database access is blocked. Close the other tab.'));
   request.onerror=()=>fail(request.error??new Error('Cannot open the save database.'));
   request.onupgradeneeded=event=>{
    if(state.settled||closed){request.transaction.abort();return;}
    if(event.oldVersion!==0){request.transaction.abort();fail(new Error('Incompatible save database version.'));return;}
    request.result.createObjectStore(STORE);
   };
   request.onsuccess=()=>{
    const db=request.result;
    if(state.settled||closed){db.close();return;}
    if(db.version!==VERSION||!db.objectStoreNames.contains(STORE)){db.close();fail(new Error('Incompatible save database version or store.'));return;}
    state.settled=true;opening=null;connection=db;
    db.onversionchange=()=>{db.close();if(connection===db)connection=null;};
    resolve(db);
   };
  });
  if(!state.settled)opening=state;
  return state.promise;
 }
 function transaction(db,mode,record,signal){
  checkSignal(signal);
  return new Promise((resolve,reject)=>{
   let tx,result=null,requestError;
   try{tx=mode==='readwrite'?db.transaction(STORE,mode,{durability:'strict'}):db.transaction(STORE,mode);}catch(error){reject(error);return;}
   transactions.add(tx);
   const cancel=()=>{try{tx.abort();}catch{/* A completed transaction stays committed. */}};
   const cleanup=()=>{transactions.delete(tx);signal?.removeEventListener('abort',cancel);};
   tx.oncomplete=()=>{cleanup();resolve(result);};
   tx.onabort=()=>{cleanup();reject(tx.error??requestError??aborted());};
   tx.onerror=event=>{requestError=event.target?.error??tx.error;};
   signal?.addEventListener('abort',cancel,{once:true});
   try{
    const store=tx.objectStore(STORE);
    const request=mode==='readwrite'?store.put(record,KEY):store.get(KEY);
    request.onsuccess=()=>{result=request.result??null;};
    request.onerror=()=>{requestError=request.error;};
   }catch(error){requestError=error;cancel();}
  });
 }
 return {
  async read(){return transaction(await open(),'readonly');},
  async write(payload,{signal}={}){
   checkSignal(signal);
   if(!payload||typeof payload!=='object'||Array.isArray(payload))throw new Error('Invalid save payload.');
   // Detach the stored object before hashing. Keep Rust field order in both copies.
   const json=JSON.stringify(payload);
   if(typeof json!=='string')throw new Error('Invalid save payload.');
   const frozen=JSON.parse(json);
   if(!frozen||typeof frozen!=='object'||Array.isArray(frozen))throw new Error('Invalid save payload.');
   const record={version:VERSION,payload:frozen,checksum:await sha256(encoder.encode(json),crypto)};
   checkSignal(signal);
   const db=await open();
   checkSignal(signal);
   await transaction(db,'readwrite',record,signal);
  },
  close(){
   closed=true;opening?.fail(new Error('Save storage is closed.'));
   for(const tx of transactions){try{tx.abort();}catch{}}
   connection?.close();connection=null;
  }
 };
}
