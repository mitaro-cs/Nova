// Страница человека кино внутри Nova (фото и фильмография с IMDb): актёр - его роли, режиссёр и
// создатель - что снял. Id IMDb («nm…») - если он есть в описании (без поиска по имени).
// Ключ в ссылке - оригинальное имя (`orig`): по нему ищет сервер и хранятся «нравится»; русское
// имя (`name`, если оно переведено) - отдельным параметром `ru` для заголовка.
export function personHref(p: { name: string; orig?: string | null; job?: string; id?: unknown }): string {
  const q = new URLSearchParams({ name: p.orig ?? p.name, role: p.job === 'actor' ? 'actor' : 'director' });
  if (p.orig) q.set('ru', p.name);
  if (typeof p.id === 'string' && p.id.startsWith('nm')) q.set('id', p.id);
  return `/person?${q}`;
}
