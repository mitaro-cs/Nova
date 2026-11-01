// Нативные режимы: файл как есть (raw), перепаковка потоком (remux) и готовая копия HLS (copy,
// Safari/WebKit играют её сами). Движок только ставит адрес; ошибки `<video>` ловит `PlaySession`.
import type { Engine } from '../session.svelte';
import type { Step } from '../plan';
import { streamUrl } from '../urls';

export interface NativeCtx {
  fileId: number;
  audio: number;
  /** Браузер умеет HEVC (перепаковка не перекодирует видео). */
  hevc: boolean;
}

export interface NativeEngine extends Engine {
  /** Время потока с нуля (перепаковка): настоящее место = offset + currentTime. */
  readonly offset: number;
}

export function createNativeEngine(ctx: NativeCtx): NativeEngine {
  let video: HTMLVideoElement | null = null;
  let onMeta: (() => void) | null = null;
  let offset = 0;

  const unhook = () => {
    if (video && onMeta) video.removeEventListener('loadedmetadata', onMeta);
    onMeta = null;
  };

  return {
    get offset() {
      return offset;
    },
    async start(v: HTMLVideoElement, step: Step, at: number) {
      unhook();
      video = v;
      const target = streamUrl(step, { ...ctx, at });
      offset = target.offset;
      if (target.seekAfterLoad > 0) {
        const seek = target.seekAfterLoad;
        onMeta = () => {
          v.currentTime = seek;
          unhook();
        };
        v.addEventListener('loadedmetadata', onMeta);
      }
      v.src = target.url;
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
      /* ошибки нативного воспроизведения ловит PlaySession по событию error у <video> */
    },
  };
}
