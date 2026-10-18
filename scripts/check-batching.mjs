// Check saved benchmark receipts, including real submitted instance coverage.
// node scripts/check-batching.mjs REPORT.json [REPORT.json ...]
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
assert.ok(process.argv.length>2,'Provide at least one benchmark report');
for(const path of process.argv.slice(2)){
 const report=JSON.parse(await readFile(path,'utf8'));
 const a=report.afterAudit??report.audit;
 assert.equal(a.state,'Ready',`${path}: renderer failed`);
 let batched=0,instances=0,draws=0,hasInstanceData=false;
 for(const [index,view] of a.graph.views.entries()){
  if(!view.frame)continue;
  assert.deepEqual(view.shadow_attempts,view.shadow_issued,`${path}: incomplete shadow draws`);
  const current=view.frame===a.graph.gpu_boundary.shadow_scene_frame;
  if(current)draws+=view.shadow_issued.reduce((sum,value)=>sum+value,0);
  if(view.shadow_instances){
   hasInstanceData=true;
   assert.deepEqual(view.shadow_instances[0],view.shadow_instances[1],`${path}: incomplete shadow instances`);
   if(view.frame===a.graph.gpu_boundary.shadow_scene_frame){
    assert.deepEqual(view.shadow_instances[1],a.graph.gpu_boundary.shadow_split_counts[index],`${path}: changed source cascade membership`);
   }
   for(let i=0;i<4;i++){
    assert.ok(view.shadow_instances[1][i]>=view.shadow_issued[i],`${path}: invalid instance range`);
    if(current){instances+=view.shadow_instances[1][i];
     if(view.shadow_instances[1][i]>view.shadow_issued[i])batched++;}
   }
  }
  if(view.surface_instances){
   assert.equal(view.surface_instances[0],view.surface_instances[1],`${path}: incomplete surface instances`);
   assert.equal(view.surface_instances[2],view.surface_instances[3],`${path}: incomplete additive instances`);
  }
 }
 if(report.frames){
  assert.equal(report.canvas[0],1280);assert.equal(report.canvas[1],720);assert.equal(report.dpr,1);
  assert.ok(report.after.totalSteps>report.before.totalSteps,`${path}: physics did not advance`);
  assert.ok(report.afterAudit.graph.views[1].frame>report.beforeAudit.graph.views[1].frame,`${path}: main render submissions did not advance`);
  assert.equal(report.before.paused,false);assert.equal(report.after.paused,false);
 }
 console.log(JSON.stringify({path,fps:report.fps,mean:report.mean,p95:report.p95,currentShadowDraws:draws,currentShadowInstances:hasInstanceData?instances:null,batchedCascades:batched,glDrawsPerFrame:report.perFrame?Object.values(report.perFrame).reduce((sum,value)=>sum+value.calls,0):null}));
}
