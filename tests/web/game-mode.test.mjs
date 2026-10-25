import test from 'node:test';
import assert from 'node:assert/strict';
import {readBiome,writeBiome} from '../../web/game-mode.js';

function storage(){const items=new Map();return {getItem:key=>items.has(key)?items.get(key):null,setItem:(key,value)=>items.set(key,String(value))};}

test('biome_round_trips',()=>{
 const s=storage();
 for(const value of [0,1,2,3]){writeBiome(s,value);assert.equal(readBiome(s),value);}
});

test('bad_biome_reads_mixed',()=>{
 const s=storage();assert.equal(readBiome(s),0);
 for(const value of ['','9','x']){s.setItem('gameMode.biome',value);assert.equal(readBiome(s),0);}
 const broken={getItem(){throw Error('blocked');},setItem(){throw Error('blocked');}};
 writeBiome(broken,2);assert.equal(readBiome(broken),0);assert.equal(readBiome(undefined),0);
});
