import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createReporter } from './diag';

describe('diag', () => {
  const sent: { kind: string; message: string; where: string }[] = [];
  beforeEach(() => { sent.length = 0; });
  const send = (b: { kind: string; message: string; where: string }) => { sent.push(b); };

  it('одинаковую ошибку шлёт один раз', () => {
    const r = createReporter(send, () => '/watch/1');
    r('error', 'сбой A');
    r('error', 'сбой A');
    expect(sent).toHaveLength(1);
    expect(sent[0]).toEqual({ kind: 'error', message: 'сбой A', where: '/watch/1' });
  });

  it('не больше 5 разных за страницу', () => {
    const r = createReporter(send, () => '/');
    for (let i = 0; i < 12; i++) r('error', `сбой ${i}`);
    expect(sent).toHaveLength(5);
  });

  it('режет длинный текст до 500 знаков', () => {
    const r = createReporter(send, () => '/');
    r('rejection', 'я'.repeat(2000));
    expect(sent[0].message.length).toBe(500);
  });

  it('ошибка отправки не бросает', () => {
    const r = createReporter(() => { throw new Error('нет сети'); }, () => '/');
    expect(() => r('error', 'x')).not.toThrow();
    vi.restoreAllMocks();
  });
});
