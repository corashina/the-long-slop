import {setTimeout as delay} from 'node:timers/promises';

// Playwright waitForFunction treats a returned Promise as a truthy value.
export async function poll(test, timeout) {
 let stopped=false,timer;
 try {
  await Promise.race([
   (async()=>{while(!stopped){if(await test())return;await delay(100);}})(),
   new Promise((_,reject)=>{timer=setTimeout(()=>{stopped=true;reject(new Error('Benchmark condition timed out'));},timeout);}),
  ]);
 } finally {stopped=true;clearTimeout(timer);}
}
