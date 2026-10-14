const STALL_MS=120000;
export function createLoadingGuard({progress,fail,clock={now:()=>performance.now(),setTimeout:(fn,ms)=>setTimeout(fn,ms),clearTimeout:id=>clearTimeout(id)}}){
 let timer=null,revision=0,deadline=0;
 function stop(){if(timer!==null)clock.clearTimeout(timer);timer=null;}
 function check(){
  const current=progress(),now=clock.now();
  if(current>revision){revision=current;deadline=now+STALL_MS;}
  if(now>=deadline){stop();fail('Loading made no progress for 120 seconds.');return;}
  timer=clock.setTimeout(check,1000);
 }
 return {
  start(){stop();revision=progress();deadline=clock.now()+STALL_MS;timer=clock.setTimeout(check,1000);},
  stop,
 };
}
