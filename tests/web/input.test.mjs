import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const bindings=JSON.parse(readFileSync(new URL('../common/reference.json',import.meta.url))).driving.bindings;
import {InputState} from '../../web/input.js';
test('blur_discards_pending_edges',()=>{
 const i=new InputState(bindings);i.code('KeyW',true);i.mouse(5,2);i.scroll(1);i.clear();
 assert.deepEqual(i.takeFrame(),{held:0,pressed:0,released:0,mouseX:0,mouseY:0,wheel:0});
});

test('source_bindings_are_required',()=>{assert.throws(()=>new InputState(),TypeError);});
