import test from 'node:test';
import assert from 'node:assert/strict';
import {InputState} from '../../web/input.js';
test('blur_discards_pending_edges',()=>{
 const i=new InputState();i.code('KeyW',true);i.mouse(5,2);i.scroll(1);i.clear();
 assert.deepEqual(i.takeFrame(),{held:0,pressed:0,released:0,mouseX:0,mouseY:0,wheel:0});
});
test('mm_button_is_rotate_and_rmb_moves',()=>{
 const i=new InputState();i.code('Mouse1',true);assert.equal(i.takeFrame().held,4096);
 i.clear();i.code('Mouse2',true);assert.equal(i.takeFrame().held,16384);
});
test('backspace_and_obsolete_controls_are_interface_only',()=>{
 const i=new InputState();for(const key of ['Backspace','KeyR','ArrowUp'])i.code(key,true);
 assert.equal(i.takeFrame().held,0);
});
test('toilet_keys_and_primary_release_are_distinct',()=>{
 const i=new InputState();i.code('KeyP',true);i.code('KeyB',true);i.code('Mouse0',true);
 assert.equal(i.takeFrame().pressed,524288|1048576|512);
 i.code('Mouse0',false);
 assert.equal(i.takeFrame().released,512);
});
test('release_survives_a_frame_without_physics',()=>{
 const i=new InputState();i.code('KeyF',true);i.code('KeyF',false);
 const f=i.takeFrame();assert.equal(f.pressed,256);assert.equal(f.released,256);assert.equal(f.held,0);
});

test('wheel_assembly_and_carry_recenter_use_distinct_bits',()=>{
 const i=new InputState();i.code('KeyX',true);i.code('Tab',true);
 assert.equal(i.takeFrame().held,8192|32768);
});

test('headlights_do_not_overlap_carry_controls',()=>{
 const i=new InputState();i.code('KeyL',true);i.code('Mouse2',true);i.code('Tab',true);
 assert.equal(i.takeFrame().held,65536|16384|32768);
 i.code('KeyL',false);assert.equal(i.takeFrame().released,65536);
});

test('source_horn_keys_share_a_held_bit',()=>{const i=new InputState();i.code('KeyH',true);assert.equal(i.takeFrame().held,131072);i.code('Numpad8',true);i.code('KeyH',false);assert.equal(i.takeFrame().held,131072);i.code('Numpad8',false);const f=i.takeFrame();assert.equal(f.held,0);assert.equal(f.released,131072);});

test('four_inventory_keys_keep_release_edges_and_cancel_on_clear',()=>{
 const i=new InputState();
 for(let n=1;n<=4;n++){i.clear();assert(i.code('Digit'+n,true));assert.equal(i.takeFrame().held,1<<(20+n));i.code('Digit'+n,false);assert.equal(i.takeFrame().released,1<<(20+n));}
 i.code('Digit2',true);i.clear();i.code('Digit2',false);assert.equal(i.takeFrame().released,0);
});
