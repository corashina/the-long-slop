// One context and fixed voice slots. Queue/voice eviction is a bounded browser adapter.
export function createAudio({bindings: reference,onError=()=>{}}) {
 let context,master,session=0,epoch=0,paused=true,playing=false,desiredGain=1,disposed=false,disabled=false;
 const errors=new Set(),buffers=new Map(),loads=new Map(),routes=new Map(),voices=[];
 const origin=[0,0,0],listener=[0,0,0],listenerAbsolute=[0,0,0],listenerVelocity=[0,0,0];
 let previousSeconds=null,frameSeconds=0,frameDelta=0,motionRevision=0;
 function clearMotion(){previousSeconds=null;frameDelta=0;listenerVelocity.fill(0);for(const v of voices){v.vx=0;v.vy=0;v.vz=0;}}
 function report(key,error){if(errors.has(key))return;errors.add(key);try{onError(error);}catch{}}
 try {
  if(!reference || reference.real_voices!==128 || reference.queue_capacity!==512 || !Array.isArray(reference.clips) || !Array.isArray(reference.bindings))throw Error('Invalid audio contract');
  if(reference.doppler?.speed_of_sound!==340||reference.doppler.minimum_pitch!==1e-6||reference.doppler.factor!==1||reference.doppler.velocity_mode!=='transform_delta')throw Error('Invalid Doppler contract');
  for(const b of reference.bindings){if(routes.has(b.kind)||!b.clips.length||!b.clips.every(id=>reference.clips[id]?.id===id)||![b.min_distance,b.max_distance,b.mixer_gain,b.mixer_pitch,...b.pitch,...b.gain].every(Number.isFinite))throw Error('Invalid audio binding');routes.set(b.kind,b);}
 }catch(error){disabled=true;report('schema',error);}
 function stop(v){v.token++;if(v.source){try{v.source.onended=null;v.source.stop();v.source.disconnect();}catch{}v.source=null;}v.active=false;v.loading=false;}
 function reset(next){if(next<session)return;session=next;epoch++;clearMotion();for(const v of voices)stop(v);}
 function position(v){
  const x=v.x-origin[0],y=v.y-origin[1],z=v.z-origin[2];
  v.panner.positionX.value=x;v.panner.positionY.value=y;v.panner.positionZ.value=z;
  const b=v.binding,d=Math.hypot(x-listener[0],y-listener[1],z-listener[2]);
  let distanceGain=1;
  if(b.spatial_blend && d>b.min_distance*v.distanceScale){
   const min=b.min_distance*v.distanceScale,max=b.max_distance*v.distanceScale;
   if(b.rolloff_mode===0)distanceGain=min/Math.min(Math.max(d,min),max);
   else if(b.rolloff_mode===1)distanceGain=1-Math.min(1,(d-min)/(max-min));
   else distanceGain=curve(b.rolloff_curve,Math.min(1,d/max));
  }
  v.gain.gain.value=v.volume*b.mixer_gain*distanceGain;
  // UnityPlayer 2021.3.18f1, AudioSource::CacheFrameParameters, RVA 0x9a5840.
  const radial=d>0?((v.vx-listenerVelocity[0])*(x-listener[0])+(v.vy-listenerVelocity[1])*(y-listener[1])+(v.vz-listenerVelocity[2])*(z-listener[2]))/d:0;
  const rule=reference.doppler;
  const shift=Math.max(rule.minimum_pitch,(rule.speed_of_sound-radial*rule.factor*b.doppler)/rule.speed_of_sound);
  v.rate=v.pitch*b.mixer_pitch*(1-b.spatial_blend+b.spatial_blend*shift);
  if(v.source)v.source.playbackRate.value=v.rate;
 }
 function curve(keys,t){
  if(t<=keys[0].time)return keys[0].value;
  for(let i=1;i<keys.length;i++){const a=keys[i-1],b=keys[i];if(t>b.time)continue;const span=b.time-a.time,u=(t-a.time)/span,u2=u*u,u3=u2*u;return (2*u3-3*u2+1)*a.value+(u3-2*u2+u)*span*a.outSlope+(-2*u3+3*u2)*b.value+(u3-u2)*span*b.inSlope;}
  return keys[keys.length-1].value;
 }
 async function load(clip){
  if(buffers.has(clip.id))return buffers.get(clip.id);
  if(loads.has(clip.id))return loads.get(clip.id);
  const promise=(async()=>{try{
   const response=await fetch('./assets/'+clip.path);if(!response.ok)throw Error('Audio HTTP '+response.status+': '+clip.path);
   const data=await response.arrayBuffer(),digest=await crypto.subtle.digest('SHA-256',data);
   if(Array.from(new Uint8Array(digest),v=>v.toString(16).padStart(2,'0')).join('')!==clip.sha256)throw Error('Audio hash differs: '+clip.path);
   const buffer=await context.decodeAudioData(data);
   if(buffer.numberOfChannels!==clip.channels || Math.abs(buffer.duration-clip.duration_seconds)>2/buffer.sampleRate)throw Error('Audio metadata differs: '+clip.path);
   if(!disposed)buffers.set(clip.id,buffer);return buffer;
  }catch(error){report('clip:'+clip.id,error);return null;}})();
  loads.set(clip.id,promise);return promise;
 }
 function resumeFromGesture(){
  if(disabled||disposed)return;
  try{
   if(!context){
    context=new AudioContext();master=context.createGain();master.gain.value=0;master.connect(context.destination);
    for(let i=0;i<reference.real_voices;i++){
     const gain=context.createGain(),panner=context.createPanner();
     // Distance gain belongs to the recovered curve, not the browser's default rolloff.
     panner.rolloffFactor=0;panner.panningModel='equalpower';panner.connect(gain);gain.connect(master);
     voices.push({gain,panner,source:null,active:false,loading:false,token:0,started:0,x:0,y:0,z:0,vx:0,vy:0,vz:0,sampleX:0,sampleY:0,sampleZ:0,sampleSeconds:0});
    }
    for(const clip of reference.clips)void load(clip);
   }
   paused=false;void context.resume().catch(error=>{disabled=true;report('context',error);});
  }catch(error){disabled=true;report('context',error);}
 }
 function play(v,buffer,token,ownerEpoch){
  if(!buffer||!v.active||v.token!==token||ownerEpoch!==epoch||paused||disposed)return;
  try{
   const source=context.createBufferSource();v.source=source;source.buffer=buffer;source.loop=v.binding.looped;
   source.playbackRate.value=v.rate;
   source.connect(v.binding.spatial_blend?v.panner:v.gain);
   source.onended=()=>{if(v.source===source){source.disconnect();v.source=null;v.active=false;}};
   source.start();if(v.binding.clip_length_lifetime)source.stop(context.currentTime+reference.clips[v.clip].duration_seconds);
  }catch(error){stop(v);report('play:'+v.clip,error);}
 }
 function publish(ids,values){
  if(disabled||disposed||!context)return;
  if(ids.length%5 || values.length!==ids.length){report('event',Error('Invalid audio event fields'));return;}
  try{for(let i=0;i<ids.length;i+=5){
   if(ids[i]!==session)continue;
   const emitter=ids[i+1],kind=ids[i+2],clip=ids[i+4],binding=routes.get(kind);let action=ids[i+3];
   if(!binding||!binding.clips.includes(clip)||!Number.isFinite(values[i])||!Number.isFinite(values[i+1])||values[i+1]<=0)continue;
   let voice=voices.find(v=>v.active&&v.emitter===emitter&&v.kind===kind);
   if(action===1&&!voice&&binding.looped)action=0;
   if(action===2){if(voice)stop(voice);continue;}
   if(paused)continue;
   if(action===1){if(!voice)continue;}
   else if(action===0){
    if(voice && (binding.looped||kind!==0)){if(binding.looped&&voice.clip===clip){}else{stop(voice);voice=null;}}
    else voice=null;
    if(!voice){
     voice=voices.find(v=>!v.active);
     if(!voice){voice=voices.reduce((a,b)=>a.binding.priority>b.binding.priority?a:a.binding.priority===b.binding.priority&&a.started<b.started?a:b);if(voice.binding.priority<binding.priority)continue;stop(voice);}
     Object.assign(voice,{active:true,emitter,kind,clip,binding,started:context.currentTime,vx:0,vy:0,vz:0,sampleSeconds:frameSeconds});
    }
   } else continue;
   voice.volume=values[i]>1?1:Math.max(0,values[i]);voice.distanceScale=values[i]>1?values[i]-1:1;voice.pitch=values[i+1];
   voice.x=values[i+2]+origin[0];voice.y=values[i+3]+origin[1];voice.z=values[i+4]+origin[2];
   if(frameDelta>0&&voice.sampleSeconds<frameSeconds){voice.vx=(voice.x-voice.sampleX)/frameDelta;voice.vy=(voice.y-voice.sampleY)/frameDelta;voice.vz=(voice.z-voice.sampleZ)/frameDelta;}
   position(voice);
   if(!voice.source&&action===0){const buffer=buffers.get(clip);if(buffer)play(voice,buffer,voice.token,epoch);else if(binding.looped&&!voice.loading){voice.loading=true;const token=voice.token,ownerEpoch=epoch;void load(reference.clips[clip]).then(b=>{if(voice.token===token)voice.loading=false;play(voice,b,token,ownerEpoch);});}else if(!binding.looped)stop(voice);}
  }}catch(error){report('publish',error);}
 }
 function setListener(next,revision,x,y,z,fx,fy,fz,ux,uy,uz,masterGain,ox,oy,oz,seconds){
  if(next<session)return;if(next!==session)reset(next);
  if(revision!==motionRevision){motionRevision=revision;clearMotion();}
  frameSeconds=seconds;frameDelta=previousSeconds!==null&&seconds>previousSeconds&&playing&&!paused?seconds-previousSeconds:0;
  const ax=x+ox,ay=y+oy,az=z+oz;
  listenerVelocity[0]=frameDelta>0?(ax-listenerAbsolute[0])/frameDelta:0;
  listenerVelocity[1]=frameDelta>0?(ay-listenerAbsolute[1])/frameDelta:0;
  listenerVelocity[2]=frameDelta>0?(az-listenerAbsolute[2])/frameDelta:0;
  listenerAbsolute[0]=ax;listenerAbsolute[1]=ay;listenerAbsolute[2]=az;
  previousSeconds=playing&&!paused?seconds:null;
  for(const v of voices)if(v.active){v.sampleX=v.x;v.sampleY=v.y;v.sampleZ=v.z;v.sampleSeconds=seconds-frameDelta;v.vx=0;v.vy=0;v.vz=0;}
  origin[0]=ox;origin[1]=oy;origin[2]=oz;listener[0]=x;listener[1]=y;listener[2]=z;
  if(!context||disabled)return;
  try{
   const l=context.listener;l.positionX.value=x;l.positionY.value=y;l.positionZ.value=z;l.forwardX.value=fx;l.forwardY.value=fy;l.forwardZ.value=fz;l.upX.value=ux;l.upY.value=uy;l.upZ.value=uz;
   desiredGain=masterGain;master.gain.value=paused||!playing?0:masterGain;
   for(const v of voices)if(v.active)position(v);
  }catch(error){report('listener',error);}
 }
 function setPlaying(value){playing=value;if(!value)clearMotion();if(master)master.gain.value=paused||!playing?0:desiredGain;}
 function pause(){if(paused)return;paused=true;playing=false;epoch++;clearMotion();if(!context)return;master.gain.value=0;for(const v of voices)if(!v.source)stop(v);void context.suspend().catch(error=>report('suspend',error));}
 function disable(error){disabled=true;pause();for(const v of voices)stop(v);report('schema',error);}
 function dispose(){if(disposed)return;disposed=true;epoch++;for(const v of voices){stop(v);v.panner.disconnect();v.gain.disconnect();}buffers.clear();loads.clear();if(context)void context.close().catch(error=>report('close',error));}
 return {resumeFromGesture,publish,setListener,setPlaying,pause,reset,dispose,disable};
}
