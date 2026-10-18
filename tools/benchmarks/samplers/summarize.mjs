import {readFile,readdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';

const directory=resolve(process.argv[2]);
const reports=[];
const rejected=[];
for(const file of await readdir(directory)){
 if(!/^(baseline(?:-recheck)?|candidate)-(garage|driving)-(\d+|counts)\.json$/.test(file))continue;
 const report=JSON.parse(await readFile(resolve(directory,file),'utf8'));
 try{
 if(report.viewport[0]!==1280||report.viewport[1]!==720||report.dpr!==1)throw Error('Unmatched viewport: '+file);
 if(report.before.audit.state!=='Ready'||report.after.audit.state!=='Ready')throw Error('Renderer not Ready: '+file);
 if(!report.after.audit.graph.presented||report.after.audit.graph.token<=report.before.audit.graph.token)throw Error('No submitted frames: '+file);
 if(!report.renderCadence)throw Error('Browser callbacks outpaced render submissions: '+file);
 if(report.before.state.paused||report.after.state.paused||!report.after.state.started)throw Error('Gameplay not active: '+file);
 if(file.includes('-driving-')&&(!report.before.state.seated||!report.after.state.seated
  ||report.after.state.travelM-report.before.state.travelM<1))throw Error('No vehicle movement: '+file);
 reports.push(report);
 }catch(error){rejected.push({file,reason:error.message});}
}
const median=values=>values.toSorted((a,b)=>a-b)[Math.floor(values.length/2)];
const summary={performance:[],calls:[],rejected,missing:[]};
for(const scene of ['garage','driving']){
 for(const build of ['baseline','candidate','baseline-recheck']){
  if(build==='baseline-recheck'&&scene==='driving')continue;
  const samples=reports.filter(report=>report.name.startsWith(build+'-'+scene+'-')&&!report.instrumented);
  if(samples.length!==3)summary.missing.push(`Need three valid FPS samples for ${build} ${scene}; found ${samples.length}`);
  else summary.performance.push({build,scene,samples:samples.length,fps:median(samples.map(sample=>sample.fps)),
   meanMs:median(samples.map(sample=>sample.meanMs)),medianMs:median(samples.map(sample=>sample.medianMs)),
   p95Ms:median(samples.map(sample=>sample.p95Ms)),p99Ms:median(samples.map(sample=>sample.p99Ms))});
  if(build==='baseline-recheck')continue;
  const count=reports.find(report=>report.name===build+'-'+scene+'-counts');
  if(!count?.instrumented){summary.missing.push(`Need separate valid counts for ${build} ${scene}`);continue;}
  const submitted=count.after.audit.graph.token-count.before.audit.graph.token;
  const draws=['drawArrays','drawElements','drawArraysInstanced','drawElementsInstanced'].reduce((total,key)=>total+count.counts[key],0);
  summary.calls.push({build,scene,submitted,perFrame:Object.fromEntries(Object.entries(count.counts).map(([key,value])=>[key,value/submitted])),
   drawsPerFrame:draws/submitted,samplerBindingsPerDraw:count.counts.bindSampler/draws,
   repeatedSamplersPerFrame:count.repeatedSamplers/submitted});
 }
}
await writeFile(resolve(directory,'summary.json'),JSON.stringify(summary,null,2));
console.log(JSON.stringify(summary,null,2));
