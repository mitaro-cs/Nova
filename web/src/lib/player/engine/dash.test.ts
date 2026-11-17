import {describe,it,expect} from 'vitest';
import {createDashEngine,type DashPlayer,type DashModule} from './dash';

class Fake implements DashPlayer {
  handlers=new Map<string,(data?:unknown)=>void>();settings:Record<string,unknown>[]=[];quality=-1;audio='ru';text=-1;visible=false;closed=false;start=0;
  initialize(_v:HTMLVideoElement,_url:string,_auto:boolean,start:number){this.start=start;}
  on(event:string,cb:(data?:unknown)=>void){this.handlers.set(event,cb);}
  updateSettings(s:Record<string,unknown>){this.settings.push(s);}
  getRepresentationsByType(){return [{id:'4k',height:2160,bandwidth:8e6},{id:'hd-a',height:1080,bandwidth:4e6},{id:'hd-b',height:1080,bandwidth:2e6}];}
  getTracksFor(type:string){return type==='audio'?[{id:'ru',lang:'ru',labels:[{text:'Русский'}]},{id:'en',lang:'en',labels:[{text:'Original'}]}]:[{id:'cc',lang:'ru',labels:[{text:'Русские'}]}];}
  getCurrentTrackFor(){return {id:this.audio};}setCurrentTrack(track:{id?:string}){this.audio=track.id??'';}
  setRepresentationForTypeByIndex(_type:string,index:number){this.quality=index;}
  enableText(enabled:boolean){this.visible=enabled;}setTextTrack(index:number){this.text=index;}reset(){this.closed=true;}
}
describe('внешний DASH',()=>{
  it('сохраняет все качества, озвучки, субтитры и позицию',async()=>{
    const p=new Fake(),e=createDashEngine('/movie.mpd',async()=>({create:()=>p,events:{STREAM_INITIALIZED:'ready',ERROR:'error'}}));
    let audio:string[]=[],subs:string[]=[];
    e.onAudioTracks(t=>audio=t.map(a=>a.label));e.onSubtitleTracks(t=>subs=t.map(s=>s.id));
    await e.start({}as HTMLVideoElement,{mode:'raw',quality:'auto',why:''},90);p.handlers.get('ready')?.();
    expect(p.start).toBe(90);expect(e.levels().map(l=>l.i)).toEqual([0,1,2]);
    for(let i=0;i<3;i++){e.setLevel(i);expect(p.quality).toBe(i);}
    e.setLevel(-1);expect(p.settings.at(-1)).toEqual({streaming:{abr:{autoSwitchBitrate:{video:true}}}});
    expect(audio).toEqual(['Русский','Original']);e.setAudioTrack(1);expect(p.audio).toBe('en');
    expect(subs).toEqual(['dash:0']);e.setSubtitleTrack(0);expect(p.visible).toBe(true);expect(p.text).toBe(0);
    e.setSubtitleTrack(-1);expect(p.visible).toBe(false);e.destroy();expect(p.closed).toBe(true);
  });
  it('не запускается после закрытия во время ленивой загрузки',async()=>{
    const p=new Fake();let release!:(value:DashModule)=>void;
    const e=createDashEngine('/movie.mpd',()=>new Promise<DashModule>(r=>release=r));
    const start=e.start({}as HTMLVideoElement,{mode:'raw',quality:'auto',why:''},0);e.destroy();release({create:()=>p,events:{STREAM_INITIALIZED:'ready',ERROR:'error'}});await start;
    expect(p.handlers.size).toBe(0);
  });
});
