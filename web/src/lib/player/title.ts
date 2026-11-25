/** В плеере показываем название, а не подпись источника с годом, качеством и оригиналом. */
export function playbackTitle(value:string):string {
  return value
    .split(/\s*[|｜]\s*/)[0]
    .replace(/\s*\((?:19|20)\d{2}(?:\s*[,·].*?)?\)\s*$/u,'')
    .replace(/\s+(?:смотреть онлайн|смотреть|онлайн)\s*$/iu,'')
    .replace(/\s+\d{3,4}p\b.*$/iu,'')
    .trim();
}

/** Номер сезона/серии из подписи, которую отдаёт сервер («1 сезон, 2 серия · Название»). */
function numbers(text:string):{season:number|null;episode:number|null} {
  const both = text.match(/(\d+)\s*сезон(?:а|ов)?\s*[,·]?\s*(\d+)\s*серия/u);
  if (both) return {season:Number(both[1]),episode:Number(both[2])};
  const one = text.match(/(\d+)\s*серия/u);
  return {season:null,episode:one?Number(one[1]):null};
}

/**
 * Заголовок вверху плеера: «Название · 1 сезон, 2 серия». Собирается из названия фильма и
 * номера серии один раз при открытии источника: смена качества, звука и перемотка его не трогают.
 */
export function playbackHeading(title:string, episode:string, season?:number|null, episodeNo?:number|null):string {
  const name = playbackTitle(title);
  const found = numbers(episode ?? '');
  const s = season ?? found.season;
  const e = episodeNo ?? found.episode;
  // В названии уже есть «4 сезон, 7 серия» (у видео по ссылке так собирает сервер) - второй раз не пишем.
  const has = (n: number, w: string) => new RegExp(`(^|\\D)${n}\\s*${w}`, 'u').test(name);
  if (s != null && e != null) return has(s, 'сезон') && has(e, 'сери') ? name : `${name} · ${s} сезон, ${e} серия`;
  if (e != null && has(e, 'сери')) return name;
  if (e != null) return `${name} · ${e} серия`;
  return name;
}
