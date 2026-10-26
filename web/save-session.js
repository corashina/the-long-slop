import {defaultName,verifyRecord} from './save-storage.js';

export function createSaveSession({storage,runtime,drawDistance,onStatus=()=>{}}){
 let sequence=0,active=null,writes=Promise.resolve();
 const current=operation=>active===operation&&operation.id===sequence;
 async function run(name,work){
  if(active)throw new Error('A save operation is pending.');
  const operation={id:++sequence,name,controller:new AbortController(),candidate:null,committing:false};
  active=operation;
  let failed=false,failure;
  try{
   onStatus({operation:name,state:'pending'});
   await work(operation);
  }catch(error){
   if(operation.candidate!==null){runtime.discard(operation.candidate);operation.candidate=null;}
   if(current(operation)){failed=true;failure=error;}
  }finally{
   if(current(operation)){
    active=null;
    onStatus(failed?{operation:name,state:'error',error:failure}:{operation:name,state:'success'});
   }
  }
  if(failed)throw failure;
 }
 async function prepare(operation,build){
  const token=await build();
  if(!current(operation)){runtime.discard(token);return;}
  operation.candidate=token;
  await runtime.commit(token,()=>{operation.committing=true;});
  operation.candidate=null;
 }
 return {
  get pending(){return active!==null;},
  save(){return run('save',async operation=>{
   const {save,summary}=await runtime.capture();
   if(!current(operation))return;
   const name=defaultName(new Date()),entry={name,date:'',seed:summary.seed,drivenKm:summary.driven_km,version:summary.build_id,thumbnail:null};
   // Preserve ordering even when an adapter ignores cancellation during a delayed write.
   const write=writes.then(async()=>{
    if(current(operation))await storage.write(name,save,entry,{signal:operation.controller.signal});
   });
   writes=write.catch(()=>{});
   await write;
  });},
  load(){return run('load',async operation=>{
   const {last}=await storage.list();
   if(!current(operation))return;
   const record=last===null?null:await storage.read(last);
   if(!current(operation))return;
   if(record===null)throw new Error('No saved game.');
   const save=await verifyRecord(record);
   if(current(operation))await prepare(operation,()=>runtime.prepare(save,operation.controller.signal,drawDistance()));
  });},
  newGame(){return run('newGame',operation=>prepare(operation,()=>runtime.prepareNewGame(operation.controller.signal,drawDistance())));},
  cancel(){
   if(!active||active.committing)return false;
   const operation=active;active=null;sequence++;
   operation.controller.abort();
   if(operation.candidate!==null){runtime.discard(operation.candidate);operation.candidate=null;}
   onStatus({operation:operation.name,state:'canceled'});
   return true;
  }
 };
}
