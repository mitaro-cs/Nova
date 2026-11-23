import { uniqueLevels } from '../quality-label';
// DASH загружается только при открытии MPD. Навигация и локальная библиотека его не скачивают.
import type { HlsEngine, Level } from './hls';
import { sourceTrackName, distinctNames } from '../tracks';

interface Track { id?: string; index?: number; lang?: string; labels?: {text:string}[]; roles?: {value:string}[] }
interface Representation { id: string; height?: number; width?: number; bandwidth?: number }
export interface DashPlayer {
  initialize(video:HTMLVideoElement,url:string,autoplay:boolean,start:number):void;
  on(event:string,callback:(data?:unknown)=>void):void;
  updateSettings(settings:Record<string,unknown>):void;
  getRepresentationsByType(type:string):Representation[];
  getTracksFor(type:string):Track[];
  getCurrentTrackFor(type:string):Track|null;
  setCurrentTrack(track:Track):void;
  setRepresentationForTypeByIndex(type:string,index:number):void;
  enableText(enabled:boolean):void;
  setTextTrack(index:number):void;
  reset():void;
}
export interface DashModule { create:()=>DashPlayer; events:{STREAM_INITIALIZED:string;ERROR:string;QUALITY_CHANGE_RENDERED?:string} }

async function loadDash():Promise<DashModule> {
  const {MediaPlayer}=await import('dashjs');
  return {create:()=>MediaPlayer().create() as unknown as DashPlayer,events:MediaPlayer.events};
}

export function createDashEngine(url:string,loader=loadDash):HlsEngine {
  let player:DashPlayer|null=null, destroyed=false, current:Level[]=[];
  let levelsCb:Parameters<HlsEngine['onLevels']>[0]|null=null;
  let levelCb:Parameters<HlsEngine['onLevel']>[0]|null=null;
  let audioCb:Parameters<HlsEngine['onAudioTracks']>[0]|null=null;
  let subsCb:Parameters<HlsEngine['onSubtitleTracks']>[0]|null=null;
  let fatal:((code:string)=>void)|null=null;
  return {
    async start(video,_step,at) {
      destroyed=false;const dash=await loader();if(destroyed)return;
      const p=dash.create();player=p;
      p.updateSettings({streaming:{scheduling:{scheduleWhilePaused:false},abr:{autoSwitchBitrate:{video:true}}}});
      p.on(dash.events.STREAM_INITIALIZED,()=>{
        current=uniqueLevels(p.getRepresentationsByType('video').map((v,i)=>({i,height:v.height??0,width:v.width??undefined,bitrate:v.bandwidth??0,label:v.height??0})))
          .map(({i,height,label})=>({i,height,label}));
        levelsCb?.(current);
        const active=p.getCurrentTrackFor('audio');
        const tracks=p.getTracksFor('audio');
        const audioNames=distinctNames(tracks.map((t,index)=>({index,label:sourceTrackName(t.labels?.[0]?.text||t.roles?.[0]?.value,t.lang,index)})));
        audioCb?.(audioNames.map((t,index)=>({...t,lang:tracks[index]?.lang??null,default:!!active&&active.id===tracks[index]?.id})));
        subsCb?.(p.getTracksFor('text').map((t,index)=>({id:`dash:${index}`,label:t.labels?.[0]?.text||t.lang||`Субтитры ${index+1}`,lang:t.lang??null,forced:false})));
        p.enableText(false);
      });
      p.on(dash.events.ERROR,(data)=>{if(data)fatal?.('network');});
      p.initialize(video,url,false,at);
    },
    destroy(){destroyed=true;player?.reset();player=null;current=[];},
    onFatal(cb){fatal=cb;},levels:()=>current,
    onLevels(cb){levelsCb=cb;},onLevel(cb){levelCb=cb;},
    setLevel(index){if(!player)return;player.updateSettings({streaming:{abr:{autoSwitchBitrate:{video:index<0}}}});if(index>=0){player.setRepresentationForTypeByIndex('video',index);levelCb?.(current.find(l=>l.i===index)?.height??null);}},
    onAudioTracks(cb){audioCb=cb;},setAudioTrack(index){const t=player?.getTracksFor('audio')[index];if(t)player?.setCurrentTrack(t);},
    onSubtitleTracks(cb){subsCb=cb;},setSubtitleTrack(index){player?.enableText(index>=0);if(index>=0)player?.setTextTrack(index);},
    pauseBuffering(){/* dash.js приостанавливает загрузку на паузе через scheduleWhilePaused. */},
    resumeBuffering(){/* Планировщик продолжает загрузку при video.play(). */},
  };
}
