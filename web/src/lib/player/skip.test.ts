import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { inIntro, inCredits, nearEnd, NextEpisode } from './skip.svelte';

describe('inIntro', () => {
  it('внутри заставки - да, граница -1 с как в прежнем плеере', () => {
    expect(inIntro(10, [5, 60])).toBe(true);
    expect(inIntro(5, [5, 60])).toBe(true);
    expect(inIntro(59.5, [5, 60])).toBe(false);
    expect(inIntro(4.9, [5, 60])).toBe(false);
    expect(inIntro(10, null)).toBe(false);
  });
});

describe('inCredits', () => {
  const base = { credits: 1000, hasNext: true, series: true, skipped: false, failed: false };
  it('у сериала со следующей серией после начала титров', () => {
    expect(inCredits(1000, base)).toBe(true);
    expect(inCredits(999, base)).toBe(false);
  });
  it('не у фильма, без следующей серии, после «пропустить» и при ошибке', () => {
    expect(inCredits(1100, { ...base, series: false })).toBe(false);
    expect(inCredits(1100, { ...base, hasNext: false })).toBe(false);
    expect(inCredits(1100, { ...base, skipped: true })).toBe(false);
    expect(inCredits(1100, { ...base, failed: true })).toBe(false);
    expect(inCredits(1100, { ...base, credits: null })).toBe(false);
  });
});

describe('nearEnd', () => {
  it('за 90 секунд до конца', () => {
    expect(nearEnd(910, 1000)).toBe(true);
    expect(nearEnd(900, 1000)).toBe(false);
    expect(nearEnd(10, 0)).toBe(false);
  });
});

describe('NextEpisode', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('считает до нуля и вызывает переход один раз', () => {
    const done = vi.fn();
    const n = new NextEpisode(done);
    n.start(3);
    expect(n.left).toBe(3);
    vi.advanceTimersByTime(1000);
    expect(n.left).toBe(2);
    vi.advanceTimersByTime(2000);
    expect(done).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(5000);
    expect(done).toHaveBeenCalledTimes(1);
  });

  it('cancel останавливает отсчёт без перехода', () => {
    const done = vi.fn();
    const n = new NextEpisode(done);
    n.start(5);
    vi.advanceTimersByTime(2000);
    n.cancel();
    expect(n.left).toBeNull();
    vi.advanceTimersByTime(10_000);
    expect(done).not.toHaveBeenCalled();
  });

  it('pause останавливает отсчёт, но оставляет число (пауза видео в титрах)', () => {
    const done = vi.fn();
    const n = new NextEpisode(done);
    n.start(10);
    vi.advanceTimersByTime(3000);
    n.pause();
    expect(n.left).toBe(7);
    vi.advanceTimersByTime(20_000);
    expect(n.left).toBe(7);
    expect(done).not.toHaveBeenCalled();
    n.start(n.left ?? 10);
    vi.advanceTimersByTime(7000);
    expect(done).toHaveBeenCalledTimes(1);
  });

  it('повторный start перезапускает отсчёт', () => {
    const done = vi.fn();
    const n = new NextEpisode(done);
    n.start(5);
    vi.advanceTimersByTime(3000);
    n.start(5);
    expect(n.left).toBe(5);
    vi.advanceTimersByTime(4000);
    expect(done).not.toHaveBeenCalled();
  });
});
