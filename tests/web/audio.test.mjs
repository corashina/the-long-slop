import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {webcrypto} from 'node:crypto';
import {createAudio} from '../../web/audio.js';
import {MenuState,ACTIONS,captureWithAudio} from '../../web/menu.js';
import {spatialValues,VoiceBank} from '../../web/audio/voices.js';
import {ClipCache} from '../../web/audio/clips.js';
import {installAudioBridge} from '../../web/audio.js';
const raw=JSON.parse(readFileSync(new URL('../../web/assets/reference/manifest.json',import.meta.url))).audio;
const binding=raw.bindings.find(b=>b.trigger==='ignition');
const samples=readFileSync(new URL('../../web/assets/'+raw.clips.find(c=>c.id===binding.clips[0]).path,import.meta.url));
const param=()=>({value:0,setValueAtTime(v){this.value=v;}});
class Node {constructor(){this.gain=param();this.pan=param();this.frequency=param();this.Q=param();this.playbackRate=param();for(const n of ['positionX','positionY','positionZ','orientationX','orientationY','orientationZ'])this[n]=param();}connect(){}disconnect(){}start(t,o=0){this.offset=o;Fake.starts.push(this);}stop(time){if(time>0)this.stopTime=time;else this.stopped=true;}}
class Fake {static starts=[];static contexts=[];constructor(){this.currentTime=0;this.destination=new Node();this.filters=[];this.state='suspended';Fake.contexts.push(this);}createGain(){return new Node();}createPanner(){return new Node();}createStereoPanner(){return new Node();}createBiquadFilter(){const n=new Node();this.filters.push(n);return n;}createBufferSource(){return new Node();}resume(){this.state='running';return Promise.resolve();}suspend(){this.state='suspended';return Promise.resolve();}close(){return Promise.resolve();}decodeAudioData(){return Promise.resolve({duration:raw.clips.find(c=>c.id===binding.clips[0]).duration_seconds,numberOfChannels:1});}}
function setup(Context=Fake){Fake.starts=[];Fake.contexts=[];globalThis.AudioContext=Context;globalThis.fetch=async()=>({ok:true,arrayBuffer:async()=>samples.buffer.slice(samples.byteOffset,samples.byteOffset+samples.byteLength)});return createAudio(raw);}
const settle=()=>new Promise(r=>setTimeout(r,30));
async function loaded(audio){await Promise.all(audio.cache?.pending.values()??[]);await Promise.resolve();await Promise.resolve();}
async function decoding(ready){for(let i=0;i<100&&!ready();i++)await new Promise(r=>setTimeout(r,10));assert.ok(ready(),'decode started');}
function event(a,session=1,sequence=1){a.begin(session,true,false,0);a.emitter(binding.emitter,new Float32Array([0,0,0]),new Float32Array([0,0,0,1]),true);a.event(session,sequence,binding.id,binding.emitter,0,0,0,1,1,0);a.finish();}
test('gesture_precedes_pointer_lock',async()=>{const a=setup();const m=new MenuState();m.ready();await m.capture(ACTIONS.START,()=>{assert.equal(Fake.contexts.length,1);return Promise.resolve();},()=>a.unlock());assert.equal(m.state,'playing');a.dispose();});
test('audio_failure_keeps_start_working',async()=>{for(const mode of ['fetch','decode','resume','validation']){class Bad extends Fake {resume(){return mode==='resume'?Promise.reject(Error('resume')):super.resume();}decodeAudioData(){return mode==='decode'?Promise.reject(Error('decode')):super.decodeAudioData();}}const a=setup(Bad);if(mode==='fetch')globalThis.fetch=async()=>{throw Error('fetch');};if(mode==='validation')globalThis.fetch=async()=>({ok:true,arrayBuffer:async()=>new ArrayBuffer(2)});const m=new MenuState();m.ready();await m.capture(ACTIONS.START,async()=>{},()=>a.unlock());await settle();event(a);await loaded(a);assert.equal(m.state,'playing');assert.equal(Fake.starts.length,0);a.dispose();}const a=createAudio({});assert.doesNotThrow(()=>a.unlock());a.dispose();});
test('restart_rejects_pending_decode',async()=>{let complete;class Slow extends Fake {decodeAudioData(){return new Promise(r=>{complete=r;});}}const a=setup(Slow);a.unlock();await settle();event(a);await decoding(()=>complete);assert.ok(complete);a.restart(2);complete({duration:1});await settle();assert.equal(Fake.starts.length,0);a.dispose();});
test('restarted_session_can_unlock',async()=>{let attempts=0;class Fail extends Fake {resume(){attempts++;return Promise.reject(Error('resume'));}}const a=setup(Fail);a.unlock();await settle();a.restart(2);a.unlock();await settle();assert.equal(attempts,2);a.dispose();});
test('rebase_preserves_distance',()=>{const v=raw.voices.find(v=>v.id===binding.voice),out=new Float32Array(6);spatialValues(v,[10,3,20],[1,2,3],[0,0,0,1],out);assert.ok(Array.from(out).every(Number.isFinite));assert.ok(Math.abs(out[0]-1/Math.sqrt(371))<1e-7);const before=Array.from(out);spatialValues(v,[-90,3,-80],[-99,2,-97],[0,0,0,1],out);assert.ok(Array.from(out).every(Number.isFinite));assert.deepEqual(Array.from(out),before);});
test('pause_requires_continue',async()=>{const copy=structuredClone(raw);const voice=copy.voices.find(v=>v.id===binding.voice);voice.loop=true;voice.clip=binding.clips[0];const a=setup();a.dispose();const c=createAudio(copy);c.unlock();await settle();c.begin(1,true,false,0);c.emitter(voice.emitter,[0,0,0],[0,0,0,1],true);c.voice(voice.id,true,1,1,true);c.finish();await loaded(c);assert.equal(Fake.starts.length,1);Fake.contexts.at(-1).currentTime=.25;c.pause();c.begin(1,true,false,1);c.emitter(voice.emitter,[0,0,0],[0,0,0,1],true);c.voice(voice.id,true,1,1,true);c.finish();await settle();assert.equal(Fake.contexts.at(-1).state,'suspended');assert.equal(Fake.starts.length,1);c.unlock();await settle();assert.equal(Fake.starts.length,2);assert.equal(Fake.starts[1].offset,.25);c.dispose();});
test('actual_source_effect_and_restart_cleanup',async()=>{const a=setup();a.unlock();await settle();event(a);await loaded(a);assert.equal(Fake.starts.length,1);a.restart(2);assert.equal(Fake.starts[0].stopped,true);a.dispose();});
test('gesture_survives_old_native_frame',async()=>{const a=setup();a.restart(2);a.unlock();a.begin(1,true,true,0);a.begin(2,true,true,0);await settle();a.begin(2,true,false,0);event(a,2);await loaded(a);assert.equal(Fake.starts.length,1);a.dispose();});
test('native_parameq_uses_proven_mapping',async()=>{const a=setup();a.unlock();await settle();assert.equal(a.context.filters[0].type,'peaking');assert.equal(a.context.filters[0].Q.value,1/raw.mixers.find(m=>m.name==='Radio').effects[1].parameters[1]);assert.equal(a.context.filters[0].gain.value,40*Math.log10(raw.mixers.find(m=>m.name==='Radio').effects[1].parameters[2]));a.dispose();});

