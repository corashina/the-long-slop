import test from 'node:test';
import assert from 'node:assert/strict';
const bindings=[];
import {InputState} from '../../web/input.js';
test('blur_discards_pending_edges',()=>{
 const i=new InputState(bindings);i.code('KeyW',true);i.mouse(5,2);i.scroll(1);i.clear();
 assert.deepEqual(i.takeFrame(),{held:0,pressed:0,released:0,mouseX:0,mouseY:0,wheel:0,inventoryHeld:0,inventoryReleased:0});
});

test('source_bindings_are_required',()=>{assert.throws(()=>new InputState(),TypeError);});

test('inventory_does_not_set_driving_bits',()=>{
 const i=new InputState([{action:'gear1',codes:['Digit1']}]);
 i.code('Digit1',true);let f=i.takeFrame();assert.equal(f.held,2**26);assert.equal(f.inventoryHeld,1);
 i.code('Digit1',false);f=i.takeFrame();assert.equal(f.released,2**26);assert.equal(f.inventoryReleased,1);
 i.clear();assert.equal(i.takeFrame().inventoryReleased,0);
});

test('obsolete_wheel_shortcut_has_no_input_bit',()=>{
 const i=new InputState(bindings);i.code('KeyX',true);
 const f=i.takeFrame();assert.equal(f.held,0);assert.equal(f.pressed,0);
});
test('source_carry_buttons_and_frame_edges',()=>{
 const input = new InputState([]);
 input.code('Mouse1', true);
 assert.equal(input.takeFrame().held, 16384);
 input.code('Mouse2', true);
 assert.equal(input.takeFrame().held, 16384 | 4096);
 input.code('Mouse1', false);
 assert.equal(input.takeFrame().held, 4096);
 input.code('Mouse1', true);
 input.code('Mouse2', false);
 assert.equal(input.takeFrame().held, 16384);
 input.clear();
 input.code('KeyF', true);
 input.code('KeyF', false);
 const frame = input.takeFrame();
 assert.equal(frame.held, 0);
 assert.equal(frame.pressed, 256);
 assert.equal(frame.released, 256);
 input.code('Mouse1', true);
 input.code('Mouse2', true);
 input.code('Mouse2', false);
 input.code('Digit1', true);
 input.code('Digit1', false);
 input.mouse(5, -2);
 input.scroll(1);
 input.clear();
 assert.deepEqual(input.takeFrame(),{held:0,pressed:0,released:0,mouseX:0,mouseY:0,wheel:0,inventoryHeld:0,inventoryReleased:0});
});
