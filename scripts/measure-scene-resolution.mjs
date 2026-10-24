// Run in a focused game tab after New game and real input checks.
import {measureContentIndex} from './measure-content-index.mjs';

export async function measureSceneResolution({scale=1,durationMs=15000,instrument=false,label='scene',scenario='garage'}={}){
 if(![1,.75].includes(scale))throw Error('Use scale 1 or 0.75');
 if(!window.gameState.started)throw Error('Start New game and verify real input before measuring');
 const control=document.querySelector('#scene-resolution');
 control.value=String(scale);control.dispatchEvent(new Event('change'));
 const {source_render_audit}=await import('/pkg/the_long_slop.js');
 const canvas=document.querySelector('#game');
 const expected=[canvas.width,canvas.height].map(side=>Math.max(1,Math.floor(side*scale)));
 const start=performance.now();
 let readyFrames=0;
 while(readyFrames<10){
  await new Promise(resolve=>requestAnimationFrame(resolve));
  const audit=JSON.parse(source_render_audit());
  if(audit.state.startsWith('Failed'))throw Error(JSON.stringify(audit));
  readyFrames=audit.state==='Ready'&&audit.graph.extent.every((side,i)=>side===expected[i])&&audit.graph.presented?readyFrames+1:0;
  if(performance.now()-start>60000)throw Error('Scene resolution did not settle');
 }
 return measureContentIndex({durationMs,instrument,label,scenario,sceneScale:scale});
}
