import test from 'node:test';
import assert from 'node:assert/strict';
const bindings=[];
import {InputState,createInput} from '../../web/input.js';
test('blur_discards_pending_edges',()=>{
 const i=new InputState(bindings);i.code('KeyW',true);i.mouse(5,2);i.scroll(1);i.clear();
 assert.deepEqual(i.takeFrame(),{held:0,pressed:0,released:0,mouseX:0,mouseY:0,wheel:0,inventoryHeld:0,inventoryReleased:0,playerHeld:0,playerPressed:0,playerReleased:0});
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

test('zoom_aliases_release_only_after_last_key',()=>{
 const i=new InputState(bindings);
 i.code('KeyZ',true);let f=i.takeFrame();assert.equal(f.playerHeld,2);assert.equal(f.playerPressed,2);
 i.code('KeyY',true);f=i.takeFrame();assert.equal(f.playerHeld,2);assert.equal(f.playerPressed,0);
 i.code('KeyZ',false);f=i.takeFrame();assert.equal(f.playerHeld,2);assert.equal(f.playerReleased,0);
 i.code('KeyY',false);f=i.takeFrame();assert.equal(f.playerHeld,0);assert.equal(f.playerReleased,2);
 assert.equal(i.takeFrame().playerReleased,0);
});

test('clear_cancels_q_without_release',()=>{
 const i=new InputState(bindings);
 i.code('KeyQ',true);let f=i.takeFrame();assert.equal(f.playerHeld,1);assert.equal(f.playerPressed,1);
 i.clear();i.code('KeyQ',false);f=i.takeFrame();
 assert.equal(f.playerHeld,0);assert.equal(f.playerPressed,0);assert.equal(f.playerReleased,0);
});

test('source_item_mouse_buttons',t=>{
 const originals=new Map(['window','document'].map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]));
 const window=new EventTarget(),document=new EventTarget(),canvas=new EventTarget();
 Object.assign(globalThis,{window,document});
 const input=createInput(canvas,()=>true,bindings);
 t.after(()=>{input.dispose();for(const [key,descriptor] of originals){if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key];}});
 const mouse=(target,type,button)=>target.dispatchEvent(Object.assign(new Event(type,{cancelable:true}),{button}));
 mouse(canvas,'mousedown',0);assert.equal(input.takeFrame().held,512);
 mouse(window,'mouseup',0);assert.equal(input.takeFrame().released,512);
 mouse(canvas,'mousedown',1);assert.equal(input.takeFrame().held,4096);
 mouse(canvas,'mousedown',2);assert.equal(input.takeFrame().held,4096|16384);
 mouse(window,'mouseup',2);let f=input.takeFrame();assert.equal(f.held,4096);assert.equal(f.released,16384);
 mouse(window,'mouseup',1);f=input.takeFrame();assert.equal(f.held,0);assert.equal(f.released,4096);
});
