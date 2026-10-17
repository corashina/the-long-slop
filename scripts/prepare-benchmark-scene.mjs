import {saveRuntime} from '/main.js';
import {source_render_audit} from '/pkg/my_summer_slop.js';
let action=0,held=0;
const names=['gameActions','gameKeys','gamePressed','gameReleased','gameMouseX','gameMouseY','gameWheel'];
const originals=Object.fromEntries(names.map(name=>[name,window[name]]));
window.gameActions=()=>{const result=originals.gameActions()|action;action=0;return result;};
window.gameKeys=()=>{originals.gameKeys();return held;};
for(const name of names.slice(2))window[name]=()=>0;
const next=()=>new Promise(resolve=>requestAnimationFrame(resolve));
async function wait(test){const start=performance.now();while(!test()){if(performance.now()-start>180000)throw Error('Benchmark scene readiness timed out');await next();}}
export async function loadBenchmarkScene(saved,keys=0){
 held=0;if(window.gameState.started&&!window.gameState.paused){action=2;await wait(()=>window.gameState.paused);}
 const token=await saveRuntime.prepare(saved);await saveRuntime.commit(token);
 await wait(()=>JSON.parse(source_render_audit())?.state==='Ready');
 held=keys;action=4;await wait(()=>!window.gameState.paused&&window.gameState.started);
 const start=performance.now();while(performance.now()-start<2000)await next();
 return {started:window.gameState.started,seated:window.gameState.seated,speedKmh:window.gameState.speedKmh};
}
export async function captureGarage(){return saveRuntime.capture();}
export function cleanupScene(){held=0;for(const name of names)window[name]=originals[name];}
