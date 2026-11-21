import { count } from '$lib/fmt';
import type { ProfileStats } from '$lib/types';
import type { IconName } from '$lib/ui/icons';

// Уровень зрителя и значки - одни правила для карточки на телефоне и «Пульта» на компьютере.

export interface Goal {
  icon: IconName;
  label: string;
  need: number;
  of: (s: ProfileStats) => number;
  word: [string, string, string];
  hint: string;
}

export const GOALS: Goal[] = [
  { icon: 'play', label: 'Новичок', need: 1, of: (s) => s.watched ?? 0, word: ['просмотр', 'просмотра', 'просмотров'], hint: 'первый просмотр' },
  { icon: 'sparkle', label: 'Киноман', need: 50, of: (s) => s.hours, word: ['час', 'часа', 'часов'], hint: '50 часов просмотра' },
  { icon: 'tv', label: 'Марафонец', need: 3, of: (s) => s.series, word: ['сериал', 'сериала', 'сериалов'], hint: '3 сериала' },
  { icon: 'edit', label: 'Критик', need: 5, of: (s) => s.reviews, word: ['отзыв', 'отзыва', 'отзывов'], hint: '5 отзывов' },
  { icon: 'users', label: 'Душа компании', need: 5, of: (s) => s.friends, word: ['друг', 'друга', 'друзей'], hint: '5 друзей' },
  { icon: 'globe', label: 'Исследователь', need: 10, of: (s) => s.outside, word: ['отметка', 'отметки', 'отметок'], hint: '10 отметок в Обзоре' },
];

export const earnedOf = (s: ProfileStats | null) => (s ? GOALS.filter((g) => g.of(s) >= g.need) : []);

/** Ближайший неполученный значок: «до «Киноман» - 8 часов». */
export function nextGoalOf(s: ProfileStats | null): string | null {
  if (!s) return null;
  const open = GOALS.filter((g) => g.of(s) < g.need).sort((a, b) => b.of(s) / b.need - a.of(s) / a.need)[0];
  if (!open) return null;
  const left = open.need - open.of(s);
  const n = left % 1 ? Math.round(left * 10) / 10 : left;
  return `До «${open.label}» - ${n % 1 ? `${String(n).replace('.', ',')} ${open.word[1]}` : count(n, ...open.word)}`;
}

/** Опыт за часы, фильмы, серии, отзывы и друзей; уровень растёт по квадратному корню. */
export function levelOf(s: ProfileStats | null) {
  const xp = s ? s.hours * 10 + s.movies * 6 + s.episodes * 2 + s.reviews * 15 + s.friends * 10 + s.outside * 2 : 0;
  const level = Math.floor(Math.sqrt(xp / 25)) + 1;
  const from = 25 * (level - 1) ** 2;
  const to = 25 * level ** 2;
  return { level, toNext: Math.max(0, Math.min(1, (xp - from) / Math.max(1, to - from))) };
}

/** Звание по уровню: растёт вместе с опытом. */
export function rankOf(level: number) {
  return level >= 12 ? 'Легенда зала' : level >= 8 ? 'Кинокритик' : level >= 5 ? 'Киноман' : level >= 3 ? 'Кинолюбитель' : 'Кинозритель';
}
