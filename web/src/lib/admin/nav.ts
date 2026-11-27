import type { IconName } from '$lib/ui/icons';

// «Управление» A1 «Зал» (выбор владельца 27.11): пять разделов вместо одиннадцати пунктов, у каждого - вкладки.
// Адрес: `?tab=<раздел>&sub=<вкладка>`. Прежние адреса (`?tab=folders`, `?tab=settings`…) ведут на новые места,
// `?tab=setup` - мастер первой настройки (в панели его нет, вход - со Сводки).
export interface Sub {
  id: string;
  label: string;
}
export interface Section {
  id: string;
  label: string;
  hint: string;
  icon: IconName;
  subs: Sub[];
}

export const SECTIONS: Section[] = [
  { id: 'summary', label: 'Сводка', hint: 'Эфир и что требует внимания', icon: 'pulse', subs: [{ id: 'now', label: 'Сейчас' }, { id: 'stats', label: 'Показатели' }, { id: 'map', label: 'Карта дома' }] },
  { id: 'viewers', label: 'Зрители', hint: 'Места беты, приглашения, права', icon: 'users', subs: [{ id: 'people', label: 'Все' }, { id: 'signup', label: 'Регистрация' }, { id: 'logins', label: 'Входы' }] },
  { id: 'library', label: 'Библиотека', hint: 'Фильмы, папки, просьбы, витрина', icon: 'film', subs: [{ id: 'catalog', label: 'Фильмы' }, { id: 'folders', label: 'Папки' }, { id: 'wishes', label: 'Просьбы' }, { id: 'showcase', label: 'Витрина' }] },
  { id: 'sources', label: 'Источники', hint: 'Откуда видео и описания', icon: 'globe', subs: [{ id: 'video', label: 'Видео' }, { id: 'meta', label: 'Описания' }] },
  { id: 'system', label: 'Система', hint: 'Сервер, интернет, обновления', icon: 'sliders', subs: [{ id: 'server', label: 'Сервер' }, { id: 'net', label: 'Интернет' }, { id: 'security', label: 'Безопасность' }, { id: 'update', label: 'Обновления' }] },
];

const LEGACY: Record<string, [string, string]> = {
  pult: ['summary', 'now'],
  overview: ['summary', 'stats'],
  map: ['summary', 'map'],
  people: ['viewers', 'people'],
  catalog: ['library', 'catalog'],
  folders: ['library', 'folders'],
  wishes: ['library', 'wishes'],
  access: ['system', 'net'],
  settings: ['system', 'server'],
  version: ['system', 'update'],
};

/** Раздел и вкладка по адресу; `setup` - мастер. */
export function resolve(tab: string | null, sub: string | null): { section: string; sub: string } {
  if (tab === 'setup') return { section: 'setup', sub: '' };
  const legacy = tab ? LEGACY[tab] : undefined;
  if (legacy) return { section: legacy[0], sub: sub && SECTIONS.find((s) => s.id === legacy[0])?.subs.some((x) => x.id === sub) ? sub : legacy[1] };
  const s = SECTIONS.find((x) => x.id === tab) ?? SECTIONS[0];
  return { section: s.id, sub: s.subs.find((x) => x.id === sub)?.id ?? s.subs[0].id };
}

export const href = (section: string, sub?: string) => `?tab=${section}${sub ? `&sub=${sub}` : ''}`;
