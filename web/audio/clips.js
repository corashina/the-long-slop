// Accepted WAV bytes are checked before browser decoding.
export class ClipCache {
 constructor(context,raw,current,fail){this.context=context;this.clips=new Map(raw.clips.map(c=>[c.id,c]));this.current=current;this.fail=fail;this.buffers=new Map();this.pending=new Map();this.controllers=new Set();this.generation=0;this.radioBuffers=new Map();this.radioPending=new Map();this.radioWanted=new Set();this.radioDemand=[null,null,null,null];this.radioDirty=true;}
 async effect(id,session){
  if(this.buffers.has(id))return this.buffers.get(id);
  if(this.pending.has(id))return this.pending.get(id);
  const generation=this.generation,abort=new AbortController();this.controllers.add(abort);
  const task=(async()=>{try{
   const buffer=await this.decode(id,session,'effect',generation,abort);
   if(buffer)this.buffers.set(id,buffer);return buffer;
  }catch(error){if(generation===this.generation&&session===this.current()&&error.name!=='AbortError')this.fail(error);return null;}
  finally{this.controllers.delete(abort);if(generation===this.generation)this.pending.delete(id);}})();
  this.pending.set(id,task);return task;
 }
 async decode(id,session,usage,generation,abort){
   const clip=this.clips.get(id);if(!clip||!clip.usages.includes(usage))throw Error('Invalid '+usage+' clip');
   const response=await fetch('./assets/'+clip.path,{signal:abort.signal});if(!response.ok)throw Error('Audio HTTP '+response.status);
   const bytes=await response.arrayBuffer();if(generation!==this.generation||session!==this.current())return null;
   if(bytes.byteLength!==clip.bytes)throw Error('Audio byte count');
   const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));let hash='';for(const value of digest)hash+=value.toString(16).padStart(2,'0');
   if(hash!==clip.sha256)throw Error('Audio hash');
   const view=new DataView(bytes);if(view.getUint32(0,true)!==0x46464952||view.getUint32(8,true)!==0x45564157)throw Error('Audio WAV header');
   let format=false,data=false;for(let offset=12;offset+8<=bytes.byteLength;){const size=view.getUint32(offset+4,true),end=offset+8+size;if(end>bytes.byteLength)throw Error('Audio WAV chunk');if(view.getUint32(offset,true)===0x20746d66){if(size<16||view.getUint16(offset+8,true)!==clip.format_tag||view.getUint16(offset+10,true)!==clip.channels||view.getUint32(offset+12,true)!==clip.frequency||view.getUint16(offset+22,true)!==clip.bits_per_sample)throw Error('Audio WAV format');format=true;}if(view.getUint32(offset,true)===0x61746164){if(size!==clip.data_bytes)throw Error('Audio WAV samples');data=true;}offset=end+(size&1);}
   if(!format||!data)throw Error('Audio WAV data');
   const buffer=await this.context.decodeAudioData(bytes);
   if(generation!==this.generation||session!==this.current())return null;
   if(Math.abs(buffer.duration-clip.duration_seconds)>.02||buffer.numberOfChannels!==clip.channels)throw Error('Audio decoded samples');
   return buffer;
 }
 radio(current,next,session){
  if(session!==this.current())return;
  const demand=this.radioDemand;const changed=demand[0]!==current[0]||demand[1]!==current[1]||demand[2]!==next[0]||demand[3]!==next[1];
  if(!changed&&!this.radioDirty)return;this.radioDirty=false;
  if(changed){demand[0]=current[0];demand[1]=current[1];demand[2]=next[0];demand[3]=next[1];this.radioWanted.clear();for(const id of current)if(id!==null)this.radioWanted.add(id);for(const id of next)if(id!==null)this.radioWanted.add(id);}
  for(const id of this.radioBuffers.keys())if(!this.radioWanted.has(id))this.radioBuffers.delete(id);
  for(const entry of this.radioPending.values())if(!this.radioWanted.has(entry.id)){entry.stale=true;entry.abort.abort();}
  for(const id of this.radioWanted){
   if(this.radioBuffers.has(id)||this.radioPending.has(id))continue;
   if(this.radioBuffers.size+this.radioPending.size>=4)break;
   const generation=this.generation,abort=new AbortController(),entry={id,abort,stale:false,task:null};
   this.radioPending.set(id,entry);
   entry.task=this.decode(id,session,'radio',generation,abort).then(buffer=>{
    if(buffer&&!entry.stale&&generation===this.generation&&session===this.current()&&this.radioWanted.has(id)){this.radioPending.delete(id);this.radioBuffers.set(id,buffer);}
   }).catch(error=>{if(!entry.stale&&generation===this.generation&&session===this.current()&&error.name!=='AbortError')this.fail(error);}).finally(()=>{if(this.radioPending.get(id)===entry)this.radioPending.delete(id);this.radioDirty=true;});
  }
 }
 reset(){this.generation++;for(const controller of this.controllers)controller.abort();this.controllers.clear();this.pending.clear();this.radioWanted.clear();this.radioDemand.fill(null);this.radioDirty=true;this.radioBuffers.clear();for(const entry of this.radioPending.values()){entry.stale=true;entry.abort.abort();}}
 dispose(){this.reset();this.buffers.clear();}
}
