export function createSaveBridge(host=window){
 let sequence=0;
 const queue=[],pending=new Map(),committing=new Set();
 function request(kind,payload){
  const id=++sequence;
  return new Promise((resolve,reject)=>{pending.set(id,{kind,resolve,reject});queue.push({id,kind,payload});});
 }
 host.gameHasSaveRequest=()=>queue.length!==0;
 host.gameSaveRequest=()=>queue.length?JSON.stringify(queue.shift()):'';
 host.gameSaveResult=(id,kind,payload,error)=>{
  const operation=pending.get(id);
  if(!operation||operation.kind!==kind)return;
  pending.delete(id);
  if(error){operation.reject(new Error(error));return;}
  try{operation.resolve(payload?JSON.parse(payload):undefined);}catch(error){operation.reject(error);}
 };
 return {
  capture:()=>request('Capture',new Date().toISOString()),
  prepare:save=>request('Prepare',save),
  prepareNewGame:()=>request('PrepareNewGame'),
  commit(token){committing.add(token);return request('Commit',token).finally(()=>committing.delete(token));},
  discard(token){
   if(committing.has(token))return;
   const result=request('Discard',token);queue.unshift(queue.pop());
   // Discard is cleanup. It never changes the active session.
   result.catch(()=>{});
  }
 };
}
