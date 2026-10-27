import test from 'node:test';
import assert from 'node:assert/strict';
import {installOutfit,readOutfit,fullReset} from '../../web/outfit.js';

const segment=(enabled,hsv)=>({name:'Part',default_enabled:enabled,disable_able:enabled,default_hsv:hsv});
const characters=[{segments:[segment(true,[0.1,0.2,0.3]),segment(false,[0.4,0.5,0.6])]},{segments:[segment(false,[0.7,0.8,0.9])]}];
const defaults={model:2,character:0,characters:[
 {edited:false,segments:[{enabled:true,hue:0.1,brightness:0.3,saturation:0.2},{enabled:false,hue:0.4,brightness:0.6,saturation:0.5}]},
 {edited:false,segments:[{enabled:false,hue:0.7,brightness:0.9,saturation:0.8}]}]};
function memory(initial){const items=new Map(initial===undefined?[]:[['the-long-slop.outfit',initial]]);return {items,storage:()=>({getItem:key=>items.get(key)??null,setItem:(key,value)=>items.set(key,value)})};}

test('missing, bad or blocked storage gives the defaults',()=>{
 assert.deepEqual(fullReset(characters),defaults);
 assert.deepEqual(readOutfit(memory().storage,characters),defaults);
 assert.deepEqual(readOutfit(memory('not json').storage,characters),defaults);
 const bad=[value=>{value.model=3;},value=>{value.character=2;},value=>{value.extra=1;},value=>{value.characters.pop();},
  value=>{value.characters[0].segments.pop();},value=>{value.characters[1].segments[0].hue=1.5;},value=>{value.characters[0].edited=1;}];
 for(const change of bad){const value=structuredClone(defaults);value.characters[0].edited=true;change(value);assert.deepEqual(readOutfit(memory(JSON.stringify(value)).storage,characters),defaults);}
 assert.deepEqual(readOutfit(()=>{throw new Error('SecurityError');},characters),defaults);
});

test('a character that is not edited resets on load',()=>{
 const value=structuredClone(defaults);
 value.model=1;value.character=1;
 value.characters[0].edited=true;value.characters[0].segments[0]={enabled:false,hue:0.5,brightness:0.5,saturation:0.5};
 value.characters[1].segments[0].hue=0.25;
 const outfit=readOutfit(memory(JSON.stringify(value)).storage,characters);
 assert.equal(outfit.model,1);assert.equal(outfit.character,1);
 assert.deepEqual(outfit.characters[0],value.characters[0]);
 assert.deepEqual(outfit.characters[1],defaults.characters[1]);
});

test('the outfit is sent at start and after a change, and stored on save',()=>{
 const store=memory(),target={},first=installOutfit(target,store.storage,characters);
 assert.equal(target.gameOutfit(),JSON.stringify(defaults));assert.equal(target.gameOutfit(),undefined);
 first.outfit.characters[0].edited=true;first.outfit.characters[0].segments[1].enabled=true;first.outfit.model=0;
 first.changed();
 const expected=JSON.stringify(first.outfit);
 assert.equal(target.gameOutfit(),expected);
 assert.equal(store.items.size,0);
 first.save();
 const second={};installOutfit(second,store.storage,characters);
 assert.equal(second.gameOutfit(),expected);
});
