/** До 100% — штатная громкость, выше — реальное усиление Web Audio. */
export function createVolumeControl(video:HTMLVideoElement) {
  let context:AudioContext|null=null, gain:GainNode|null=null, pending:Promise<boolean>|null=null;
  let requested=1, disposed=false;
  async function enable():Promise<boolean> {
    if(gain)return true;
    if(pending)return pending;
    pending=(async()=>{
      const Constructor=window.AudioContext??(window as Window & {webkitAudioContext?:typeof AudioContext}).webkitAudioContext;
      if(!Constructor)return false;
      let candidate:AudioContext|undefined;
      try {
        candidate=new Constructor();
        await candidate.resume();
        if(disposed){await candidate.close();return false;}
        const node=candidate.createGain();
        candidate.createMediaElementSource(video).connect(node);
        node.connect(candidate.destination);
        context=candidate;gain=node;return true;
      } catch {await candidate?.close().catch(()=>{});return false;}
    })().finally(()=>{pending=null;});
    return pending;
  }
  return {
    async set(value:number) {
      requested=Math.max(0,Math.min(2,value));
      if(requested>1&&!await enable())requested=1;
      if(disposed)return 1;
      const actual=requested;
      try {video.volume=Math.min(1,actual);}catch{/* Системная громкость iOS. */}
      if(gain&&context){gain.gain.setValueAtTime(Math.max(1,actual),context.currentTime);if(context.state==='suspended')void context.resume().catch(()=>{});}
      video.muted=actual===0;
      return actual;
    },
    resume(){if(context?.state==='suspended')void context.resume().catch(()=>{});},
    destroy(){disposed=true;gain?.disconnect();void context?.close().catch(()=>{});context=null;gain=null;},
  };
}
