// Один предсказуемый путь выбора режима воспроизведения: по данным файла (`/api/files/{id}/info`),
// возможностям браузера и настройкам зрителя строится упорядоченная цепочка вариантов. Остальной код
// плеера режим не выбирает, а идёт по цепочке: не заработал первый - следующий, с того же места.
import type { PlayInfo } from '$lib/types';
import { hevcSupport } from './codecs';

/** Старт без первого кадра дольше этого - следующий вариант цепочки. */
export const START_TIMEOUT_MS = 8000;
/** Ожидание посреди фильма дольше этого (не из-за перемотки) - ступень качества вниз. */
export const STALL_TIMEOUT_MS = 3000;

export interface Caps {
  /** HLS родной (Safari, окно приложения на Mac): им готовая копия кусочками, без hls.js. */
  nativeHls: boolean;
  hevcMain: boolean;
  hevcMain10: boolean;
}

export type Mode = 'raw' | 'remux' | 'copy' | 'hls';
export type Quality = 'orig' | 'auto' | number;

export interface Step {
  mode: Mode;
  quality: Quality;
  /** Почему этот вариант (для журнала). */
  why: string;
}

export interface Pref {
  quality: Quality;
  audio: number;
  /** Уже подвисало у этого зрителя: начинаем с адаптивного качества. */
  slow: boolean;
}

/** Возможности этого браузера. */
export function detectCaps(video?: HTMLVideoElement, ua?: string): Caps {
  const v = video ?? (typeof document === 'undefined' ? null : document.createElement('video'));
  const agent = ua ?? (typeof navigator === 'undefined' ? '' : navigator.userAgent);
  const hevc = hevcSupport(v ? (t) => v.canPlayType(t) : null);
  const nativeHls =
    !!v && v.canPlayType('application/vnd.apple.mpegurl') !== '' && !('MediaSource' in globalThis && /Chrome|Android/.test(agent));
  return { nativeHls, hevcMain: hevc.main, hevcMain10: hevc.main10 };
}

/** Цепочка вариантов от лучшего к запасному. Никогда не пустая. */
export function buildChain(info: PlayInfo, caps: Caps, pref: Pref): Step[] {
  const p = info.play;
  const hevcOk = !p.hevc || (p.tenBit ? caps.hevcMain10 : caps.hevcMain);
  const direct = (p.direct && pref.audio === 0 && hevcOk) || !p.remux;
  // «Оригинал» годится: прямой файл, или перепаковка без перекодирования, или иначе нечем (нет адаптивного).
  const origOk = direct || (p.remux && !p.transcode && hevcOk) || (!p.remux && !info.hls);
  const wantHls = info.hls && (pref.quality !== 'orig' || pref.slow || !origOk);

  const chain: Step[] = [];
  if (wantHls) {
    const quality = pref.quality === 'orig' ? 'auto' : pref.quality;
    chain.push({ mode: 'hls', quality, why: pref.slow ? 'уже подвисало' : pref.quality !== 'orig' ? 'выбрано качество' : 'оригинал недоступен' });
  }
  if (origOk || !info.hls) {
    if (direct) chain.push({ mode: 'raw', quality: 'orig', why: 'файл как есть' });
    else if (caps.nativeHls) chain.push({ mode: 'copy', quality: 'orig', why: 'копия HLS без перекодирования' });
    else chain.push({ mode: 'remux', quality: 'orig', why: 'перепаковка потока' });
  }
  if (info.hls && !wantHls) chain.push({ mode: 'hls', quality: 'auto', why: 'запасной адаптивный' });
  return chain;
}