test('removed_owner_rejects_pending_decode',async()=>{let complete;class Slow extends Fake {decodeAudioData(){return new Promise(r=>{complete=r;});}}const a=setup(Slow);a.unlock();await settle();event(a);await decoding(()=>complete);a.emitter(binding.emitter,[0,0,0],[0,0,0,1],false);complete({duration:raw.clips.find(c=>c.id===binding.clips[0]).duration_seconds,numberOfChannels:1});await settle();assert.equal(Fake.starts.length,0);a.dispose();});


for(const nextAction of [ACTIONS.CONTINUE,ACTIONS.RESTART])test('failed_restart_then_'+(nextAction===ACTIONS.CONTINUE?'continue':'restart')+'_keeps_native_session',async()=>{
 const audio=setup(),menu=new MenuState();menu.ready();menu.started=true;menu.pause();menu.takeActions();audio.begin(1,true,true,0);audio.event(1,9,binding.id,binding.emitter,0,0,0,1,1,0);
 assert.equal(await captureWithAudio(menu,ACTIONS.RESTART,async()=>{throw Error('capture blocked');},audio),false);
 assert.equal(menu.takeActions(),0);assert.equal(audio.session,1);assert.equal(audio.sequence,9);
 assert.equal(await captureWithAudio(menu,nextAction,async()=>{if(nextAction===ACTIONS.RESTART){assert.equal(audio.pendingRestart,true);event(audio,1,10);audio.begin(1,true,true,0);await settle();assert.equal(Fake.starts.length,0);}},audio),true);
 const action=menu.takeActions(),nativeSession=action===ACTIONS.RESTART?2:1;await settle();event(audio,nativeSession,nextAction===ACTIONS.RESTART?1:10);await loaded(audio);
 assert.equal(audio.session,nativeSession);assert.equal(audio.accepted,true);assert.equal(Fake.starts.length,1);audio.dispose();
});

