import {readFile} from 'node:fs/promises';

const paths=process.argv.slice(2);
if(!paths.length)throw Error('Pass one or more streaming profile receipts.');
for(const path of paths){
 const receipt=JSON.parse(await readFile(path,'utf8'));
 const profile=receipt.streamingProfile;
 if(!receipt.profileStreaming||!profile||profile.stages.length!==profile.measurements.length)throw Error(`Invalid streaming profile: ${path}`);
 const rows=profile.stages.map((stage,index)=>({stage,...profile.measurements[index]})).filter(row=>row.calls);
 rows.sort((a,b)=>b.max_ms-a.max_ms);
 console.log(JSON.stringify({path,label:receipt.label,scenario:receipt.scenario,
  valid:receipt.valid,failures:receipt.failures,realInputObserved:receipt.realInputObserved,
  performanceSample:false,nestedStages:profile.nested_stages,
  stages:rows.map(row=>({...row,mean_ms:row.total_ms/row.calls}))},null,2));
}
