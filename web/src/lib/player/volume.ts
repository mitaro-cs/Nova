/**
 * Громкость и звук плеера. До 100% - штатная громкость, выше - реальное усиление Web Audio. «Ночной звук»
 * (выбор владельца, 23.11) - компрессор в той же цепочке: тихие диалоги громче, взрывы тише. Цепочка Web Audio
 * собирается только когда нужна (громкость выше 100% или ночной звук): видео из неё уже не выходит, поэтому
 * без необходимости её не заводим.
 */
export function createVolumeControl(video:HTMLVideoElement) {
  let context:AudioContext|null=null, gain:GainNode|null=null, comp:DynamicsCompressorNode|null=null, pending:Promise<boolean>|null=null;
  let requested=1, night=false, disposed=false;
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
        const c=candidate.createDynamicsCompressor?.()??null;
        const source=candidate.createMediaElementSource(video);
        if(c){source.connect(c);c.connect(node);}else source.connect(node);
        node.connect(candidate.destination);
        context=candidate;gain=node;comp=c;applyNight();return true;
      } catch {await candidate?.close().catch(()=>{});return false;}
    })().finally(()=>{pending=null;});
    return pending;
  }
  /** Компрессор: ночью - порог -38 дБ, сжатие 8:1 и подъём громкости, днём - нейтральный (ничего не меняет). */
  function applyNight() {
    if(!comp||!context)return;
    const t=context.currentTime;
    comp.threshold.setValueAtTime(night?-38:0,t);
    comp.ratio.setValueAtTime(night?8:1,t);
    comp.knee.setValueAtTime(night?18:0,t);
    comp.attack.setValueAtTime(0.004,t);
    comp.release.setValueAtTime(0.3,t);
    if(gain)gain.gain.setValueAtTime(Math.max(1,requested)*(night?1.7:1),t);
  }
  return {
    async set(value:number) {
      requested=Math.max(0,Math.min(2,value));
      if(requested>1&&!await enable())requested=1;
      if(disposed)return 1;
      const actual=requested;
      try {video.volume=Math.min(1,actual);}catch{/* Системная громкость iOS. */}
      if(gain&&context){gain.gain.setValueAtTime(Math.max(1,actual)*(night?1.7:1),context.currentTime);if(context.state==='suspended')void context.resume().catch(()=>{});}
      video.muted=actual===0;
      return actual;
    },
    /** Включить или выключить ночной звук. false - браузер не дал собрать цепочку Web Audio. */
    async night(on:boolean) {
      night=on;
      if(on&&!await enable()){night=false;return false;}
      applyNight();
      return true;
    },
    resume(){if(context?.state==='suspended')void context.resume().catch(()=>{});},
    destroy(){disposed=true;gain?.disconnect();comp?.disconnect();void context?.close().catch(()=>{});context=null;gain=null;comp=null;},
  };
}