function vehicleBank(){Fake.starts=[];const context=new Fake(),cache={effect:async id=>({duration:raw.clips.find(c=>c.id===id).duration_seconds})};return new VoiceBank(context,raw,cache,new Map(raw.mixers.map(m=>[m.id,new Node()])),()=>1,()=>true,error=>{throw error;});}
function vehicleFrame(bank,t=1){bank.begin();for(const id of [raw.engine.emitter,raw.voices.find(v=>v.id===raw.horn.voice).emitter])bank.emitter(id,[0,0,0],[0,0,0,1],true);for(const id of [raw.engine.idle_voice,raw.engine.motor_voice,raw.horn.voice])bank.update(id,true,.7,1,true);bank.finish();}
test('vehicle_updates_retain_voices',async()=>{const bank=vehicleBank();vehicleFrame(bank);await Promise.resolve();for(let i=0;i<600;i++)vehicleFrame(bank);assert.equal(Fake.starts.length,3);bank.dispose();});
test('starter_uses_source_interval',async()=>{const bank=vehicleBank(),id=raw.engine.emitter,b=raw.bindings.find(b=>b.id===raw.engine.starter_binding),duration=raw.clips.find(c=>c.id===b.clips[0]).source_duration_seconds,interval=duration*raw.engine.constants.engineStartTimeModifier;bank.begin();bank.emitter(id,[0,0,0],[0,0,0,1],true);bank.starter(id,true,interval);await Promise.resolve();assert.equal(Fake.starts.length,0);bank.starter(id,true,interval+.125);await Promise.resolve();assert.equal(Fake.starts.length,1);bank.starter(id,true,2*interval+.125);await Promise.resolve();assert.equal(Fake.starts.length,1);bank.starter(id,true,2*interval+.25);await Promise.resolve();assert.equal(Fake.starts.length,2);bank.starter(id,false,10);assert.ok(Fake.starts.every(s=>!s.stopped),'release preserves started tails');bank.remove(id);assert.ok(Fake.starts.every(s=>s.stopped));bank.dispose();});
test('starter_release_cancels_decode',async()=>{const bank=vehicleBank();let complete;bank.cache.effect=()=>new Promise(r=>complete=r);bank.begin();bank.emitter(raw.engine.emitter,[0,0,0],[0,0,0,1],true);bank.starter(raw.engine.emitter,true,10);bank.starter(raw.engine.emitter,false,10);complete({duration:1});await Promise.resolve();assert.equal(Fake.starts.length,0);bank.dispose();});

test('engine_removal_keeps_horn_owner',async()=>{const bank=vehicleBank();vehicleFrame(bank);await Promise.resolve();bank.starter(raw.engine.emitter,true,10);await Promise.resolve();bank.remove(raw.engine.emitter);const horn=bank.voices.get(raw.horn.voice);assert.ok(horn.source);assert.ok(Fake.starts.filter(s=>s!==horn.source).every(s=>s.stopped));bank.dispose();});

