// Адаптивный режим (hls) и видео по ссылке с HLS: hls.js, подгружается лениво. Параметры загрузки и
// повторов - те же, что у прежнего плеера (хост готовит плейлист несколько секунд, интернет бывает плохим).
import type { Engine } from '../session.svelte';
import type { Step } from '../plan';
import { qualityTier, uniqueLevels } from '../quality-label';
import { abrEstimate } from '../speed';
import { distinctNames, sourceTrackName } from '../tracks';
import { streamUrl } from '../urls';

export interface Level {
  i: number;
  height: number;
  label?: number;
  display?: string;
}

/** Минимум, который мы берём от hls.js (и что подменяем в тестах). */
export interface HlsLike {
  new (config: Record<string, unknown>): HlsInstance;
  isSupported(): boolean;
  Events: { MANIFEST_PARSED: string; LEVEL_SWITCHED: string; ERROR: string; AUDIO_TRACKS_UPDATED?: string; SUBTITLE_TRACKS_UPDATED?: string };
  ErrorTypes: { MEDIA_ERROR: string; NETWORK_ERROR: string };
}

export interface HlsInstance {
  levels: { height: number; width: number; bitrate?: number; videoCodec?: string }[];
  audioTracks?: { name?: string; lang?: string; default?: boolean }[];
  audioTrack?: number;
  subtitleTracks?: {name?:string;lang?:string;forced?:boolean}[];
  subtitleTrack?: number;
  subtitleDisplay?: boolean;
  currentLevel: number;
  loadLevel: number;
  on(event: string, cb: (event: string, data: never) => void): void;
  loadSource(url: string): void;
  attachMedia(video: HTMLVideoElement): void;
  destroy(): void;
  startLoad(): void;
  recoverMediaError(): void;
  pauseBuffering?(): void;
  resumeBuffering?(): void;
}

export interface HlsCtx {
  fileId?: number;
  audio?: number;
  /** Скорость до хоста, Мбит/с: с неё начинается автокачество. */
  mbit?: number | null;
  /** Видео по ссылке: адрес потока вместо файла хоста (запас вперёд меньше, нужна полная сборка). */
  source?: string;
  /** Лёгкая сборка hls.js (без отдельных звуковых дорожек). */
  light?: boolean;
  /** С какого уровня начать (видео по ссылке): -1 - автоматически. */
  startLevel?: number;
  loader?: () => Promise<{ default: HlsLike }>;
}

export interface HlsEngine extends Engine {
  levels(): Level[];
  setLevel(i: number): void;
  onLevels(cb: (levels: Level[]) => void): void;
  /** Высота текущего уровня (или null). */
  onLevel(cb: (height: number | null) => void): void;
  onAudioTracks(cb: (tracks: { index: number; label: string; lang: string | null; default: boolean }[]) => void): void;
  setAudioTrack(index: number): void;
  onSubtitleTracks(cb: (tracks: {id:string;label:string;lang:string|null;forced:boolean}[]) => void): void;
  setSubtitleTrack(index: number): void;
  /** На паузе hls.js не качает вперёд (экономит интернет и нагрузку на хосте). */
  pauseBuffering(): void;
  resumeBuffering(): void;
}

const policy = (ttfb: number, load: number, timeoutRetry: [number, number, number], errorRetry: [number, number, number]) => ({
  default: {
    maxTimeToFirstByteMs: ttfb,
    maxLoadTimeMs: load,
    timeoutRetry: { maxNumRetry: timeoutRetry[0], retryDelayMs: timeoutRetry[1], maxRetryDelayMs: timeoutRetry[2] },
    errorRetry: { maxNumRetry: errorRetry[0], retryDelayMs: errorRetry[1], maxRetryDelayMs: errorRetry[2] },
  },
});

const defaultLoader = (light: boolean) => () =>
  (light ? import('hls.js/light') : import('hls.js')) as unknown as Promise<{ default: HlsLike }>;

