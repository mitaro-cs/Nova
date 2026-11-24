import {afterEach,expect,it,vi} from 'vitest';
import {createVolumeControl} from './volume';
afterEach(()=>vi.unstubAllGlobals());
it('усиливает сигнал до x2 и создаёт граф один раз',async()=>{
  const gain={gain:{setValueAtTime:vi.fn()},connect:vi.fn(),disconnect:vi.fn()};
  const media=vi.fn(()=>({connect:vi.fn()})),close=vi.fn(async()=>{});
  class Context{state='running';currentTime=0;destination={};resume=async()=>{};createGain=()=>gain;createMediaElementSource=media;close=close;}
  vi.stubGlobal('window',{AudioContext:Context});
  const video={volume:1,muted:false} as HTMLVideoElement;const control=createVolumeControl(video);
  expect(await control.set(2)).toBe(2);expect(video.volume).toBe(1);expect(gain.gain.setValueAtTime).toHaveBeenLastCalledWith(2,0);
  expect(await control.set(.5)).toBe(.5);expect(video.volume).toBe(.5);expect(gain.gain.setValueAtTime).toHaveBeenLastCalledWith(1,0);
  await control.set(0);expect(video.muted).toBe(true);await control.set(2);expect(media).toHaveBeenCalledTimes(1);
  control.destroy();expect(close).toHaveBeenCalledTimes(1);
});
it('при запрете Web Audio не показывает фиктивные 200%',async()=>{
  class Context{constructor(){throw new Error('Нет устройства');}}
  vi.stubGlobal('window',{AudioContext:Context});const video={volume:1,muted:false} as HTMLVideoElement;
  expect(await createVolumeControl(video).set(2)).toBe(1);expect(video.volume).toBe(1);
});
it('ночной звук: компрессор сжимает и поднимает громкость, днём - нейтрален',async()=>{
  const param=()=>({setValueAtTime:vi.fn()});
  const gain={gain:param(),connect:vi.fn(),disconnect:vi.fn()};
  const comp={threshold:param(),ratio:param(),knee:param(),attack:param(),release:param(),connect:vi.fn(),disconnect:vi.fn()};
  class Context{state='running';currentTime=0;destination={};resume=async()=>{};createGain=()=>gain;createDynamicsCompressor=()=>comp;createMediaElementSource=()=>({connect:vi.fn()});close=async()=>{};}
  vi.stubGlobal('window',{AudioContext:Context});
  const control=createVolumeControl({volume:1,muted:false} as HTMLVideoElement);
  expect(await control.night(true)).toBe(true);
  expect(comp.threshold.setValueAtTime).toHaveBeenLastCalledWith(-38,0);
  expect(comp.ratio.setValueAtTime).toHaveBeenLastCalledWith(8,0);
  expect(gain.gain.setValueAtTime).toHaveBeenLastCalledWith(1.7,0);
  await control.night(false);
  expect(comp.ratio.setValueAtTime).toHaveBeenLastCalledWith(1,0);
  expect(gain.gain.setValueAtTime).toHaveBeenLastCalledWith(1,0);
});