test('fresh_effects_start_at_zero_after_natural_completion',async()=>{
 const bank=vehicleBank(),id=raw.engine.emitter,stop=raw.bindings.find(b=>b.id===raw.engine.stop_binding);
 bank.begin();bank.emitter(id,[0,0,0],[0,0,0,1],true);
 bank.starter(id,true,1);await Promise.resolve();
 const slot=bank.effects.find(s=>s.playing);
 bank.context.currentTime=.5;bank.finish();
 bank.context.currentTime=slot.buffer.duration;slot.source.onended();
 bank.starter(id,true,2);await Promise.resolve();
 assert.equal(bank.effects.find(s=>s.playing),slot,'reuse the completed starter slot');
 bank.context.currentTime+=.5;bank.finish();
 bank.context.currentTime+=slot.buffer.duration;slot.source.onended();
 bank.event(stop,id,0,0,0,1,1);await Promise.resolve();
 assert.equal(bank.effects.find(s=>s.playing),slot,'reuse the completed slot for the stop effect');
 assert.deepEqual(Fake.starts.map(s=>s.offset),[0,0,0]);
 bank.dispose();
});

 test('crafted_effect_follows_owner',async()=>{
 const bank=vehicleBank(),b=raw.bindings.find(b=>b.trigger==='crafted');
 bank.begin();bank.emitter(b.emitter,[1,2,3],[0,0,0,1],true);
 bank.event(b,b.emitter,1,2,3,1,1);await Promise.resolve();
 const s=bank.effects.find(s=>s.playing);
 bank.emitter(b.emitter,[4,5,6],[0,0,0,1],true);bank.finish();
 assert.equal(s.spatialEmitter,b.emitter);
 assert.deepEqual([s.panner.positionX.value,s.panner.positionY.value,s.panner.positionZ.value],[4,5,6]);
 bank.dispose();
 });
 test('removal_rejects_late_load',async()=>{
 const bank=vehicleBank(),b=raw.bindings.find(b=>b.trigger==='crafted');let complete;
 bank.cache.effect=()=>new Promise(r=>complete=r);
 bank.begin();bank.emitter(b.emitter,[0,0,0],[0,0,0,1],true);bank.event(b,b.emitter,0,0,0,1,1);
 bank.remove(b.emitter);complete({duration:1});await Promise.resolve();assert.equal(Fake.starts.length,0);bank.dispose();
 });
 test('endpoint_gain_above_one_scales_source_distances',async()=>{
 const bank=vehicleBank(),b=raw.bindings.find(b=>b.trigger==='startPosEnd');
 bank.begin();bank.emitter(b.emitter,[0,0,0],[0,0,0,1],true);bank.event(b,b.emitter,0,0,0,3/b.gain,1);await Promise.resolve();
 const s=bank.effects.find(s=>s.playing);
 assert.equal(s.volume,1);assert.equal(s.distanceScale,2);bank.dispose();
 });

test('shared_effect_lifetime_uses_unpitched_source_length',async()=>{
 const bank=vehicleBank(),b=raw.bindings.find(b=>b.trigger==='crafted');
 bank.context.currentTime=10;bank.begin();bank.emitter(b.emitter,[0,0,0],[0,0,0,1],true);bank.event(b,b.emitter,0,0,0,1,.85);await Promise.resolve();
 assert.equal(Fake.starts[0].stopTime,10+raw.clips.find(c=>c.id===b.clips[0]).source_duration_seconds);
 bank.dispose();
});

