import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createProgress, elsewherePosition } from './progress';

describe('createProgress', () => {
  const calls: { url: string; init: RequestInit }[] = [];
  beforeEach(() => {
    vi.useFakeTimers();
    calls.length = 0;
    vi.stubGlobal('fetch', async (url: string, init: RequestInit) => {
      calls.push({ url, init });
      return new Response('{}');
    });
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  const make = (over: Partial<Parameters<typeof createProgress>[0]> = {}) => {
    let time = 100;
    const p = createProgress({ url: () => '/api/progress/7', position: () => time, duration: () => 5000, net: () => ({ mbit: 20 }), ...over });
    return { p, setTime: (t: number) => (time = t) };
  };

  it('без force не чаще раза в 5 секунд', async () => {
    const { p } = make();
    await p.save();
    await p.save();
    expect(calls).toHaveLength(1);
    vi.advanceTimersByTime(5001);
    await p.save();
    expect(calls).toHaveLength(2);
  });

  it('force отправляет всегда', async () => {
    const { p } = make();
    await p.save();
    await p.save(true);
    expect(calls).toHaveLength(2);
  });

  it('тело, метод, keepalive и заголовок защиты', async () => {
    const { p } = make();
    await p.save(true);
    const { url, init } = calls[0];
    expect(url).toBe('/api/progress/7');
    expect(init.method).toBe('PUT');
    expect(init.keepalive).toBe(true);
    expect((init.headers as Record<string, string>)['x-fsfy']).toBe('1');
    expect(JSON.parse(init.body as string)).toEqual({ position: 100, duration: 5000, net: { mbit: 20 } });
  });

  it('не отправляет, пока нет длительности или место меньше секунды, или нет адреса', async () => {
    const a = make({ duration: () => 0 });
    await a.p.save(true);
    const b = make();
    b.setTime(0.5);
    await b.p.save(true);
    const c = make({ url: () => null });
    await c.p.save(true);
    expect(calls).toHaveLength(0);
  });

  it('stop отправляет последнее место', async () => {
    const { p } = make();
    await p.save();
    await p.stop();
    expect(calls).toHaveLength(2);
  });

  it('сбой сети не бросает', async () => {
    vi.stubGlobal('fetch', async () => {
      throw new Error('offline');
    });
    const { p } = make();
    await expect(p.save(true)).resolves.not.toThrow();
  });
});

describe('elsewherePosition', () => {
  it('предлагает место, если оно дальше чем на 20 секунд и не у самого конца', () => {
    expect(elsewherePosition(500, 100, 5000)).toBe(500);
    expect(elsewherePosition(110, 100, 5000)).toBeNull();
    expect(elsewherePosition(4980, 100, 5000)).toBeNull();
    expect(elsewherePosition(null, 100, 5000)).toBeNull();
  });
});
