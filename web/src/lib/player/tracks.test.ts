import { describe, it, expect, vi, afterEach } from 'vitest';
import { savedLook, saveLook, loadSubtitles, SUB_SIZES, SUB_LOOKS } from './tracks';

afterEach(() => vi.unstubAllGlobals());

function storage(initial: Record<string, string> = {}) {
  const data = { ...initial };
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => data[k] ?? null,
    setItem: (k: string, v: string) => void (data[k] = v),
  });
  return data;
}

describe('вид субтитров', () => {
  it('по умолчанию средние на плашке', () => {
    storage();
    expect(savedLook()).toEqual({ size: 'm', look: 'box', high: false });
  });

  it('помнится на устройстве', () => {
    const data = storage();
    saveLook({ size: 'l', look: 'yellow', high: true });
    expect(JSON.parse(data['fsfy-subs-look'])).toEqual({ size: 'l', look: 'yellow', high: true });
    expect(savedLook()).toEqual({ size: 'l', look: 'yellow', high: true });
  });

  it('испорченная запись - по умолчанию', () => {
    storage({ 'fsfy-subs-look': '{не json' });
    expect(savedLook().size).toBe('m');
  });

  it('без localStorage (приватный режим) не падает', () => {
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('нет');
      },
      setItem: () => {
        throw new Error('нет');
      },
    });
    expect(savedLook().look).toBe('box');
    expect(() => saveLook({ size: 's', look: 'box', high: false })).not.toThrow();
  });

  it('варианты для меню по-русски', () => {
    expect(SUB_SIZES.map((s) => s.label)).toEqual(['Мелкие', 'Средние', 'Крупные']);
    expect(SUB_LOOKS.map((s) => s.id)).toEqual(['box', 'shadow', 'cinema', 'yellow']);
  });
});

describe('loadSubtitles', () => {
  const vtt = 'WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nПривет\n';

  it('разбирает WebVTT с сервера', async () => {
    const fetcher = vi.fn(async () => new Response(vtt));
    const cues = await loadSubtitles(7, 's1', fetcher as unknown as typeof fetch);
    expect(fetcher).toHaveBeenCalledWith('/api/files/7/subs/s1', { credentials: 'same-origin' });
    expect(cues).toHaveLength(1);
    expect(cues![0].text).toContain('Привет');
  });

  it('ошибка сервера - пусто', async () => {
    const cues = await loadSubtitles(7, 's1', (async () => new Response('', { status: 500 })) as unknown as typeof fetch);
    expect(cues).toEqual([]);
  });

  it('нет сети - null (дорожку не выбираем)', async () => {
    const cues = await loadSubtitles(7, 's1', (async () => {
      throw new Error('offline');
    }) as unknown as typeof fetch);
    expect(cues).toBeNull();
  });
});