test('cap_fill_loop_change_keeps_source_and_survives_pause',async()=>{
 const bank=vehicleBank(),rule=raw.fluids[0],id=rule.fill_voice,record=raw.voices.find(v=>v.id===id);
 bank.begin();bank.emitter(record.emitter,[0,0,0],[0,0,0,1],true);bank.update(id,true,.5,1,true);await Promise.resolve();
 const source=bank.voices.get(id).source;assert.equal(source.loop,true);
 bank.update(id,true,.7,1,false);assert.equal(bank.voices.get(id).source,source);assert.equal(source.loop,false);
 bank.pause();bank.resume();bank.update(id,true,.7,1,false);await Promise.resolve();assert.equal(bank.voices.get(id).source.loop,false);assert.equal(bank.voices.get(id).source.offset,0);bank.dispose();
});

test('non_loop_fluid_completion_restarts_at_zero',async()=>{
 const bank=vehicleBank(),id=raw.fluids[0].fill_voice,record=raw.voices.find(v=>v.id===id);
 bank.begin();bank.emitter(record.emitter,[0,0,0],[0,0,0,1],true);bank.update(id,true,.5,1,false);await Promise.resolve();
 const slot=bank.voices.get(id);bank.context.currentTime=.5;bank.parameters(slot);slot.source.onended();bank.update(id,true,.5,1,false);
 assert.deepEqual(Fake.starts.map(s=>s.offset),[0,0]);bank.dispose();
});

