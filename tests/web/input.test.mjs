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
 const i=new InputState();for(const key of ['Backspace','KeyR','KeyP','ArrowUp'])i.code(key,true);
 assert.equal(i.takeFrame().held,0);
});
test('release_survives_a_frame_without_physics',()=>{
 const i=new InputState();i.code('KeyF',true);i.code('KeyF',false);
 const f=i.takeFrame();assert.equal(f.pressed,256);assert.equal(f.released,256);assert.equal(f.held,0);
});

test('wheel_assembly_and_carry_recenter_use_distinct_bits',()=>{
 const i=new InputState();i.code('KeyX',true);i.code('Tab',true);
 assert.equal(i.takeFrame().held,8192|32768);
});