export function createHlsEngine(ctx: HlsCtx): HlsEngine {
  let hls: HlsInstance | null = null;
  let destroyed = false;
  let fatalCb: ((code: string) => void) | null = null;
  let levelsCb: ((l: Level[]) => void) | null = null;
  let levelCb: ((h: number | null) => void) | null = null;
  let tracksCb: ((tracks: { index: number; label: string; lang: string | null; default: boolean }[]) => void) | null = null;
  let subtitlesCb: ((tracks: {id:string;label:string;lang:string|null;forced:boolean}[]) => void) | null = null;
  let current: Level[] = [];

  return {
    async start(video: HTMLVideoElement, step: Step, at: number) {
      destroyed = false;
      const load = ctx.loader ?? defaultLoader(ctx.light ?? true);
      const { default: Hls } = await load();
      if (destroyed) return;
      if (!Hls.isSupported()) {
        fatalCb?.('decode');
        return;
      }
      const url = ctx.source ?? streamUrl(step, { fileId: ctx.fileId ?? 0, audio: ctx.audio ?? 0, at, hevc: false }).url;
      const estimate = ctx.source ? undefined : abrEstimate(ctx.mbit ?? null);
      let retries = 0;
      const h = new Hls({
        startPosition: at,
        // Запас вперёд - до минуты: короткие провалы скорости не останавливают видео.
        maxBufferLength: ctx.source ? 20 : 60,
        maxMaxBufferLength: 120,
        startLevel: ctx.startLevel ?? -1,
        // Автокачество начинает с уровня под замеренную скорость, а не наугад.
        ...(estimate === undefined ? {} : { abrEwmaDefaultEstimate: estimate }),
        capLevelToPlayerSize: true,
        backBufferLength: 60,
        // Плейлист у хоста может готовиться несколько секунд (копия, перекодирование).
        manifestLoadPolicy: policy(60_000, 90_000, [2, 1000, 4000], [3, 1000, 8000]),
        playlistLoadPolicy: policy(60_000, 90_000, [2, 1000, 4000], [3, 1000, 8000]),
        fragLoadPolicy: policy(45_000, 120_000, [4, 500, 4000], [6, 1000, 8000]),
      }) as HlsInstance;
      hls = h;
      if (ctx.source) h.subtitleDisplay = false;
      h.on(Hls.Events.LEVEL_SWITCHED, ((_e: string, d: { level: number }) => levelCb?.(h.levels[d.level]?.height ?? null)) as never);
      h.on(Hls.Events.MANIFEST_PARSED, (() => {
        // Одно качество - одна строка (у сайтов бывает по три варианта 720p): лучший битрейт на ступень.
        current = uniqueLevels(h.levels.map((l, i) => ({ i, height: l.height, width: l.width, bitrate: l.bitrate, label: qualityTier(l.height, l.width) })))
          .map(({ i, height, label }) => ({ i, height, label }));
        levelsCb?.(current);
        if (ctx.source) {
          const names = distinctNames((h.audioTracks ?? []).map((t, index) => ({ index, label: sourceTrackName(t.name, t.lang, index) })));
          tracksCb?.(names.map((t, index) => ({ ...t, lang: h.audioTracks?.[index]?.lang || null, default: !!h.audioTracks?.[index]?.default })));
        }
        if (ctx.source) subtitlesCb?.((h.subtitleTracks ?? []).map((t,index) => ({id:`hls:${index}`,label:t.name || t.lang || `Субтитры ${index+1}`,lang:t.lang || null,forced:!!t.forced})));
      }) as never);
      if (ctx.source && Hls.Events.AUDIO_TRACKS_UPDATED) h.on(Hls.Events.AUDIO_TRACKS_UPDATED, (() => {
        const names=distinctNames((h.audioTracks ?? []).map((t,index)=>({index,label:sourceTrackName(t.name,t.lang,index)})));
        tracksCb?.(names.map((t,index)=>({...t,lang:h.audioTracks?.[index]?.lang||null,default:!!h.audioTracks?.[index]?.default})));
      }) as never);
      if (ctx.source && Hls.Events.SUBTITLE_TRACKS_UPDATED) h.on(Hls.Events.SUBTITLE_TRACKS_UPDATED, (() => {
        subtitlesCb?.((h.subtitleTracks ?? []).map((t,index) => ({id:`hls:${index}`,label:t.name || t.lang || `Субтитры ${index+1}`,lang:t.lang || null,forced:!!t.forced})));
      }) as never);
      h.on(Hls.Events.ERROR, ((_e: string, d: { fatal?: boolean; type?: string }) => {
        if (!d.fatal) return;
        if (retries++ < 3) {
          if (d.type === Hls.ErrorTypes.MEDIA_ERROR) h.recoverMediaError();
          else h.startLoad();
          return;
        }
        fatalCb?.(d.type === Hls.ErrorTypes.MEDIA_ERROR ? 'decode' : 'network');
      }) as never);
      h.loadSource(url);
      h.attachMedia(video);
    },
    destroy() {
      destroyed = true;
      hls?.destroy();
      hls = null;
      current = [];
    },
    onFatal(cb) {
      fatalCb = cb;
    },
    levels: () => current,
    setLevel(i) {
      if (!hls) return;
      hls.currentLevel = i;
      hls.loadLevel = i;
    },
    onLevels(cb) {
      levelsCb = cb;
    },
    onLevel(cb) {
      levelCb = cb;
    },
    onAudioTracks(cb) { tracksCb = cb; },
    setAudioTrack(index) { if (hls) hls.audioTrack = index; },
    onSubtitleTracks(cb) { subtitlesCb = cb; },
    setSubtitleTrack(index) { if(hls) { hls.subtitleDisplay = index >= 0; hls.subtitleTrack = index; } },
    pauseBuffering() {
      hls?.pauseBuffering?.();
    },
    resumeBuffering() {
      hls?.resumeBuffering?.();
    },
  };
}
