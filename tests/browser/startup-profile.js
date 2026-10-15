const result=document.querySelector('#result');
const frame=document.querySelector('iframe');
let saved=false;
const timer=setInterval(async()=>{
 if(saved||!frame.contentWindow?.gameStartupDone?.())return;
 const trace=frame.contentWindow.gameStartupTrace();
 saved=true;clearInterval(timer);
 try{
  const response=await fetch('/__tests__/startup-trace',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(trace)});
  if(!response.ok)throw Error(`HTTP ${response.status}`);
  const path=await response.text();
  result.textContent=`${trace.failure?'Failed':'Ready'}: ${path}`;
 }catch(error){result.textContent=`Could not save trace: ${error.message}`;}
},250);
