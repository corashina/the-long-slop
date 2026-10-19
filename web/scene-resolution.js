export function installSceneResolution(target,select){
 let scale=1;
 select.value='1';
 select.addEventListener('change',()=>{scale=select.value==='0.75'?0.75:1;});
 target.gameSceneScale=()=>scale;
}
