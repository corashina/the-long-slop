// Web Audio player for the source sound frame. Rust decides every voice and one-shot.
// Frame: listener position and rotation, outside gain, car gain, voice count, one-shot count,
// then eleven floats per sound: clip, volume, pitch, x, y, z, min distance, max distance, spatial, bus, play.
// Play is zero for a loop. A new play number starts the clip once from its start.
export const HEADER=11,SOUND=11,MAX_ONE_SHOTS=32;
// Long ambient loops stream from media elements so the page does not hold their decoded samples.
const STREAM_SECONDS=30;

export function rotate(q,v){
 const [x,y,z,w]=q,tx=2*(y*v[2]-z*v[1]),ty=2*(z*v[0]-x*v[2]),tz=2*(x*v[1]-y*v[0]);
 return [v[0]+w*tx+y*tz-z*ty,v[1]+w*ty+z*tx-x*tz,v[2]+w*tz+x*ty-y*tx];
}

export function createAudio(audio){
 const context=new AudioContext(),master=context.createGain(),world=context.createGain();
 world.connect(master);master.connect(context.destination);world.gain.value=0;
 const buses=[0,1,2].map(()=>{const g=context.createGain();g.connect(world);return g;});
 const buffers=[],streams=[],media=[];let oneShots=0,playing=false;
 audio.clips.forEach((clip,i)=>{
  const url='./assets/'+clip.path;
  if(clip.seconds>STREAM_SECONDS){const element=new Audio(url);element.loop=true;element.preservesPitch=false;streams[i]=element;media[i]=context.createMediaElementSource(element);return;}
  fetch(url).then(r=>{if(!r.ok)throw Error('Cannot load sound '+clip.name);return r.arrayBuffer();})
   .then(data=>context.decodeAudioData(data)).then(buffer=>{buffers[i]=buffer;}).catch(error=>console.warn(error.message));
 });
 function route(spatial,min,max,bus){
  const gain=context.createGain();
  if(spatial>=0.5){const panner=new PannerNode(context,{panningModel:'equalpower',distanceModel:'inverse',refDistance:min,maxDistance:max,rolloffFactor:1});gain.connect(panner);panner.connect(buses[bus]);return {gain,panner};}
  gain.connect(buses[bus]);return {gain,panner:null};
 }
 function place(panner,f,o){if(panner){panner.positionX.value=f[o+3];panner.positionY.value=f[o+4];panner.positionZ.value=f[o+5];}}
 const voices=[];
 function voice(index,f,o){
  const clip=f[o],volume=f[o+1],play=f[o+10],key=clip+':'+f[o+8]+':'+f[o+9]+':'+play;let v=voices[index];
  if(v&&(v.key!==key||volume<=0)){stop(v);voices[index]=v=null;}
  if(!v&&volume>0&&clip>=0){
   const nodes=route(f[o+8],f[o+6],f[o+7],f[o+9]);v={key,...nodes,source:null,element:null};
   if(streams[clip]){v.element=streams[clip];v.media=media[clip];v.media.connect(v.gain);v.element.play().catch(()=>{});}
   else if(buffers[clip]){const s=context.createBufferSource();s.buffer=buffers[clip];s.loop=play===0;s.connect(v.gain);s.start(0,s.loop?Math.random()*s.buffer.duration:0);v.source=s;}
   else return;
   voices[index]=v;
  }
  if(!v)return;
  v.gain.gain.value=volume;
  if(v.source)v.source.playbackRate.value=f[o+2];else v.element.playbackRate=f[o+2];
  place(v.panner,f,o);
 }
 function stop(v){if(v.source)v.source.stop();if(v.element){v.element.pause();v.media.disconnect(v.gain);}v.gain.disconnect();v.panner?.disconnect();}
 function oneShot(f,o){
  const buffer=buffers[f[o]];if(!buffer||oneShots>=MAX_ONE_SHOTS)return;
  const nodes=route(f[o+8],f[o+6],f[o+7],f[o+9]),s=context.createBufferSource();
  s.buffer=buffer;s.playbackRate.value=f[o+2];nodes.gain.gain.value=f[o+1];place(nodes.panner,f,o);
  s.connect(nodes.gain);oneShots++;s.onended=()=>{oneShots--;nodes.gain.disconnect();nodes.panner?.disconnect();};s.start();
 }
 function frame(f){
  const l=context.listener,forward=rotate([f[3],f[4],f[5],f[6]],[0,0,-1]),up=rotate([f[3],f[4],f[5],f[6]],[0,1,0]);
  l.positionX.value=f[0];l.positionY.value=f[1];l.positionZ.value=f[2];
  l.forwardX.value=forward[0];l.forwardY.value=forward[1];l.forwardZ.value=forward[2];l.upX.value=up[0];l.upY.value=up[1];l.upZ.value=up[2];
  buses[1].gain.value=f[7];buses[2].gain.value=f[8];
  const count=f[9],shots=f[10];
  for(let i=0;i<count;i++)voice(i,f,HEADER+i*SOUND);
  for(let i=0;i<shots;i++)oneShot(f,HEADER+(count+i)*SOUND);
 }
 // buttonSoundMain: touch sounds on hover, button sounds on press and release.
 const menu=audio.menu;
 function button(clips,volume){
  const buffer=buffers[clips[Math.floor(Math.random()*clips.length)]];if(!buffer)return;
  const s=context.createBufferSource(),g=context.createGain();s.buffer=buffer;
  s.playbackRate.value=menu.pitch[0]+Math.random()*(menu.pitch[1]-menu.pitch[0]);g.gain.value=volume;s.connect(g);g.connect(master);s.start();
 }
 function bindButtons(root){
  for(const node of root.querySelectorAll('button')){
   node.addEventListener('pointerenter',()=>{if(!node.disabled)button(menu.touch_down,menu.touch_volume);});
   node.addEventListener('pointerleave',()=>{if(!node.disabled)button(menu.touch_up,menu.touch_volume);});
   node.addEventListener('pointerdown',()=>{if(!node.disabled)button(menu.down,menu.volume);});
   node.addEventListener('pointerup',()=>{if(!node.disabled)button(menu.up,menu.volume);});
  }
 }
 return {
  frame,bindButtons,
  unlock(){if(context.state!=='running')context.resume().catch(()=>{});},
  // The world is silent outside play. Menu sounds keep playing.
  setPlaying(value){
   if(value===playing)return;playing=value;world.gain.value=playing?1:0;
   for(const v of voices)if(v?.element){if(playing)v.element.play().catch(()=>{});else v.element.pause();}
  },
 };
}
