/** Familiar encoding tiers; selection still uses the real dimensions from the manifest. */
export function qualityTier(height: number, width?: number) {
  const nominal = width && width / height > 1.9 ? width * 9 / 16 : height;
  return [144,240,360,480,720,1080,1440,2160,4320].reduce((best,v) =>
    Math.abs(v-nominal)<Math.abs(best-nominal)?v:best,144);
}

/** Одна человеческая подпись качества везде в плеере: 1080p, 720p, 4K. */
export function qualityName(height: number, width?: number | null) {
  const tier = qualityTier(height, width ?? undefined);
  return tier === 2160 || tier === 4320 ? '4K' : `${tier}p`;
}

type LabelLevel = { i: number; height: number; label?: number | string | null; display?: string | null };

/**
 * Подпись варианта источника. Все варианты остаются доступными; одинаковые разрешения
 * различаются порядковым номером, а не «вариант N» и не техническим мусором из плейлиста.
 */
export function levelLabel(level: LabelLevel, all: LabelLevel[]) {
  if (!level.height) return 'Оригинал';
  const base = qualityName(level.height);
  const same = all.filter((l) => qualityName(l.height) === base);
  if (same.length < 2) return base;
  const order = same.findIndex((l) => l.i === level.i);
  return order === 0 ? base : `${base} (${order + 1})`;
}

/**
 * Одно качество - одна строка: у сайтов в плейлисте бывает по два-три варианта одной высоты (разные битрейты
 * и кодеки), и меню показывало «720p», «720p (2)», «720p (3)». Оставляем на каждую ступень вариант с большим
 * битрейтом (при равном - первый), по убыванию высоты.
 */
export function uniqueLevels<T extends { height: number; width?: number; bitrate?: number }>(list: T[]): T[] {
  const best = new Map<string, T>();
  for (const l of list) {
    if (!l.height) continue;
    const key = qualityName(l.height, l.width);
    const was = best.get(key);
    if (!was || (l.bitrate ?? 0) > (was.bitrate ?? 0)) best.set(key, l);
  }
  return [...best.values()].sort((a, b) => b.height - a.height);
}
