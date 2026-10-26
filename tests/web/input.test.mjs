import test from 'node:test';
import assert from 'node:assert/strict';
const bindings=[];
import {InputState,createInput} from '../../web/input.js';
import {createCpuProfile} from '../../web/cpu-profile.js';
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

test('profile_input_marks_the_fixed_step_and_clears_pending_events_on_blur',t=>{
 const oldWindow=globalThis.window,oldDocument=globalThis.document;
 const host=new EventTarget(),doc=new EventTarget(),canvas=new EventTarget();globalThis.window=host;globalThis.document=doc;
 t.after(()=>{globalThis.window=oldWindow;globalThis.document=oldDocument;});
 let clock=10;const profile=createCpuProfile(()=>clock,1000);profile.enable(true);
 const input=createInput(canvas,()=>true,[],profile);t.after(()=>input.dispose());
 const key=()=>{const event=new Event('keydown',{cancelable:true});Object.defineProperties(event,{code:{value:'KeyW'},repeat:{value:false},timeStamp:{value:5}});return event;};
 host.dispatchEvent(key());assert.equal(input.takeFrame().held,1);profile.submitFrame(1);
 assert.equal(profile.snapshot().inputs[0].step,null);
 clock=15;input.consume(2);clock=20;profile.submitFrame(3);
 assert.equal(profile.snapshot().inputs[0].step,2);assert.equal(profile.snapshot().inputs[0].token,3);
 host.dispatchEvent(key());host.dispatchEvent(new Event('blur'));assert.equal(input.takeFrame().held,0);input.consume(3);profile.submitFrame(4);
 assert.equal(profile.snapshot().inputs[1].step,null);
});

test('F07 typing routes keys to the computer and blocks game bits',()=>{
 const i=new InputState(bindings);i.code('KeyW',true);i.setTyping(true);
 assert.equal(i.takeFrame().held,0,'typing releases held game keys');
 for(const [code,key] of [['KeyH','h'],['KeyI','i'],['Enter','Enter'],['Backspace','Backspace'],['KeyA','a']])i.type(code,key,false);
 i.type('KeyA','a',true);i.type('Space',' ',true);
 const buffer=new Uint8Array(64),n=i.takeText(buffer);
 assert.deepEqual([...buffer.subarray(0,n)],[104,105,13,8,97,32]);
 const keys=i.typingKeys();
 assert.equal(keys&(1<<12),1<<12,'I is the high C piano key');
 assert.equal(keys&(1<<21),1<<21,'A lowers the octave');
 assert.equal(keys&(1<<24),1<<24,'a key is held');
 assert.equal(i.typingKeys()&(1<<21),0,'music keys are edges');
 assert.equal(i.takeText(buffer),0);
 i.setTyping(false);assert.equal(i.typingKeys(),0);
});
