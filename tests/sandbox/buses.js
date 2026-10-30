// ROADMAP test pass, "More cars", bus steps. Open `?sandbox&cars=Bus01,Bus02,Bus03`, paste helpers.js, start a New game,
// then paste this file. Each `__buses.<step>()` records one step; take a screenshot after each.
// Steps that need eyes or a seat end with "Tomasz judges": Claude drives them by hand with the key helpers.
window.__buses={
 spawned:()=>__step('Bus01, Bus02 and Bus03 stand in front of the garage',async()=>{
  const names=(await sandbox.state(80)).data.cars.map(c=>c.name);
  return ['Bus01','Bus02','Bus03'].map(b=>b+(names.some(n=>n.toLowerCase()===b.toLowerCase())?' found':' MISSING')).join(', ');
 }),
 diesel:name=>__step(name+' runs on diesel',async()=>{
  const fill=await sandbox.fill('diesel',40);
  return fill.message+' Sit in the driver seat, ignition, start. Tomasz judges: engine runs and drives.';
 }),
 petrol:name=>__step(name+' does not start on petrol',async()=>{
  const fill=await sandbox.fill('petrol',40);
  return fill.message+' Start the engine: it must fail. Tomasz judges.';
 }),
 gauges:()=>__step('Bus01 gauges read speed',async()=>'Drive Bus01 at about 40 km/h. Both kmh gauges, the counters and the gauge lamps move. Tomasz judges.'),
 doors:()=>__step('Bus doors',async()=>'Signal button lights green, open button swings the leaves and lights red, leaf sounds play. With the ignition off the buttons do nothing. Tomasz judges.'),
 trailer:()=>__step('Bus02 trailer',async()=>'Put the trailer grip on the Bus02 hitch, drive and turn. Brake switch on: brake force 100 per wheel. Rear lamps follow lights and brake. Tomasz judges.'),
 save:()=>__step('Save and load with doors open and the trailer on',async()=>'Pause, Save, Load. Doors stay open and signalled, the trailer stays on the hitch. Tomasz judges.'),
};
'buses ok'
