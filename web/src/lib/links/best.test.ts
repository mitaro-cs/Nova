import { describe, it, expect } from 'vitest';
import { pickBest, rankFound } from './best';
import type { FoundVideo } from '$lib/types';

const v = (over: Partial<FoundVideo>): FoundVideo => ({
  site: 'rutube',
  url: 'https://rutube.ru/video/x/',
  label: 'Первый человек (2018)',
  thumb: null,
  thumbUrl: null,
  duration: null,
  author: null,
  ...over,
});

describe('rankFound', () => {
  it('ближе к длительности фильма - выше', () => {
    const ranked = rankFound(
      [v({ url: 'a', duration: 150, label: 'Первый человек - ролик' }), v({ url: 'b', duration: 8300 })],
      8460,
    );
    expect(ranked.map((x) => x.url)).toEqual(['b', 'a']);
  });

  it('при равной длительности выше качество', () => {
    const ranked = rankFound(
      [v({ url: 'hd', duration: 8400, label: 'Первый человек (1080p)' }), v({ url: '4k', duration: 8400, label: 'Первый человек (4K)' })],
      8460,
    );
    expect(ranked[0].url).toBe('4k');
  });

  it('LordFilm без длительности выше безымянных ссылок других источников', () => {
    const ranked = rankFound([v({ url: 'r' }), v({ url: 'l', site: 'lordfilm' })], 8460);
    expect(ranked[0].url).toBe('l');
  });

  it('не меняет исходный список', () => {
    const input = [v({ url: 'a' }), v({ url: 'b', site: 'lordfilm' })];
    rankFound(input, 100);
    expect(input.map((x) => x.url)).toEqual(['a', 'b']);
  });
});

describe('pickBest', () => {
  it('пусто - null', () => {
    expect(pickBest([], 8460)).toBeNull();
  });

  it('берёт лучшую', () => {
    expect(pickBest([v({ url: 'a', duration: 100 }), v({ url: 'b', duration: 8400 })], 8460)?.url).toBe('b');
  });
});
