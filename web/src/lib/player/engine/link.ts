// Видео по ссылке (Rutube, VK, свой сайт, прямой файл): поток идёт через сервер. Прямой файл - обычный
// `<video src>`; HLS - родной (Safari) или hls.js полной сборки (у внешних плейлистов звук часто в
// отдельных дорожках, лёгкая сборка их не знает).
import type { Engine } from '../session.svelte';
import type { Step } from '../plan';
import { createHlsEngine, type HlsEngine } from './hls';
import { createDashEngine } from './dash';
import { sourceTrackName, distinctNames } from '../tracks';

export interface LinkCtx {
  url: string;
  hls: boolean;
  dash?: boolean;
  nativeHls: boolean;
  /** С какого уровня начать (-1 - автоматически). */
  startLevel?: number;
}

/** Движок видео по ссылке; для HLS через hls.js - с уровнями качества. */
export function createLinkEngine(ctx: LinkCtx): Engine & Partial<HlsEngine> {
  if (ctx.dash) return createDashEngine(ctx.url);
  if (ctx.hls && !ctx.nativeHls) {
    return createHlsEngine({ source: ctx.url, light: false, startLevel: ctx.startLevel });
  }
  let video: HTMLVideoElement | null = null;
  let cleanup=()=>{};
  let audioCb:Parameters<HlsEngine['onAudioTracks']>[0]|null=null;
  let subsCb:Parameters<HlsEngine['onSubtitleTracks']>[0]|null=null;
  type AudioList=EventTarget&{length:number;[index:number]:{label:string;language:string;enabled:boolean}};
  const audios=()=> (video as (HTMLVideoElement&{audioTracks?:AudioList})|null)?.audioTracks;
  const publish=()=>{
    const tracks=audios();
    const names=tracks?distinctNames(Array.from({length:tracks.length},(_,index)=>({index,label:sourceTrackName(tracks[index].label,tracks[index].language,index)}))):[];
    audioCb?.(names.map(t=>({ ...t, lang:tracks?.[t.index]?.language||null, default:!!tracks?.[t.index]?.enabled })));
    subsCb?.(video?.textTracks?Array.from(video.textTracks).filter(t=>t.kind==='subtitles'||t.kind==='captions').map(t=>({id:`native:${Array.from(video!.textTracks).indexOf(t)}`,label:t.label||t.language||'Субтитры',lang:t.language||null,forced:false})):[]);
  };
  return {
    async start(v: HTMLVideoElement, _step: Step, at: number) {
      cleanup();video=v;let sought=false;
      const meta=()=>{if(!sought&&at>0){v.currentTime=at;sought=true;}publish();};
      v.addEventListener('loadedmetadata',meta);
      const lists=[audios(),v.textTracks].filter((l):l is AudioList|TextTrackList=>!!l);
      for(const l of lists){l.addEventListener?.('addtrack',publish);l.addEventListener?.('removetrack',publish);}
      cleanup=()=>{v.removeEventListener('loadedmetadata',meta);for(const l of lists){l.removeEventListener?.('addtrack',publish);l.removeEventListener?.('removetrack',publish);}};
      v.src = ctx.url;
    },
    destroy() {
      cleanup();
      if (video) {
        video.removeAttribute('src');
        video.load();
      }
      video = null;
    },
    onAudioTracks(cb){audioCb=cb;},setAudioTrack(index){const tracks=audios();if(tracks&&index>=0&&index<tracks.length)for(let i=0;i<tracks.length;i++)tracks[i].enabled=i===index;},
    onSubtitleTracks(cb){subsCb=cb;},setSubtitleTrack(index){if(video?.textTracks)for(let i=0;i<video.textTracks.length;i++)video.textTracks[i].mode=i===index?'showing':'disabled';},
    onFatal() {
      /* ошибки ловит PlaySession по событию error у <video> */
    },
  };
}
