import type { FoundVideo } from '$lib/types';

/** Качество по подписи: 4K > 1080 > 720/HD > остальное. */
const quality = (label: string) => (/2160|4k|4к/i.test(label) ? 4 : /1080|full ?hd/i.test(label) ? 3 : /720|hd/i.test(label) ? 2 : 0);

/**
 * Находки по убыванию пригодности: длительность ближе к длительности фильма - лучше (у LordFilm её
 * нет - считается средней), выше качество в подписи, LordFilm - первым среди равных. Названия здесь
 * не проверяются: совпадение названия, года и серии проверяет сервер (`video_search::matches`).
 */
export function rankFound(found: FoundVideo[], runtimeSec: number): FoundVideo[] {
  const catalogOrder = new Map<string | null, number>();
  for (const item of found) if (item.site === 'own' && !catalogOrder.has(item.author)) catalogOrder.set(item.author, catalogOrder.size);
  const score = (f: FoundVideo) => {
    const span = runtimeSec && f.duration ? Math.min(1, Math.abs(f.duration - runtimeSec) / runtimeSec) * 10 : 3;
    return span - quality(f.label) - (f.site === 'lordfilm' ? 0.5 : 0);
  };
  return [...found].sort((a, b) => {
    if (a.site === 'own' && b.site === 'own') {
      const priority = (catalogOrder.get(a.author) ?? 0) - (catalogOrder.get(b.author) ?? 0);
      if (priority) return priority;
    }
    return score(a) - score(b);
  });
}

export function pickBest(found: FoundVideo[], runtimeSec: number): FoundVideo | null {
  return rankFound(found, runtimeSec)[0] ?? null;
}
