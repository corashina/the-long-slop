import test from 'node:test';
import assert from 'node:assert/strict';
import {readBiome,writeBiome,readStartCar,writeStartCar} from '../../web/game-mode.js';

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

test('start_car_round_trips',()=>{
 const s=storage();assert.equal(readStartCar(s,6),-1);
 for(const value of [-1,0,5]){writeStartCar(s,value);assert.equal(readStartCar(s,6),value);}
});

test('bad_start_car_reads_random',()=>{
 const s=storage();
 for(const value of ['','6','-2','1.5','x']){s.setItem('gameMode.startCar',value);assert.equal(readStartCar(s,6),-1);}
 const broken={getItem(){throw Error('blocked');},setItem(){throw Error('blocked');}};
 writeStartCar(broken,2);assert.equal(readStartCar(broken,6),-1);assert.equal(readStartCar(undefined,6),-1);
});
