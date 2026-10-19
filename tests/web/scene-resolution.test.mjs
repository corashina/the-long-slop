import test from 'node:test';
import assert from 'node:assert/strict';
import {installSceneResolution} from '../../web/scene-resolution.js';

function control(){
 const select=new EventTarget();
 select.value='0.75';
 let handlers=0;
 const add=select.addEventListener.bind(select);
 select.addEventListener=(...args)=>{handlers++;add(...args);};
 return {select,handlers:()=>handlers};
}

test('each page starts at native scene resolution',()=>{
 const target={},first=control();
 installSceneResolution(target,first.select);
 assert.equal(first.select.value,'1');
 assert.equal(target.gameSceneScale(),1);
 first.select.value='0.75';first.select.dispatchEvent(new Event('change'));
 assert.equal(target.gameSceneScale(),.75);
 const nextPage={},nextControl=control();
 installSceneResolution(nextPage,nextControl.select);
 assert.equal(nextPage.gameSceneScale(),1);
});

test('the selector changes the bridge value without repeated setup',()=>{
 const target={},node=control();
 installSceneResolution(target,node.select);
 const bridge=target.gameSceneScale;
 for(const [value,expected] of [['0.75',.75],['1',1],['0.75',.75]]){
  node.select.value=value;node.select.dispatchEvent(new Event('change'));
  for(let frame=0;frame<1000;frame++)assert.equal(bridge(),expected);
  assert.equal(target.gameSceneScale,bridge);
  assert.equal(node.handlers(),1);
 }
});
