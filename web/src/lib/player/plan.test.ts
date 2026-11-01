import { describe, it, expect } from 'vitest';
import { buildChain, detectCaps, START_TIMEOUT_MS, STALL_TIMEOUT_MS, type Caps } from './plan';
import type { PlayInfo } from '$lib/types';

type Play = Partial<PlayInfo['play']>;
const info = (play: Play = {}, hls = true): PlayInfo =>
  ({
    play: { direct: true, remux: true, transcode: false, hevc: false, tenBit: false, duration: 100, width: 1920, height: 1080, audio: [], subs: [], ...play },
    hls,
  }) as unknown as PlayInfo;

const chromium: Caps = { nativeHls: false, hevcMain: true, hevcMain10: false };
const safari: Caps = { nativeHls: true, hevcMain: true, hevcMain10: true };
const orig = { quality: 'orig' as const, audio: 0, slow: false };
const modes = (c: ReturnType<typeof buildChain>) => c.map((s) => s.mode);

describe('buildChain', () => {
  it('Chromium + MP4 H.264: файл как есть, запасной адаптивный', () => {
    expect(modes(buildChain(info(), chromium, orig))).toEqual(['raw', 'hls']);
  });

  it('Chromium + MKV: перепаковка, запасной адаптивный', () => {
    expect(modes(buildChain(info({ direct: false }), chromium, orig))).toEqual(['remux', 'hls']);
  });

  it('Safari + MKV: копия HLS, запасной адаптивный', () => {
    expect(modes(buildChain(info({ direct: false }), safari, orig))).toEqual(['copy', 'hls']);
  });

  it('HEVC 10 бит без поддержки: оригинал недоступен, только адаптивный', () => {
    expect(modes(buildChain(info({ direct: false, hevc: true, tenBit: true }), chromium, orig))).toEqual(['hls']);
  });

  it('HEVC 10 бит с поддержкой: оригинал напрямую недоступен только из-за контейнера', () => {
    expect(modes(buildChain(info({ direct: true, hevc: true, tenBit: true }), safari, orig))).toEqual(['raw', 'hls']);
  });

  it('вторая звуковая дорожка у MP4: перепаковка', () => {
    expect(modes(buildChain(info(), chromium, { ...orig, audio: 1 }))).toEqual(['remux', 'hls']);
  });

  it('нет адаптивного потока, файл прямой: только raw', () => {
    expect(modes(buildChain(info({}, false), chromium, orig))).toEqual(['raw']);
  });

  it('нет адаптивного потока и нельзя перепаковать: raw, а не пусто', () => {
    expect(modes(buildChain(info({ direct: false, remux: false }, false), chromium, orig))).toEqual(['raw']);
  });

  it('уже подвисало: начинаем с адаптивного, оригинал - запасным', () => {
    expect(modes(buildChain(info(), chromium, { ...orig, slow: true }))).toEqual(['hls', 'raw']);
  });

  it('выбрано качество 720: адаптивный 720 первым, оригинал запасным', () => {
    const chain = buildChain(info(), chromium, { quality: 720, audio: 0, slow: false });
    expect(chain[0]).toMatchObject({ mode: 'hls', quality: 720 });
    expect(modes(chain)).toEqual(['hls', 'raw']);
  });

  it('качество выбрано, но адаптивного потока нет: оригинал', () => {
    expect(modes(buildChain(info({}, false), chromium, { quality: 720, audio: 0, slow: false }))).toEqual(['raw']);
  });

  it('никогда не пустая цепочка', () => {
    for (const direct of [true, false])
      for (const remux of [true, false])
        for (const hls of [true, false])
          for (const caps of [chromium, safari]) {
            const c = buildChain(info({ direct, remux }, hls), caps, orig);
            expect(c.length, JSON.stringify({ direct, remux, hls })).toBeGreaterThan(0);
          }
  });
});

describe('detectCaps', () => {
  const video = (hls: boolean) => ({ canPlayType: (t: string) => (t.includes('mpegurl') ? (hls ? 'maybe' : '') : t.includes('hvc1') ? 'probably' : '') }) as unknown as HTMLVideoElement;

  it('Safari: родной HLS', () => {
    expect(detectCaps(video(true), 'Mozilla/5.0 (Macintosh) AppleWebKit/605 Version/17 Safari/605').nativeHls).toBe(true);
  });

  it('Chrome с MediaSource: HLS не родной, даже если canPlayType отвечает', () => {
    (globalThis as Record<string, unknown>).MediaSource = class {};
    expect(detectCaps(video(true), 'Mozilla/5.0 Chrome/120 Safari/537').nativeHls).toBe(false);
    delete (globalThis as Record<string, unknown>).MediaSource;
  });

  it('HEVC определяется по canPlayType', () => {
    const c = detectCaps(video(false), 'x');
    expect(c.hevcMain).toBe(true);
    expect(c.hevcMain10).toBe(true);
  });
});

describe('пороги', () => {
  it('старт 8 с, ожидание посреди фильма 3 с', () => {
    expect(START_TIMEOUT_MS).toBe(8000);
    expect(STALL_TIMEOUT_MS).toBe(3000);
  });
});
