import test from 'node:test';
import assert from 'node:assert/strict';
import {formatTank,FeedbackState,selectIcon} from '../../web/feedback.js';
const tank=(amount,capacity=39)=>({amount,capacity,quantities:[amount,0,0,0,0,0],hasName:false,name:'',fluidName:'',noText:false});
test('empty_full_and_fractional_source_format',()=>{
 assert.equal(formatTank(tank(Math.fround(.05))),'empty');
 assert.equal(formatTank(tank(.5)),'0.5 / 39 L\n100% Gas');
 assert.equal(formatTank(tank(39)),'39 L (full)\n100% Gas');
 assert.equal(formatTank({...tank(.5),noText:true}),'');
});
test('mixture_filters_below_one_percent_and_keeps_source_order',()=>{
 assert.equal(formatTank({...tank(10),quantities:[9.91,.09,0,0,0,0]}),'10 / 39 L\n99% Gas');
 assert.equal(formatTank({...tank(10),quantities:[9,.1,0,.9,0,0]}),'10 / 39 L\n90% Gas\n1% Oil\n9% Water');
});
test('same_volume_new_container_refreshes_capacity_and_name',()=>{
 const m=new FeedbackState();
 m.update({...tank(.5),hasName:true,name:'Fuel cap',fluidName:'Fuel Tank',actions:1,context:2,carrying:false,notice:''});
 assert.match(m.fluidText,/Fuel Tank:\n0.5 \/ 39 L/);
 m.update({...tank(.5,5),hasName:true,name:'Oil cap',fluidName:'Oil Tank',actions:1,context:2,carrying:false,notice:''});
 assert.match(m.fluidText,/Oil Tank:\n0.5 \/ 5 L/);
});
test('chase_and_menu_hide_feedback_and_new_game_preserves_choice',()=>{
 const m=new FeedbackState();m.setVisible(false);assert.equal(m.visible,false);
 m.toggle();m.setVisible(true);assert.equal(m.visible,false);
 m.setVisible(false);m.setVisible(true);assert.equal(m.visible,false);
});
test('context_icons_prioritize_mount_fill_and_use',()=>{
 assert.equal(selectIcon(16|256,5,false),'mount');
 assert.equal(selectIcon(64|1,2,false),'fill');
 assert.equal(selectIcon(512,4,false),'exit');
 assert.equal(selectIcon(0,0,false),'default');
 assert.equal(selectIcon(0,0,true),'carrying');
});

test('fluid_label_and_action_owner_have_independent_caches',()=>{
 const m=new FeedbackState();
 const r={...tank(.5),hasName:true,name:'Fuel cap',fluidName:'Fuel Tank',actions:1,context:2,carrying:false,notice:'',condition:-1,throwCharge:0,throwActive:false};
 m.update(r);
 assert.equal(m.targetName,'Fuel cap');
 assert.match(m.fluidText,/Fuel Tank:/);
 assert.equal(m.update({...r,name:'Oil cap'}),true);
 assert.equal(m.targetName,'Oil cap');
 assert.match(m.fluidText,/Fuel Tank:/);
 assert.equal(m.update({...r,name:'Oil cap',fluidName:'Oil Tank'}),true);
 assert.match(m.fluidText,/Oil Tank:/);
});

test('light_switch_uses_source_icon_without_changing_mechanism_feedback',()=>{
 assert.equal(selectIcon(1,10,false),'light_switch');
 assert.equal(selectIcon(2,7,false),'door');
 assert.equal(selectIcon(64|1,2,false),'fill');
});

test('radio_feedback_uses_source_values_without_playback',async()=>{
 const {createAudio,installAudioBridge}=await import('../../web/audio.js');
 const a=createAudio({}),bridge={};installAudioBridge(bridge,a);
 bridge.gameAudioRadio(false,false,94.5,.3,0,0,0,0,0,0,0,0,0);
 const m=new FeedbackState(),r={...tank(0),noText:true,name:'RadioColVolume',actions:128,context:11,radioVolume:a.radioFrame.volume};
 assert.equal(m.update(r),true);assert.equal(m.fluidText,'Volume: 03');assert.equal(m.targetName,'Volume +/-');assert.equal(m.update(r),false);
 r.context=12;r.radioFrequency=a.radioFrame.frequency;r.radioAm=false;m.update(r);assert.equal(m.fluidText,'FM 94.5');assert.equal(m.targetName,'Tune');
 r.radioAm=true;r.radioFrequency=630;m.update(r);assert.equal(m.fluidText,'AM 630');
 r.radioFrequency=undefined;m.update(r);assert.equal(m.fluidText,'');
 r.context=2;m.update(r);assert.equal(m.targetName,'RadioColVolume');assert.equal(m.fluidText,'');a.dispose();
});
test('source_paint_count_is_bare_and_hidden_during_carry',()=>{
 const r={...tank(37.5,100),context:14,hasName:true,fluidName:'Paint'};
 assert.equal(formatTank(r),'38');
 assert.equal(formatTank({...r,amount:0}),'0');
 assert.equal(formatTank({...r,noText:true}),'');
 const state=new FeedbackState(); state.update({...r,actions:4,carrying:false,name:'Paint'});
 assert.equal(state.fluidText,'38');
 state.update({...r,noText:true,actions:8,carrying:true,name:'Paint'});
 assert.equal(state.fluidText,'');
});
