import { describe, it, expect } from 'vitest';
import { streamUrl } from './urls';

const ctx = { fileId: 7, audio: 0, at: 125.456, hevc: false };

describe('streamUrl', () => {
  it('raw: файл как есть, место - после загрузки', () => {
    expect(streamUrl({ mode: 'raw', quality: 'orig', why: '' }, ctx)).toEqual({ url: '/api/files/7/raw', offset: 0, seekAfterLoad: 125.456 });
  });

  it('remux: поток с места, смещение времени равно месту', () => {
    expect(streamUrl({ mode: 'remux', quality: 'orig', why: '' }, ctx)).toEqual({
      url: '/api/files/7/play?t=125.46&audio=0',
      offset: 125.456,
      seekAfterLoad: 0,
    });
  });

  it('remux: hevc=1 только когда браузер умеет HEVC', () => {
    const u = streamUrl({ mode: 'remux', quality: 'orig', why: '' }, { ...ctx, hevc: true, audio: 1 }).url;
    expect(u).toBe('/api/files/7/play?t=125.46&audio=1&hevc=1');
  });

  it('copy: HLS-копия с целой секунды', () => {
    expect(streamUrl({ mode: 'copy', quality: 'orig', why: '' }, ctx)).toEqual({
      url: '/api/files/7/copy/0/index.m3u8?from=125',
      offset: 0,
      seekAfterLoad: 125.456,
    });
  });

  it('hls авто: общий список, звук в запросе', () => {
    expect(streamUrl({ mode: 'hls', quality: 'auto', why: '' }, { ...ctx, audio: 2 }).url).toBe('/api/files/7/hls/master.m3u8?audio=2');
  });

  it('hls с высотой: список этой высоты', () => {
    expect(streamUrl({ mode: 'hls', quality: 720, why: '' }, ctx).url).toBe('/api/files/7/hls/720/index.m3u8?audio=0');
  });

  it('hls: место запоминается для hls.js, смещения нет', () => {
    const r = streamUrl({ mode: 'hls', quality: 'auto', why: '' }, ctx);
    expect(r.offset).toBe(0);
    expect(r.seekAfterLoad).toBe(125.456);
  });
});
