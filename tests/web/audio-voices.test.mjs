import test from 'node:test';
import assert from 'node:assert/strict';
import {VoiceBank,CAR_KIND,CAR_ROUTE} from '../../web/audio/voices.js';

const param=()=>({value:0});
class Node {constructor(){this.gain=param();this.pan=param();this.playbackRate=param();for(const n of ['positionX','positionY','positionZ'])this[n]=param();}connect(){}disconnect(){}start(){this.started=true;}stop(t){this.stopAt=t;}}
class Context {constructor(){this.currentTime=0;}createGain(){return new Node();}createPanner(){return new Node();}createStereoPanner(){return new Node();}createBufferSource(){return new Node();}}
const settings={priority:100,loop:false,min_distance:1,max_distance:10,rolloff_mode:1,spatial_blend_curve:{keys:[{time:0,value:1}]}};
const record=(kind,one_shot,priority,mixer=1)=>({kind,one_shot,settings:{...settings,priority},pan:0,doppler:0,mixer,audio_source:{file:'a',path_id:1}});
const raw={emitters:[{id:1}],voices:[],portable_radios:[],clips:[{id:1,source_duration_seconds:.5}],mixers:[{id:1,pitch:1},{id:2,pitch:1},{id:3,pitch:1}],
 car_voices:[{id:1,source:{file:'a',path_id:2},one_shot:false,clip:1,settings:{...settings,priority:80,loop:true},mixer:1,doppler:0,pan:0}],shared:{mixer:1,settings,audio_manager:{real_voices:2,doppler_factor:1}},
 bindings:[80,55].map(priority=>({id:priority,clips:[1],priority,gain:1,mixer:1,emitter:1,trigger:'test'})),
 survival:[record(1,false,55),record(11,true,100),record(12,true,100,3)]};
function bank(){const b=new VoiceBank(new Context(),raw,{effect:async()=>({duration:.5})},new Map(raw.mixers.map(m=>[m.id,new Node()])),()=>1,()=>true,error=>{throw error;});b.begin();b.emitter(1,[0,0,0],[0,0,0,1],true);return b;}
const tick=()=>new Promise(r=>setTimeout(r,0));
const hit=(b,gain=.5,instance=0,kind=11)=>b.survival(kind,instance,0,1,gain,1,0,0,0);

test('one_shot_survival_plays_once_beside_its_channel',async()=>{
 const b=bank();b.survival(1,0,0,1,1,1,0,0,0);hit(b,1.6);await tick();
 const [burp,one]=b.effects;
 assert.ok(burp.playing&&one.playing,'a one-shot keeps other voices on its channel');
 assert.equal(one.source.loop,false);assert.equal(one.source.stopAt,.5);
 assert.equal(one.volume,1);assert.ok(Math.abs(one.distanceScale-.6)<1e-6);
 const car=bank();car.route(0x50000000+2,2);hit(car,.5,2,12);hit(car,.5,0,12);
 assert.deepEqual(car.effects.map(s=>s.mixer),[2,3],'a car hit uses its vehicle route or the car mixer');
});

test('hit_priority_loses_voices',()=>{
 const b=bank();
 for(let i=0;i<2;i++)b.event(raw.bindings[0],1,0,0,0,1,1);
 hit(b);assert.deepEqual(b.effects.map(s=>s.binding),[80,80],'a hit does not take an engine voice');
 b.reset();b.emitter(1,[0,0,0],[0,0,0,1],true);hit(b,.5,0);hit(b,.5,1);
 assert.deepEqual(b.effects.map(s=>s.priority),[100,100]);
 b.event(raw.bindings[1],1,0,0,0,1,1);
 assert.deepEqual(b.effects.map(s=>s.priority).sort(),[100,55].sort(),'a living sound takes a hit voice');
});

test('voice_update_switches_clip',async()=>{
 const voice={...settings,id:5,emitter:1,clip:1,loop:true,mixer:1};
 const b=new VoiceBank(new Context(),{...raw,voices:[voice]},{effect:async id=>({duration:.5,id})},new Map(raw.mixers.map(m=>[m.id,new Node()])),()=>1,()=>true,error=>{throw error;});
 b.begin();b.emitter(1,[0,0,0],[0,0,0,1],true);
 const s=b.voices.get(5);
 b.update(5,true,1,1,true,0);await tick();assert.equal(s.buffer.id,1,'0 keeps the record clip');
 b.update(5,true,1,1,true,2);await tick();assert.equal(s.buffer.id,2);assert.ok(s.source.started);
 b.update(5,true,1,1,true,2);assert.equal(s.buffer.id,2,'the same clip keeps playing');
});

test('car_voice_loops_follow_their_car_route',async()=>{
 const b=bank();b.route(CAR_ROUTE+7,2);
 b.survival(CAR_KIND+1,7,1,1,.5,1.2,3,0,0);b.survival(CAR_KIND+1,8,1,1,.5,1,0,0,0);await tick();
 assert.deepEqual(b.effects.map(s=>[s.mixer,s.survivalLoop,s.pitch]),[[2,true,1.2],[1,true,1]],'each car instance keeps its own loop and route');
 b.begin();b.survival(CAR_KIND+1,7,1,1,.5,1.2,3,0,0);b.finish();
 assert.deepEqual(b.effects.map(s=>s.playing),[true,false],'a loop that the frame does not name stops');
});
