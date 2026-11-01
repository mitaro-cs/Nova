// Видео по ссылке (Rutube, VK, свой сайт, прямой файл): поток идёт через сервер. Прямой файл - обычный
// `<video src>`; HLS - родной (Safari) или hls.js полной сборки (у внешних плейлистов звук часто в
// отдельных дорожках, лёгкая сборка их не знает).
import type { Engine } from '../session.svelte';
import type { Step } from '../plan';
import { createHlsEngine, type HlsEngine } from './hls';

export interface LinkCtx {
  url: string;
  hls: boolean;
  nativeHls: boolean;
  /** С какого уровня начать (-1 - автоматически). */
  startLevel?: number;
}

/** Движок видео по ссылке; для HLS через hls.js - с уровнями качества. */
export function createLinkEngine(ctx: LinkCtx): Engine & Partial<HlsEngine> {
  if (ctx.hls && !ctx.nativeHls) {
    return createHlsEngine({ source: ctx.url, light: false, startLevel: ctx.startLevel });
  }
  let video: HTMLVideoElement | null = null;
  let onMeta: (() => void) | null = null;
  const unhook = () => {
    if (video && onMeta) video.removeEventListener('loadedmetadata', onMeta);
    onMeta = null;
  };
  return {
    async start(v: HTMLVideoElement, _step: Step, at: number) {
      unhook();
      video = v;
      if (at > 0) {
        onMeta = () => {
          v.currentTime = at;
          unhook();
        };
        v.addEventListener('loadedmetadata', onMeta);
      }
      v.src = ctx.url;
    },
    destroy() {
      unhook();
      if (video) {
        video.removeAttribute('src');
        video.load();
      }
      video = null;
    },
    onFatal() {
      /* ошибки ловит PlaySession по событию error у <video> */
    },
  };
}
