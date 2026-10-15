import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const bindings=JSON.parse(readFileSync(new URL('../common/reference.json',import.meta.url))).driving.bindings;
import {InputState} from '../../web/input.js';
test('blur_discards_pending_edges',()=>{
 const i=new InputState(bindings);i.code('KeyW',true);i.mouse(5,2);i.scroll(1);i.clear();
 assert.deepEqual(i.takeFrame(),{held:0,pressed:0,released:0,mouseX:0,mouseY:0,wheel:0});
});
test('mm_button_is_rotate_and_rmb_moves',()=>{
 const i=new InputState(bindings);i.code('Mouse1',true);assert.equal(i.takeFrame().held,4096);
 i.clear();i.code('Mouse2',true);assert.equal(i.takeFrame().held,16384);
});
test('unbound_keys_do_not_create_actions',()=>{
 const i=new InputState(bindings);for(const key of ['Backspace','KeyQ'])i.code(key,true);
 assert.equal(i.takeFrame().held,0);
});
test('toilet_keys_and_primary_release_are_distinct',()=>{
 const i=new InputState(bindings);i.code('KeyP',true);i.code('KeyB',true);i.code('Mouse0',true);
 assert.equal(i.takeFrame().pressed,524288|1048576|512);
 i.code('Mouse0',false);
 assert.equal(i.takeFrame().released,512);
});
test('release_survives_a_frame_without_physics',()=>{
 const i=new InputState(bindings);i.code('KeyF',true);i.code('KeyF',false);
 const f=i.takeFrame();assert.equal(f.pressed,256);assert.equal(f.released,256);assert.equal(f.held,0);
});

test('wheel_assembly_and_carry_recenter_use_distinct_bits',()=>{
 const i=new InputState(bindings);i.code('KeyX',true);i.code('Tab',true);
 assert.equal(i.takeFrame().held,8192|32768);
});

test('headlights_do_not_overlap_carry_controls',()=>{
 const i=new InputState(bindings);i.code('KeyL',true);i.code('Mouse2',true);i.code('Tab',true);
 assert.equal(i.takeFrame().held,65536|16384|32768);
 i.code('KeyL',false);assert.equal(i.takeFrame().released,65536);
});

test('source_horn_keys_share_a_held_bit',()=>{const i=new InputState(bindings);i.code('KeyH',true);assert.equal(i.takeFrame().held,131072);i.code('Numpad8',true);i.code('KeyH',false);assert.equal(i.takeFrame().held,131072);i.code('Numpad8',false);const f=i.takeFrame();assert.equal(f.held,0);assert.equal(f.released,131072);});

test('source_driving_bindings_keep_unsigned_edges_and_shared_keys',()=>{
 const i=new InputState(bindings);
 i.code('Digit6',true);let f=i.takeFrame();
 assert.equal(f.held,2147483648);assert.equal(f.pressed,2147483648);
 i.code('Digit6',false);f=i.takeFrame();
 assert.equal(f.held,0);assert.equal(f.released,2147483648);
 i.code('ControlLeft',true);f=i.takeFrame();
 assert.equal(f.held,2097152|128);assert.equal(f.pressed,2097152|128);
 i.code('ControlRight',true);i.code('ControlLeft',false);f=i.takeFrame();
 assert.equal(f.held,2097152|128);assert.equal(f.released,0);
 i.code('ControlRight',false);f=i.takeFrame();
 assert.equal(f.held,0);assert.equal(f.released,2097152|128);
 i.code('Numpad0',true);assert.equal(i.takeFrame().held,2097152);
 i.clear();assert.equal(i.takeFrame().held,0);
 for (const [action,codes] of bindings.map(b=>[b.action,b.codes])) {
  for(const code of codes) {i.code(code,true);assert.ok(i.takeFrame().held>0,action);i.code(code,false);i.takeFrame();}
 }
});

test('source_bindings_are_required',()=>{assert.throws(()=>new InputState(),TypeError);});
