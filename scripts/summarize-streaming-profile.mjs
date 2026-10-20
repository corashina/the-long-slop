import {readFile,writeFile} from 'node:fs/promises';
import {createReadStream} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {contentIndexSampleFailures} from './measure-content-index.mjs';

const canonical=value=>JSON.stringify(value&&typeof value==='object'?Array.isArray(value)?value.map(v=>JSON.parse(canonical(v))):Object.fromEntries(Object.keys(value).sort().map(key=>[key,JSON.parse(canonical(value[key]))])):value);
function validate(receipt){
 if(receipt.schemaVersion!==2||!receipt.valid||receipt.failures?.length||contentIndexSampleFailures(receipt).length)throw Error('Comparison has an invalid sample');
 if(!receipt.configuration?.identity||canonical(receipt.configuration.identity)!==canonical(receipt.server?.identity))throw Error('Receipt package identity differs from server identity');
 for(const kind of ['web','scripts','packages'])if(!receipt.configuration.identity[kind]||!Object.keys(receipt.configuration.identity[kind]).length||!Object.values(receipt.configuration.identity[kind]).every(hash=>/^[a-f0-9]{64}$/.test(hash)))throw Error('Receipt identity is incomplete');
 if(!['cold','warm'].includes(receipt.cacheState)||!['none','cpu','streaming','webgl','browser-trace'].includes(receipt.instrumentationMode))throw Error('Comparison has invalid sample metadata');
 if(!/^[a-f0-9]{40}$/.test(receipt.configuration.commit??'')||!/^[a-f0-9]{64}$/.test(receipt.configuration.manifestSha256??'')||receipt.configuration.buildProfile!=='release')throw Error('Comparison has invalid build metadata');
 if(!Array.isArray(receipt.canvas)||receipt.canvas.length!==2||!receipt.canvas.every(n=>Number.isFinite(n)&&n>0)||!(receipt.dpr>0)||!receipt.gpu||!receipt.browser||!receipt.route||receipt.route==='unspecified'||!Number.isInteger(receipt.seed))throw Error('Comparison has invalid scenario metadata');
 const mode=receipt.profileCpu?'cpu':receipt.profileStreaming?'streaming':receipt.instrument?'webgl':receipt.traceRecording||receipt.browserTrace?'browser-trace':'none';
 if(mode!==receipt.instrumentationMode||receipt.performanceSample!==(receipt.valid&&mode==='none'))throw Error('Comparison has mismatched instrumentation metadata');
}

export async function attachBrowserTrace(receiptPath,tracePath){
 const receipt=JSON.parse(await readFile(receiptPath,'utf8'));
 if(receipt.schemaVersion!==2)throw Error('Unsupported receipt schema');
 const path=resolve(tracePath),hash=createHash('sha256');
 for await(const chunk of createReadStream(path))hash.update(chunk);
 receipt.browserTrace={path,sha256:hash.digest('hex'),kind:'chromium-trace',coverageVerified:false};
 receipt.traceRecording=true;receipt.performanceSample=false;
 if(receipt.instrumentationMode==='none')receipt.instrumentationMode='browser-trace';
 await writeFile(receiptPath,JSON.stringify(receipt,null,2)+'\n');
 return receipt.browserTrace;
}
function metrics(receipt){
 const out={};
 const add=(prefix,row)=>{for(const name of ['meanMs','p95Ms','p99Ms','maxMs']){if(!Number.isFinite(row?.[name])||row[name]<0)throw Error(`Missing or invalid metric: ${prefix}.${name}`);out[`${prefix}.${name}`]=row[name];}};
 if(receipt.scenario.startsWith('startup-')){
  const phases=Object.entries(receipt.startupTrace.phases??{});
  if(!phases.length)throw Error('Missing startup metric');
  for(const [name,row] of phases){if(!Number.isFinite(row.durationMs)||row.durationMs<0)throw Error('Invalid startup metric');out[`startup.${name}.durationMs`]=row.durationMs;}
 }else add('frame',receipt.intervals);
 if(receipt.profileCpu){
  if(!receipt.cpuProfile?.browser||!receipt.cpuProfile?.rust)throw Error('Missing CPU profile metric');
  for(const [stage,row] of Object.entries(receipt.cpuProfile.browser.stages))if(row.count)add(`browser.${stage}`,row);
  for(const [name,row] of Object.entries(receipt.cpuProfile.browser.inputLatencies))if(row.count)add(`input.${name}`,row);
  const rust=receipt.cpuProfile.rust;
  rust.stages.forEach((stage,index)=>{const row=rust.measurements[index];if(row.calls)add(`rust.${stage}`,{meanMs:row.mean_ms,p95Ms:row.p95_ms,p99Ms:row.p99_ms,maxMs:row.max_ms});});
 }
 const stream=receipt.profileCpu?receipt.cpuProfile.rust.streaming:receipt.profileStreaming?receipt.streamingProfile:null;
 if((receipt.profileCpu||receipt.profileStreaming)&&(!stream||stream.stages.length!==stream.measurements.length))throw Error('Missing streaming metric');
 stream?.stages.forEach((stage,index)=>{const row=stream.measurements[index];if(row.calls)add(`stream.${stage}`,{meanMs:row.mean_ms,p95Ms:row.p95_ms,p99Ms:row.p99_ms,maxMs:row.max_ms});});
 return out;
}
function workerCosts(receipt){
 const groups=new Map(),excluded=[];
 for(const row of receipt.cpuProfile?.browser?.workers??[]){
  const group=row.phase?`${row.kind}.${row.phase}`:row.kind;
  let fields=groups.get(group);if(!fields){fields=new Map();groups.set(group,fields);}
  for(const [prefix,record] of [['main',row],['worker',row.worker??{}]])for(const [name,value] of Object.entries(record)){
   if(!name.endsWith('Ms'))continue;
   const diagnostic=prefix==='main'&&name==='submitDeliveryMs'?row.timing?.submitDelivery:prefix==='main'&&name==='returnDeliveryMs'?row.timing?.returnDelivery:typeof value==='object'&&value!==null?value:row.timing?.[name]??null;
   const duration=typeof value==='object'&&value!==null?value.valueMs:value;
   const crossContext=prefix==='main'&&['submitDeliveryMs','returnDeliveryMs'].includes(name);
   const reason=crossContext&&diagnostic?.valid!==true?diagnostic?.reason??'missing-clock-diagnostic':diagnostic?.valid===false?diagnostic.reason??'invalid-duration':!Number.isFinite(duration)?'unavailable-or-non-finite-duration':duration<0?'negative-duration':null;
   if(reason){excluded.push({kind:row.kind,id:row.id??null,phase:row.phase??null,metric:`${prefix}.${name}`,reason,rawValue:typeof value==='number'&&!Number.isFinite(value)?String(value):value,diagnostic});continue;}
   const key=`${prefix}.${name}`;let values=fields.get(key);if(!values){values=[];fields.set(key,values);}values.push(duration);
  }
 }
 const values=Object.fromEntries([...groups].map(([kind,fields])=>[kind,Object.fromEntries([...fields].map(([name,values])=>{
  values.sort((a,b)=>a-b);return [name,{count:values.length,meanMs:values.reduce((a,b)=>a+b,0)/values.length,p95Ms:values[Math.ceil(values.length*.95)-1],p99Ms:values[Math.ceil(values.length*.99)-1],maxMs:values.at(-1)}];
 }))]));
 return {values,excluded};
}

