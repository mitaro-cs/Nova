// События плеера - в журнал приложения («Управление - Обзор»): какой режим стартовал, куда перешли и
// почему, зависания, окончательные сбои. Короткие строки без адресов; частые события не дублируются.
import { report as send, type DiagKind } from '$lib/diag';
import type { PlayEvent } from './session.svelte';

const CODES: Record<string, string> = {
  timeout: 'таймаут старта',
  network: 'сеть',
  decode: 'ошибка воспроизведения',
  no_variant: 'нет подходящего режима',
};

/** Один раз в 2 секунды на тип события (сбой - всегда). */
const MIN_GAP_MS = 2000;

function text(e: PlayEvent): string {
  const code = e.code ? (CODES[e.code] ?? e.code) : '';
  switch (e.type) {
    case 'start':
      return `старт: ${e.mode}`;
    case 'fallback':
      return `переход: ${e.mode} -> ${e.to} (${code})`;
    case 'stall':
      return e.to ? `зависание: ${e.mode} -> ${e.to}` : `зависание: ${e.mode}`;
    case 'fatal':
      return `сбой: ${e.code}, режим ${e.mode}`;
  }
}

export function createEventReporter(post: (kind: DiagKind, message: string) => void, now: () => number = Date.now) {
  const last = new Map<PlayEvent['type'], number>();
  return (e: PlayEvent, ctx: { browser: string }) => {
    if (e.type !== 'fatal') {
      const at = now();
      const prev = last.get(e.type);
      if (prev !== undefined && at - prev < MIN_GAP_MS) return;
      last.set(e.type, at);
    }
    post('player', `${text(e)} [${ctx.browser}]`);
  };
}

/** Короткое имя браузера для журнала. */
export function browserName(ua = typeof navigator === 'undefined' ? '' : navigator.userAgent): string {
  if (/Edg\//.test(ua)) return 'edge';
  if (/Chrome\//.test(ua)) return 'chrome';
  if (/Firefox\//.test(ua)) return 'firefox';
  if (/Safari\//.test(ua)) return 'safari';
  return 'другой';
}

export const reportEvent = createEventReporter(send);
