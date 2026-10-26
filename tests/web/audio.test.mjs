import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {webcrypto} from 'node:crypto';
import {createAudio} from '../../web/audio.js';
import {configureAssetCatalog} from '../../web/world-assets.js';
import {MenuState,ACTIONS,captureWithAudio} from '../../web/menu.js';
import {spatialValues,VoiceBank} from '../../web/audio/voices.js';
import {ClipCache} from '../../web/audio/clips.js';
import {installAudioBridge} from '../../web/audio.js';
const manifest=JSON.parse(readFileSync(new URL('../../web/assets/reference/manifest.json',import.meta.url)));
const raw=manifest.audio;
const binding=raw.bindings.find(b=>b.id===raw.radio.click_binding);
const samples=readFileSync(new URL('../../web/assets/'+raw.clips.find(c=>c.id===binding.clips[0]).path,import.meta.url));
const param=()=>({value:0,setValueAtTime(v){this.value=v;}});
class Node {constructor(){this.gain=param();this.pan=param();this.frequency=param();this.Q=param();this.playbackRate=param();for(const n of ['positionX','positionY','positionZ','orientationX','orientationY','orientationZ'])this[n]=param();}connect(){}disconnect(){}start(t,o=0){this.offset=o;Fake.starts.push(this);}stop(time){if(time>0)this.stopTime=time;else this.stopped=true;}}
class Fake {static starts=[];static contexts=[];constructor(){this.currentTime=0;this.destination=new Node();this.filters=[];this.state='suspended';Fake.contexts.push(this);}createGain(){return new Node();}createPanner(){return new Node();}createStereoPanner(){return new Node();}createBiquadFilter(){const n=new Node();this.filters.push(n);return n;}createBufferSource(){return new Node();}resume(){this.state='running';return Promise.resolve();}suspend(){this.state='suspended';return Promise.resolve();}close(){return Promise.resolve();}decodeAudioData(){return Promise.resolve({duration:raw.clips.find(c=>c.id===binding.clips[0]).duration_seconds,numberOfChannels:1});}}
function setup(Context=Fake){configureAssetCatalog(manifest.content_base,manifest.file_catalog);Fake.starts=[];Fake.contexts=[];globalThis.AudioContext=Context;globalThis.fetch=async()=>new Response(samples);return createAudio(raw);}
const settle=()=>new Promise(r=>setTimeout(r,30));
async function loaded(audio){await Promise.all(audio.cache?.pending.values()??[]);await Promise.resolve();await Promise.resolve();}
async function decoding(ready){for(let i=0;i<100&&!ready();i++)await new Promise(r=>setTimeout(r,10));assert.ok(ready(),'decode started');}
function event(a,session=1,sequence=1){a.begin(session,true,false,0);a.emitter(binding.emitter,new Float32Array([0,0,0]),new Float32Array([0,0,0,1]),true);a.event(session,sequence,binding.id,binding.emitter,0,0,0,1,1,0);a.finish();}
test('audio_failure_keeps_start_working',async()=>{
 const warnings=[],originalWarn=console.warn;
 console.warn=message=>warnings.push(message);
 try{
  for(const mode of ['fetch','decode','resume','validation']){
   class Bad extends Fake {resume(){return mode==='resume'?Promise.reject(Error('resume')):super.resume();}decodeAudioData(){return mode==='decode'?Promise.reject(Error('decode')):super.decodeAudioData();}}
   const a=setup(Bad);
   if(mode==='fetch')globalThis.fetch=async()=>{throw Error('fetch');};
   if(mode==='validation')globalThis.fetch=async()=>new Response(new Uint8Array(2));
   const m=new MenuState();m.ready();m.started=true;m.pause();m.takeActions();await m.capture(ACTIONS.CONTINUE,async()=>{},()=>a.unlock());await settle();event(a);await loaded(a);
   assert.equal(m.state,'playing');assert.equal(Fake.starts.length,0);
   assert.equal(a.disabled,mode==='resume');
   event(a,1,2);await loaded(a);
   const detail=mode==='validation'?'Asset length differs: '+raw.clips.find(c=>c.id===binding.clips[0]).path:mode;
   assert.deepEqual(warnings,[mode==='resume'?'Audio disabled: resume':`Audio clip ${binding.clips[0]} disabled: ${detail}`]);
   warnings.length=0;a.dispose();
  }
  const a=createAudio({});assert.doesNotThrow(()=>a.unlock());
  assert.deepEqual(warnings,['Audio disabled: Invalid audio data']);a.dispose();
 }finally{console.warn=originalWarn;}
});
test('restart_rejects_pending_decode',async()=>{let complete;class Slow extends Fake {decodeAudioData(){return new Promise(r=>{complete=r;});}}const a=setup(Slow);a.unlock();await settle();event(a);await decoding(()=>complete);assert.ok(complete);a.restart(2);complete({duration:1});await settle();assert.equal(Fake.starts.length,0);a.dispose();});