function radioSamples(id){const clip=raw.clips.find(c=>c.id===id),bytes=readFileSync(new URL('../../web/assets/'+clip.path,import.meta.url));return bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength);}
function radioSetup(Context=Fake){const a=setup(Context);globalThis.fetch=async url=>{const clip=raw.clips.find(c=>'./assets/'+c.path===url);return {ok:true,arrayBuffer:async()=>radioSamples(clip.id)};};return a;}
const recording=raw.radio.stations[0].music[0];
function radioFrame(a,offset=0,clip=recording){a.begin(a.session,true,false,offset);a.emitter(raw.radio.emitter,[0,0,0],[0,0,0,1],true);for(const id of raw.radio.voices)a.emitter(raw.voices.find(v=>v.id===id).emitter,[0,0,0],[0,0,0,1],true);a.radio({power:true,am:false,frequency:89.2,volume:.1,staticGain:0,primary:{clip,offsetSeconds:offset,gain:.1,nextClip:null},secondary:{clip:null,offsetSeconds:0,gain:0,nextClip:null}});a.finish();}
async function radioLoaded(a){await Promise.all([...a.cache.radioPending.values()].map(e=>e.task));await loaded(a);}
test('late_radio_decode_joins_broadcast',async()=>{
 let complete;const duration=raw.clips.find(c=>c.id===recording).duration_seconds;
 class Slow extends Fake{decodeAudioData(){return new Promise(r=>complete=r);}}
 const a=radioSetup(Slow);a.unlock();await settle();const staticClip=raw.clips.find(c=>c.id===raw.voices.find(v=>v.id===raw.radio.voices[0]).clip);a.cache.buffers.set(staticClip.id,{duration:staticClip.duration_seconds});radioFrame(a,0);await decoding(()=>complete);radioFrame(a,3);
 const before=Fake.starts.length;complete({duration,numberOfChannels:raw.clips.find(c=>c.id===recording).channels});await radioLoaded(a);radioFrame(a,5);
 assert.equal(Fake.starts.length,before+1);assert.equal(Fake.starts.at(-1).offset,5);
 const source=Fake.starts.at(-1);a.context.currentTime=.25;radioFrame(a,5.5);assert.equal(Fake.starts.length,before+1,'source threshold retains playback');
 radioFrame(a,6);assert.equal(source.stopped,true);assert.equal(Fake.starts.at(-1).offset,6);a.dispose();
});
test('radio_cache_is_bounded',async()=>{
 const waiting=[],ids=raw.radio.stations.flatMap(s=>s.music).filter((id,i,a)=>a.indexOf(id)===i);
 class Slow extends Fake{decodeAudioData(bytes){return new Promise(resolve=>waiting.push({resolve,bytes}));}}
 const a=radioSetup(Slow);a.unlock();await settle();a.cache.radio(ids.slice(0,2),ids.slice(2,4),1);await decoding(()=>waiting.length===4);
 assert.equal(a.cache.radioPending.size,4);
 for(let i=4;i<30;i++){a.cache.radio([ids[i],ids[i+1]],[ids[i+2],ids[i+3]],1);assert.ok(a.cache.radioPending.size+a.cache.radioBuffers.size<=4);}
 a.restart(2);a.cache.radio([ids[31],ids[32]],[ids[33],ids[34]],2);assert.equal(waiting.length,4,'stale decodes still occupy slots');
 a.cache.radio([ids[0],ids[1]],[null,null],1);assert.ok(a.cache.radioWanted.has(ids[31]));assert.ok(!a.cache.radioWanted.has(ids[0]),'old demand cannot replace current demand');
 for(const pending of waiting)pending.resolve({duration:1,numberOfChannels:1});await Promise.all([...a.cache.radioPending.values()].map(e=>e.task));
 assert.equal(a.cache.radioBuffers.size,0,'stale requests cannot return buffers');assert.equal(a.cache.radioPending.size,0);
 a.cache.radio([ids[31],ids[31]],[null,null],2);await decoding(()=>waiting.length===5);assert.equal(a.cache.radioPending.size,1,'duplicate recording uses one slot');
 a.dispose();waiting[4].resolve({duration:1,numberOfChannels:1});await settle();assert.equal(a.cache.radioBuffers.size,0);
});
test('radio_frame_bridge_is_retained',()=>{
 let first;const target={};installAudioBridge(target,{radio(frame){if(first){assert.equal(frame,first);assert.equal(frame.primary,first.primary);}else first=frame;}});
 target.gameAudioRadio(true,false,89.2,.1,0,recording,3,.1,0,0,0,0,0);target.gameAudioRadio(true,true,630,.2,0,recording,4,.2,0,0,0,0,0);
 assert.equal(first.primary.offsetSeconds,4);assert.equal(first.secondary.clip,null);
});
test('radio_removal_rejects_late_load_and_engine_owner_is_separate',async()=>{
 let complete;const clip=raw.clips.find(c=>c.id===recording);
 class Slow extends Fake{decodeAudioData(){return new Promise(r=>complete=r);}}
 const a=radioSetup(Slow);a.unlock();await settle();const staticClip=raw.clips.find(c=>c.id===raw.voices.find(v=>v.id===raw.radio.voices[0]).clip);a.cache.buffers.set(staticClip.id,{duration:staticClip.duration_seconds});radioFrame(a,3);await decoding(()=>complete);
 a.emitter(raw.engine.emitter,[0,0,0],[0,0,0,1],false);assert.equal(a.bank.emitters.get(raw.radio.emitter).alive,true);
 a.emitter(raw.radio.emitter,[0,0,0],[0,0,0,1],false);complete({duration:clip.duration_seconds,numberOfChannels:clip.channels});await radioLoaded(a);
 assert.ok(Fake.starts.every(s=>s.stopped));assert.equal(a.cache.radioBuffers.size,0);a.dispose();
});

test('radio_pause_mutes_and_resume_joins_retained_clock',async()=>{
 const a=radioSetup();a.unlock();await settle();const clip=raw.clips.find(c=>c.id===recording),buffer={duration:clip.duration_seconds};
 const staticClip=raw.clips.find(c=>c.id===raw.voices.find(v=>v.id===raw.radio.voices[0]).clip);a.cache.buffers.set(staticClip.id,{duration:staticClip.duration_seconds});a.cache.radioBuffers.set(recording,buffer);
 radioFrame(a,3);const first=a.bank.voices.get(raw.radio.voices[1]).source;assert.equal(first.offset,3);
 a.pause();assert.equal(first.stopped,true);radioFrame(a,3);assert.equal(a.bank.voices.get(raw.radio.voices[1]).source,null);
 a.unlock();await settle();a.cache.radioBuffers.set(recording,buffer);radioFrame(a,3);const resumed=a.bank.voices.get(raw.radio.voices[1]).source;assert.ok(resumed);assert.equal(resumed.offset,3);a.dispose();
});
