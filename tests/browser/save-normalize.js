export function normalizeSnapshot(snapshot){
 const normalized=structuredClone(snapshot);delete normalized.saved_at;
 function sort(records,key){records?.sort((a,b)=>{const left=JSON.stringify(key(a)),right=JSON.stringify(key(b));return left<right?-1:left>right?1:0;});}
 sort(normalized.world?.records,record=>record.instance);
 sort(normalized.vehicles,record=>record.instance);sort(normalized.bodies,record=>record.identity);sort(normalized.items,record=>record.instance);
 sort(normalized.attachments?.parts,record=>record.slot);sort(normalized.attachments?.wheels,record=>record.slot);sort(normalized.attachments?.tires,record=>record.rim);
 sort(normalized.mechanisms?.controls,record=>record.source);sort(normalized.mechanisms?.openings,record=>record.source);
 sort(normalized.lighting?.controllers,record=>record.source);
 for(const controller of normalized.lighting?.controllers??[])sort(controller.switch_poses,record=>record.node);
 return normalized;
}
