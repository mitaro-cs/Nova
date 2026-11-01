import type { Step } from './plan';

export interface StreamCtx {
  fileId: number;
  audio: number;
  /** Место просмотра, с (куда перемотать или откуда начать). */
  at: number;
  /** Браузер умеет HEVC: перепаковка не перекодирует видео. */
  hevc: boolean;
}

export interface StreamTarget {
  url: string;
  /** Время потока ffmpeg с нуля (перепаковка `t=`): прибавляется к `currentTime` для настоящего места. */
  offset: number;
  /** Перемотать `<video>` на это место после загрузки (0 - не нужно). */
  seekAfterLoad: number;
}

/** Адрес потока для шага цепочки. */
export function streamUrl(step: Step, ctx: StreamCtx): StreamTarget {
  const { fileId, audio, at } = ctx;
  switch (step.mode) {
    case 'raw':
      return { url: `/api/files/${fileId}/raw`, offset: 0, seekAfterLoad: at };
    case 'remux': {
      const q = new URLSearchParams({ t: at.toFixed(2), audio: String(audio) });
      if (ctx.hevc) q.set('hevc', '1');
      return { url: `/api/files/${fileId}/play?${q}`, offset: at, seekAfterLoad: 0 };
    }
    case 'copy':
      return { url: `/api/files/${fileId}/copy/${audio}/index.m3u8?from=${Math.floor(at)}`, offset: 0, seekAfterLoad: at };
    case 'hls':
      return {
        url:
          step.quality === 'auto' || step.quality === 'orig'
            ? `/api/files/${fileId}/hls/master.m3u8?audio=${audio}`
            : `/api/files/${fileId}/hls/${step.quality}/index.m3u8?audio=${audio}`,
        offset: 0,
        seekAfterLoad: at,
      };
  }
}
