import {ClipCache} from './audio/clips.js';
import {VoiceBank} from './audio/voices.js';
export const sourceGainDb=value=>20*Math.log(Math.max(.001,Math.min(1,value)));
export class AudioController {
 constructor(raw){this.raw=raw;this.session=1;this.sequence=0;this.context=null;this.bank=null;this.cache=null;this.mixers=new Map();this.disabled=false;this.paused=true;this.started=false;this.disposed=false;this.epoch=0;this.diagnostics=new Set();this.unlocking=false;this.awaitingPlaying=false;this.accepted=true;try{if(raw.schema_version!==1||!raw.clips.length||!raw.voices.length||!raw.bindings.length||!raw.mixers.length||!raw.shared.settings)throw Error('Invalid audio data');this.bindings=new Map(raw.bindings.map(b=>[b.id,b]));this.engineConstants=new Map(Object.entries(raw.engine.constants));this.radioCurrent=[null,null];this.radioNext=[null,null];}catch(error){this.fail(error);}}
 fail(error){if(this.disposed)return;this.disabled=true;this.paused=true;try{this.bank?.pause();this.context?.suspend().catch(()=>{});}catch{}const message=error?.message??String(error);if(!this.diagnostics.has(message)){this.diagnostics.add(message);console.warn('Audio disabled: '+message);}}
 clipFailure(id,error){if(this.disposed)return;const message='Audio clip '+id+' disabled: '+(error?.message??String(error));if(!this.diagnostics.has(message)){this.diagnostics.add(message);console.warn(message);}}
 sourceFailure(error,s){if(this.disposed)return;if(this.context?.state==='closed'){this.fail(error);return;}this.bank.disable(s);const kind=s.event?'binding':'voice',id=s.event?s.binding:s.record.id,message='Audio '+kind+' '+id+' disabled: '+(error?.message??String(error));if(!this.diagnostics.has(message)){this.diagnostics.add(message);console.warn(message);}}
 initialize(){this.context=new AudioContext();const ctx=this.context;for(const m of this.raw.mixers){const gain=ctx.createGain();gain.gain.value=m.mute?0:10**(m.gain_db/20);this.mixers.set(m.id,gain);}for(const m of this.raw.mixers){let node=this.mixers.get(m.id);if(!m.bypass_effects)for(const effect of m.effects){if(effect.type!==10||effect.bypass)continue;const eq=ctx.createBiquadFilter();eq.type='peaking';eq.frequency.value=effect.parameters[0];eq.Q.value=1/effect.parameters[1];eq.gain.value=40*Math.log10(effect.parameters[2]);node.connect(eq);node=eq;}node.connect(m.parent?this.mixers.get(m.parent):ctx.destination);}this.cache=new ClipCache(ctx,this.raw,()=>this.session,(id,error)=>this.clipFailure(id,error));this.bank=new VoiceBank(ctx,this.raw,this.cache,this.mixers,()=>this.session,()=>this.started&&!this.paused&&!this.disabled&&!this.disposed,(error,slot)=>slot?this.sourceFailure(error,slot):this.fail(error));}
 unlock(){if(this.disposed||this.disabled||this.unlocking)return;const epoch=this.epoch;try{if(!this.context)this.initialize();this.unlocking=true;this.awaitingPlaying=true;const pending=this.context.resume();Promise.resolve(pending).then(()=>{if(epoch!==this.epoch||this.disposed)return;this.unlocking=false;this.paused=false;this.bank.resume();}).catch(error=>{if(epoch===this.epoch){this.unlocking=false;this.fail(error);}});}catch(error){this.unlocking=false;this.fail(error);}}
 begin(session,started,paused,gameSeconds){this.accepted=session>=this.session;if(!this.accepted)return;if(session!==this.session)this.restart(session);this.started=started;this.gameSeconds=gameSeconds;if(started&&!paused)this.awaitingPlaying=false;else if(paused&&!this.awaitingPlaying)this.pause();this.bank?.begin();}
 originShift(x,y,z){if(!this.accepted)return;this.bank?.originShift(x,y,z);}
 listener(position,rotation){if(!this.accepted)return;this.bank?.listener(position,rotation);}
 emitter(id,position,rotation,alive){if(!this.accepted)return;this.bank?.emitter(id,position,rotation,alive);if(!alive&&id===this.raw.radio?.emitter&&this.bank){for(const voice of this.raw.radio.voices)this.bank.stop(this.bank.voices.get(voice),false);this.radioCurrent.fill(null);this.radioNext.fill(null);this.cache.radio(this.radioCurrent,this.radioNext,this.session);}}
 voice(id,playing,gain,pitch,looping){try{if(this.accepted&&!this.disabled)this.bank?.update(id,playing,gain,pitch,looping);}catch(error){this.fail(error);}}
 starter(emitter,enabled,simulationSeconds){try{if(this.accepted&&!this.disabled)this.bank?.starter(emitter,enabled,simulationSeconds);}catch(error){this.fail(error);}}
 event(session,sequence,binding,emitter,x,y,z,gain,pitch,timeSeconds){if(session!==this.session||sequence<=this.sequence)return;this.sequence=sequence;if(this.paused||this.disabled||!this.started)return;const record=this.bindings?.get(binding);if(!record)return;try{this.bank?.event(record,emitter,x,y,z,gain,pitch);}catch(error){this.fail(error);}}
 mixer(id,sourceDb){const mixer=this.mixers.get(id);if(mixer)mixer.gain.value=10**(sourceDb/20);}
 route(id,mixer){this.bank?.route(id,mixer);}
 radio(frame){
  if(!this.accepted)return;
  this.radioFrame=frame;
  if(!this.bank||this.disabled||this.disposed)return;
  const r=this.raw.radio,alive=this.bank.emitters.get(r.emitter)?.alive;
  const playing=frame.power&&alive&&this.started&&!this.paused;
  this.bank.update(r.voices[0],playing,frame.staticGain,1,true);
  for(let i=0;i<2;i++){
   const p=i===0?frame.primary:frame.secondary,s=this.bank.voices.get(r.voices[i+1]);
   const clip=playing?p.clip:null;
   if(s.clip!==clip){this.bank.stop(s,false);s.clip=clip;}
   s.seen=this.bank.frame;s.playing=playing&&clip!==null&&!this.bank.failedVoices.has(s.record.id);s.looping=false;s.volume=p.gain;s.pitch=1;
   this.radioCurrent[i]=s.playing?clip:null;this.radioNext[i]=s.playing?p.nextClip:null;
   if(!s.playing)this.bank.stop(s,false);
   else this.bank.parameters(s);
  }
  this.cache.radio(this.radioCurrent,this.radioNext,this.session);
  for(let i=0;i<2;i++){
   const p=i===0?frame.primary:frame.secondary,s=this.bank.voices.get(r.voices[i+1]);
   if(!s.playing)continue;
   const buffer=this.cache.radioBuffers.get(s.clip);if(!buffer)continue;
   const offset=p.offsetSeconds<buffer.duration?p.offsetSeconds:Math.max(0,buffer.duration-r.late_seek_margin_seconds);
   if(s.source&&Math.abs(s.offset-offset)>r.time_correction_seconds){this.bank.stop(s,false);s.playing=true;}
   if(!s.source){s.buffer=buffer;s.offset=offset;this.bank.connect(s,this.bank.routes.get(s.record.id)??s.record.mixer);this.bank.start(s);}
  }
 }
 finish(){if(!this.accepted)return;try{this.bank?.finish();}catch(error){this.fail(error);}}
 pause(){this.awaitingPlaying=false;if(this.paused&&!this.unlocking)return;this.epoch++;this.unlocking=false;this.paused=true;this.bank?.pause();try{this.context?.suspend().catch(()=>{});}catch(error){this.fail(error);}}
 restart(session){this.epoch++;this.unlocking=false;this.session=session;this.sequence=0;this.awaitingPlaying=false;this.disabled=false;this.paused=true;this.diagnostics.clear();this.cache?.reset();this.bank?.reset();try{this.context?.suspend().catch(()=>{});}catch(error){this.fail(error);}if(!this.bindings)this.disabled=true;}
 dispose(){this.disposed=true;this.epoch++;this.cache?.dispose();this.bank?.dispose();try{this.context?.close().catch(()=>{});}catch{}this.mixers.clear();}
}
export function createAudio(raw){return new AudioController(raw);}
export function installAudioBridge(target,audio){const position=new Float32Array(3),rotation=new Float32Array(4),frame={power:false,am:false,frequency:0,volume:0,staticGain:0,primary:{clip:null,offsetSeconds:0,gain:0,nextClip:null},secondary:{clip:null,offsetSeconds:0,gain:0,nextClip:null}};let calendar=0,nextDay=0;target.gameAudioCalendar=()=>{const now=Date.now();if(now>=nextDay){const date=new Date(now);calendar=(date.getMonth()+1)*100+date.getDate();nextDay=new Date(date.getFullYear(),date.getMonth(),date.getDate()+1).getTime();}return calendar;};target.gameAudioBegin=(s,started,paused,t)=>audio.begin(s,started,paused,t);target.gameAudioOriginShift=(x,y,z)=>audio.originShift(x,y,z);target.gameAudioListener=(x,y,z,qx,qy,qz,qw)=>{position[0]=x;position[1]=y;position[2]=z;rotation[0]=qx;rotation[1]=qy;rotation[2]=qz;rotation[3]=qw;audio.listener(position,rotation);};target.gameAudioEmitter=(id,x,y,z,qx,qy,qz,qw,alive)=>{position[0]=x;position[1]=y;position[2]=z;rotation[0]=qx;rotation[1]=qy;rotation[2]=qz;rotation[3]=qw;audio.emitter(id,position,rotation,alive);};target.gameAudioVoice=(id,playing,gain,pitch,looping)=>audio.voice(id,playing,gain,pitch,looping);target.gameAudioStarter=(e,on,t)=>audio.starter(e,on,t);target.gameAudioEvent=(s,n,b,e,x,y,z,g,p,t)=>audio.event(s,n,b,e,x,y,z,g,p,t);target.gameAudioMixer=(id,db)=>audio.mixer(id,db);target.gameAudioRoute=(id,mixer)=>audio.route(id,mixer);target.gameAudioRadio=(power,am,frequency,volume,staticGain,pClip,pOffset,pGain,pNext,sClip,sOffset,sGain,sNext)=>{frame.power=power;frame.am=am;frame.frequency=frequency;frame.volume=volume;frame.staticGain=staticGain;frame.primary.clip=pClip||null;frame.primary.offsetSeconds=pOffset;frame.primary.gain=pGain;frame.primary.nextClip=pNext||null;frame.secondary.clip=sClip||null;frame.secondary.offsetSeconds=sOffset;frame.secondary.gain=sGain;frame.secondary.nextClip=sNext||null;audio.radio(frame);};target.gameAudioEnd=()=>audio.finish();}
