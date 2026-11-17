import {it,expect} from 'vitest';
import {createLinkEngine} from './link';

it('переключает все дорожки нативного HLS и снимает слушатели',async()=>{
  const events=new EventTarget();
  const audio=Object.assign([{label:'Русский',language:'ru',enabled:true},{label:'English',language:'en',enabled:false}],{addEventListener:events.addEventListener.bind(events),removeEventListener:events.removeEventListener.bind(events)});
  const text=Object.assign([{kind:'subtitles',label:'Русские',language:'ru',mode:'disabled'},{kind:'captions',label:'English',language:'en',mode:'disabled'}],{addEventListener:events.addEventListener.bind(events),removeEventListener:events.removeEventListener.bind(events)});
  const video=Object.assign(new EventTarget(),{audioTracks:audio,textTracks:text,src:'',currentTime:0,removeAttribute(){this.src=''},load(){}});
  const e=createLinkEngine({url:'/master.m3u8',hls:true,nativeHls:true});let audios:string[]=[],subs:string[]=[];
  e.onAudioTracks?.(t=>audios=t.map(a=>a.label));e.onSubtitleTracks?.(t=>subs=t.map(s=>s.id));
  await e.start(video as unknown as HTMLVideoElement,{mode:'hls',quality:'auto',why:''},45);video.dispatchEvent(new Event('loadedmetadata'));
  expect(video.currentTime).toBe(45);expect(audios).toEqual(['Русский','English']);expect(subs).toEqual(['native:0','native:1']);
  e.setAudioTrack?.(1);expect(audio.map(a=>a.enabled)).toEqual([false,true]);
  e.setSubtitleTrack?.(1);expect(text.map(t=>t.mode)).toEqual(['disabled','showing']);
  e.setSubtitleTrack?.(-1);expect(text.map(t=>t.mode)).toEqual(['disabled','disabled']);
  e.destroy();audios=[];video.dispatchEvent(new Event('loadedmetadata'));expect(audios).toEqual([]);
});
