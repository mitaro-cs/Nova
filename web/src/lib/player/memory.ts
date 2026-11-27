// Память плеера (выбор владельца, 23.11; качество - 27.11): у сериала запоминаются озвучка, субтитры, качество и
// «пропускать заставку» - в следующей серии всё как вы выбрали, и у своих файлов, и у видео по ссылке. Ночной звук - общий для всех фильмов. Хранится в браузере (localStorage), без
// сервера: это настройки этого устройства. Хранилище недоступно (частный режим) - просто ничего не помним.

export interface Pick {
  label: string;
  lang: string | null;
}
export interface SeriesPrefs {
  audio?: Pick;
  /** null - субтитры выключены нарочно. */
  subs?: Pick | null;
  skipIntro?: boolean;
  /** Своё видео: выбранное качество ('auto', 'orig' или высота). Без него - по скорости сети. */
  quality?: 'auto' | 'orig' | number;
  /** Видео по ссылке: высота выбранного уровня, -1 - автоматически. */
  height?: number;
}

/** Своё видео - номер сериала, видео по ссылке - строка сериала из ленты («l:tt…»). */
export type PrefsId = number | string;
const key = (id: PrefsId) => `nova-series:${id}`;

export function loadPrefs(titleId: PrefsId | null | undefined): SeriesPrefs {
  if (!titleId) return {};
  try {
    const v = JSON.parse(localStorage.getItem(key(titleId)) ?? '{}');
    return v && typeof v === 'object' ? (v as SeriesPrefs) : {};
  } catch {
    return {};
  }
}

export function savePrefs(titleId: PrefsId | null | undefined, patch: Partial<SeriesPrefs>) {
  if (!titleId) return;
  try {
    localStorage.setItem(key(titleId), JSON.stringify({ ...loadPrefs(titleId), ...patch }));
  } catch {
    /* хранилище недоступно */
  }
}

/** Та же дорожка в другой серии: сначала по названию («Кубик в кубе»), потом по языку. */
export function matchTrack<T extends { label: string; lang: string | null }>(list: T[], want: Pick | null | undefined): T | undefined {
  if (!want) return undefined;
  return list.find((t) => t.label === want.label) ?? (want.lang ? list.find((t) => t.lang === want.lang) : undefined);
}

const NIGHT = 'nova-night-audio';
export function nightSaved(): boolean {
  try {
    return localStorage.getItem(NIGHT) === '1';
  } catch {
    return false;
  }
}
export function saveNight(on: boolean) {
  try {
    if (on) localStorage.setItem(NIGHT, '1');
    else localStorage.removeItem(NIGHT);
  } catch {
    /* хранилище недоступно */
  }
}