export function summarizeWorkerComparison(baseline,candidate,recheck){
 for(const receipt of [baseline,candidate,recheck])validate(receipt);
 if(canonical(baseline.configuration.identity)!==canonical(recheck.configuration.identity))throw Error('Baseline recheck package identity mismatch');
 for(const key of ['scenario','cacheState','instrumentationMode'])if(baseline[key]!==candidate[key]||baseline[key]!==recheck[key])throw Error(`Comparison ${key} mismatch`);
 for(const key of ['manifestSha256','buildProfile'])if(baseline.configuration[key]!==candidate.configuration[key]||baseline.configuration[key]!==recheck.configuration[key])throw Error(`Comparison ${key} mismatch`);
 for(const key of ['canvas','dpr','gpu','browser','route','seed'])if(canonical(baseline[key]??null)!==canonical(candidate[key]??null)||canonical(baseline[key]??null)!==canonical(recheck[key]??null))throw Error(`Comparison ${key} mismatch`);
 const a=metrics(baseline),b=metrics(candidate),c=metrics(recheck);
 if(canonical(Object.keys(a).sort())!==canonical(Object.keys(b).sort())||canonical(Object.keys(a).sort())!==canonical(Object.keys(c).sort()))throw Error('Comparison metric mismatch');
 const comparison=Object.fromEntries(Object.keys(a).map(name=>[name,{baseline:a[name],candidate:b[name],recheck:c[name],variation:Math.abs(a[name]-c[name]),saving:a[name]-b[name],regression:b[name]-a[name],withinVariation:b[name]-a[name]<=Math.abs(a[name]-c[name])}]));
 const workers=[baseline,candidate,recheck].map(workerCosts);
 return {scenario:baseline.scenario,performanceSample:baseline.performanceSample&&candidate.performanceSample&&recheck.performanceSample,metrics:comparison,nestedStages:baseline.cpuProfile?.rust?.streaming?.nested_stages??baseline.streamingProfile?.nested_stages??[],workerCosts:workers.map(row=>row.values),workerTimingExcluded:workers.map(row=>row.excluded),workerRecords:[baseline,candidate,recheck].map(r=>r.cpuProfile?.browser?.workers??[]),qualification:'Review main-thread savings, transfer costs, input trust, startup, and memory before selecting a worker. Excluded worker timings cannot support a gain claim.'};
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 const paths=process.argv.slice(2);
 if(paths[0]==='--attach-trace'&&paths.length===3)console.log(JSON.stringify(await attachBrowserTrace(paths[1],paths[2]),null,2));
 else {
  if(paths.length!==3)throw Error('Pass baseline, candidate, and baseline recheck receipts.');
  const receipts=await Promise.all(paths.map(async path=>JSON.parse(await readFile(path,'utf8'))));
  console.log(JSON.stringify(summarizeWorkerComparison(...receipts),null,2));
 }
}
