import { beforeEach, expect, it, vi } from 'vitest';
import { loadPrefs, matchTrack, nightSaved, saveNight, savePrefs } from './memory';

beforeEach(() => {
  const store = new Map<string, string>();
  vi.stubGlobal('localStorage', { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => store.set(k, v), removeItem: (k: string) => store.delete(k) });
});

it('запоминает выбор для сериала и дополняет, а не затирает', () => {
  savePrefs(7, { audio: { label: 'Кубик в кубе', lang: 'ru' } });
  savePrefs(7, { skipIntro: true });
  expect(loadPrefs(7)).toEqual({ audio: { label: 'Кубик в кубе', lang: 'ru' }, skipIntro: true });
  expect(loadPrefs(8)).toEqual({});
});

it('находит ту же дорожку по названию, иначе по языку', () => {
  const tracks = [{ index: 0, label: 'Original', lang: 'en' }, { index: 1, label: 'LostFilm', lang: 'ru' }];
  expect(matchTrack(tracks, { label: 'LostFilm', lang: 'ru' })?.index).toBe(1);
  expect(matchTrack(tracks, { label: 'Кубик в кубе', lang: 'ru' })?.index).toBe(1);
  expect(matchTrack(tracks, { label: 'Другая', lang: 'de' })).toBeUndefined();
  expect(matchTrack(tracks, null)).toBeUndefined();
});

it('ночной звук - общий переключатель', () => {
  expect(nightSaved()).toBe(false);
  saveNight(true);
  expect(nightSaved()).toBe(true);
  saveNight(false);
  expect(nightSaved()).toBe(false);
});

it('видео по ссылке: качество и озвучка помнятся по сериалу из ленты', () => {
  savePrefs('l:tt0141842', { height: 720, audio: { label: 'Дмитрий Goblin Пучков', lang: 'ru' } });
  savePrefs(7, { quality: 1080 });
  expect(loadPrefs('l:tt0141842')).toMatchObject({ height: 720, audio: { label: 'Дмитрий Goblin Пучков' } });
  expect(loadPrefs(7).quality).toBe(1080);
  expect(loadPrefs(null)).toEqual({});
  savePrefs(null, { height: 360 });
  expect(loadPrefs('l:tt0141842').height).toBe(720);
});
