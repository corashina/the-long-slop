import test from 'node:test';
import assert from 'node:assert/strict';
import {optionBits,parseSeed,readOptions} from '../../web/game-mode.js';

test('options_default_to_the_original',()=>{
 const empty={getItem:()=>null};
 assert.deepEqual(readOptions(empty),{peaceful:false,survival:true,homesick:false});
 assert.equal(optionBits(readOptions(empty)),2);
});
test('seed_parses_like_the_original_menu',()=>{
 assert.equal(parseSeed('123'),123);
 assert.equal(parseSeed('-5'),-5);
 assert.equal(parseSeed(''),0);
 assert.equal(parseSeed('99999999999'),2147483647);
 assert.equal(parseSeed('-abc'),-2147483648);
 assert.equal(parseSeed('abc'),2147483647);
});