test('removed_owner_rejects_pending_decode',async()=>{let complete;class Slow extends Fake {decodeAudioData(){return new Promise(r=>{complete=r;});}}const a=setup(Slow);a.unlock();await settle();event(a);await decoding(()=>complete);a.emitter(binding.emitter,[0,0,0],[0,0,0,1],false);complete({duration:raw.clips.find(c=>c.id===binding.clips[0]).duration_seconds,numberOfChannels:1});await settle();assert.equal(Fake.starts.length,0);a.dispose();});


function radioSamples(id){const clip=raw.clips.find(c=>c.id===id),bytes=readFileSync(new URL('../../web/assets/'+clip.path,import.meta.url));return bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength);}
function radioSetup(Context=Fake){const a=setup(Context);globalThis.fetch=async url=>{const clip=raw.clips.find(c=>manifest.content_base+c.path===url);return new Response(radioSamples(clip.id));};return a;}
const recording=raw.radio.stations[0].music[0];
function radioFrame(a,offset=0,clip=recording){a.begin(a.session,true,false,offset);a.emitter(raw.radio.emitter,[0,0,0],[0,0,0,1],true);for(const id of raw.radio.voices)a.emitter(raw.voices.find(v=>v.id===id).emitter,[0,0,0],[0,0,0,1],true);a.radio(raw.radio.emitter,{power:true,am:false,frequency:89.2,volume:.1,staticGain:0,primary:{clip,offsetSeconds:offset,gain:.1,nextClip:null},secondary:{clip:null,offsetSeconds:0,gain:0,nextClip:null}});a.finish();}
async function radioLoaded(a){await Promise.all([...a.cache.radioPending.values()].map(e=>e.task));await loaded(a);}
test('radio_resume_reuses_playback',async()=>{
 class Wav extends Fake {decodeAudioData(bytes){const v=new DataView(bytes),channels=v.getUint16(22,true),rate=v.getUint32(24,true),bits=v.getUint16(34,true);let data=0;for(let i=12;i+8<=bytes.byteLength;){const n=v.getUint32(i+4,true);if(v.getUint32(i,true)===0x61746164)data=n;i+=8+n+(n&1);}return Promise.resolve({duration:data/(rate*channels*bits/8),numberOfChannels:channels});}}
 const a=radioSetup(Wav);a.unlock();await settle();radioFrame(a,2);await radioLoaded(a);radioFrame(a,2);
 const voice=a.bank.voices.get(raw.radio.voices[1]),source=voice.source;
 assert.ok(source);assert.equal(voice.offset,2);
 a.pause();assert.equal(voice.source,null);assert.ok(source.stopped);
 a.unlock();await settle();radioFrame(a,2);await radioLoaded(a);
 const resumed=voice.source;assert.ok(resumed);assert.notEqual(resumed,source);assert.equal(voice.offset,2);
 radioFrame(a,2);a.unlock();await settle();radioFrame(a,2);
 assert.equal(voice.source,resumed);
 assert.equal(Fake.starts.filter(s=>!s.stopped&&s.buffer===resumed.buffer).length,1);
 a.dispose();
});
test('radio_cache_is_bounded',async()=>{
 const waiting=[],ids=raw.radio.stations.flatMap(s=>s.music).filter((id,i,a)=>a.indexOf(id)===i);
 class Slow extends Fake{decodeAudioData(bytes){return new Promise(resolve=>waiting.push({resolve,bytes}));}}
 const a=radioSetup(Slow);a.unlock();await settle();a.cache.radio(ids.slice(0,4),1);await decoding(()=>waiting.length===4);
 assert.equal(a.cache.radioPending.size,4);
 for(let i=4;i<30;i++){a.cache.radio([ids[i],ids[i+1],ids[i+2],ids[i+3]],1);assert.ok(a.cache.radioPending.size+a.cache.radioBuffers.size<=4);}
 a.restart(2);a.cache.radio([ids[31],ids[32],ids[33],ids[34]],2);assert.equal(waiting.length,4,'stale decodes still occupy slots');
 a.cache.radio([ids[0],ids[1],null,null],1);assert.ok(a.cache.radioWanted.has(ids[31]));assert.ok(!a.cache.radioWanted.has(ids[0]),'old demand cannot replace current demand');
 for(const pending of waiting)pending.resolve({duration:1,numberOfChannels:1});await Promise.all([...a.cache.radioPending.values()].map(e=>e.task));
 assert.equal(a.cache.radioBuffers.size,0,'stale requests cannot return buffers');assert.equal(a.cache.radioPending.size,0);
 a.cache.radio([ids[31],ids[31],null,null],2);await decoding(()=>waiting.length===5);assert.equal(a.cache.radioPending.size,1,'duplicate recording uses one slot');
 a.dispose();waiting[4].resolve({duration:1,numberOfChannels:1});await settle();assert.equal(a.cache.radioBuffers.size,0);
});
