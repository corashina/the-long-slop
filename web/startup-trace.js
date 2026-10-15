export function createStartupTrace(now){
 const startedAt=now(),phases={},fetches=[];
 let failure=null,completed=false,presentedFrames=0,inputReady=false,frozen=null;
 let buildIdentity=null;
 const open=new Map();
 const closeOpen=()=>{const end=now();for(const [stage,start] of open)phases[stage]={startMs:start-startedAt,endMs:end-startedAt,durationMs:end-start,incomplete:true};open.clear();};
 const copy=()=>structuredClone({startedAt,phases,fetches,
  requiredBytes:fetches.reduce((sum,item)=>sum+(item.requiredBytes??0),0),
  receivedBytes:fetches.reduce((sum,item)=>sum+(item.receivedBytes??0),0),
  downloadMs:fetches.reduce((sum,item)=>sum+(item.downloadMs??0),0),
  cache:{hit:fetches.filter(item=>item.cache==='hit').length,miss:fetches.filter(item=>item.cache==='miss').length,unknown:fetches.filter(item=>item.cache!=='hit'&&item.cache!=='miss').length},
  buildIdentity,failure,completed,presentedFrames,inputReady});
 return {
  begin(stage){if(!frozen&&!open.has(stage))open.set(stage,now());},
  end(stage){if(frozen)return;const start=open.get(stage);if(start===undefined)return;const end=now();phases[stage]={startMs:start-startedAt,endMs:end-startedAt,durationMs:end-start};open.delete(stage);},
  fetched(record){if(!frozen)fetches.push({...record});},
  reconcile(resources){
   if(frozen)return;
   const byPath=new Map();
   for(const resource of resources){
    const path=new URL(resource.name).pathname.slice(1);
    if(!byPath.has(path))byPath.set(path,[]);
    byPath.get(path).push(resource);
   }
   for(const item of fetches){
    const resource=byPath.get(item.path)?.shift();
    if(!resource)continue;
    item.cache=resource.transferSize===0?'hit':'miss';
    item.downloadMs=resource.responseEnd-resource.requestStart;
   }
  },
  build(identity){if(!frozen)buildIdentity={...buildIdentity,...identity};},
  fail(error){if(frozen)return;closeOpen();failure=typeof error==='string'?{message:error}:{...error};frozen=copy();},
  finish(result){if(frozen)return;if(result.presentedFrames<2||!result.inputReady)throw Error('Startup needs two presented frames and ready input.');closeOpen();presentedFrames=result.presentedFrames;inputReady=true;completed=true;frozen=copy();},
  snapshot(){return frozen?structuredClone(frozen):copy();},
 };
}
