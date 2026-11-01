import { describe, it, expect } from 'vitest';
import { createEventReporter } from './report';
import type { PlayEvent } from './session.svelte';

function setup() {
  const sent: { kind: string; message: string }[] = [];
  let t = 1_000_000;
  const emit = createEventReporter(
    (kind, message) => sent.push({ kind, message }),
    () => t,
  );
  return { sent, emit, tick: (ms: number) => (t += ms) };
}
const ctx = { browser: 'chrome' };

describe('reportEvent', () => {
  it('формулирует каждый тип события без адресов', () => {
    const { sent, emit, tick } = setup();
    const events: PlayEvent[] = [
      { type: 'start', mode: 'remux' },
      { type: 'fallback', mode: 'remux', to: 'hls', code: 'timeout' },
      { type: 'stall', mode: 'raw', to: 'hls' },
      { type: 'fatal', mode: 'hls', code: 'network' },
    ];
    for (const e of events) {
      emit(e, ctx);
      tick(5000);
    }
    expect(sent.map((s) => s.message)).toEqual([
      'старт: remux [chrome]',
      'переход: remux -> hls (таймаут старта) [chrome]',
      'зависание: raw -> hls [chrome]',
      'сбой: network, режим hls [chrome]',
    ]);
    expect(sent.every((s) => s.kind === 'player' && !/http|\/api\//.test(s.message))).toBe(true);
  });

  it('зависание без перехода', () => {
    const { sent, emit } = setup();
    emit({ type: 'stall', mode: 'hls' }, ctx);
    expect(sent[0].message).toBe('зависание: hls [chrome]');
  });

  it('два зависания подряд за секунду - отправлено одно', () => {
    const { sent, emit, tick } = setup();
    emit({ type: 'stall', mode: 'raw', to: 'hls' }, ctx);
    tick(1000);
    emit({ type: 'stall', mode: 'raw', to: 'hls' }, ctx);
    expect(sent).toHaveLength(1);
    tick(2000);
    emit({ type: 'stall', mode: 'raw', to: 'hls' }, ctx);
    expect(sent).toHaveLength(2);
  });

  it('сбой отправляется всегда', () => {
    const { sent, emit } = setup();
    emit({ type: 'fatal', mode: 'hls', code: 'network' }, ctx);
    emit({ type: 'fatal', mode: 'hls', code: 'network' }, ctx);
    expect(sent).toHaveLength(2);
  });

  it('коды по-русски: decode и неизвестный', () => {
    const { sent, emit, tick } = setup();
    emit({ type: 'fallback', mode: 'raw', to: 'remux', code: 'decode' }, ctx);
    tick(5000);
    emit({ type: 'fallback', mode: 'raw', to: 'remux', code: 'weird' }, ctx);
    expect(sent[0].message).toContain('(ошибка воспроизведения)');
    expect(sent[1].message).toContain('(weird)');
  });
});
