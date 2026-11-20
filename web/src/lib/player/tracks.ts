// Дорожки: субтитры (загрузка, разбор, вид) и варианты оформления. Звуковая дорожка - это запрос
// потока с другим `audio` (цепочка `plan.ts`), отдельного кода не требует.
import { parseVtt, type Cue } from './vtt';

/**
 * Озвучку подписываем так, как она названа в источнике. Если две дорожки называются одинаково,
 * добавляем « 2», « 3» - иначе в выборе нельзя понять, чем они отличаются.
 */
export function distinctNames(items: { index: number; label: string }[]) {
  const used = new Map<string, number>();
  return items.map(({ index, label }) => {
    const base = label.trim();
    const seen = (used.get(base) ?? 0) + 1;
    used.set(base, seen);
    return { index, label: seen === 1 ? base : `${base} ${seen}` };
  });
}

/** Replace position-only names with a clear language label; keep genuine distributor names unchanged. */
export function sourceTrackName(name: string | undefined, lang: string | undefined, index: number) {
  const raw = (name || '').trim();
  const code = (lang || raw).trim().toLowerCase();
  const generic = !raw || /^(?:рус|русс|rus|ru|eng|en|ukr|uk|ua|jpn|ja|audio|track|дорожка)(?:\s*[-_ ]?\s*\d+)?$/iu.test(raw);
  if (!generic) return raw;
  const language = ({ ru: 'Русская дорожка', rus: 'Русская дорожка', en: 'Английская дорожка', eng: 'Английская дорожка', uk: 'Украинская дорожка', ukr: 'Украинская дорожка', ua: 'Украинская дорожка', ja: 'Японская дорожка', jpn: 'Японская дорожка', de: 'Немецкая дорожка', deu: 'Немецкая дорожка', fr: 'Французская дорожка', fra: 'Французская дорожка', es: 'Испанская дорожка', spa: 'Испанская дорожка' } as Record<string, string>)[code] ?? ({ 'рус': 'Русская дорожка', 'русский': 'Русская дорожка', 'английский': 'Английская дорожка', 'украинский': 'Украинская дорожка', 'японский': 'Японская дорожка' } as Record<string, string>)[raw.toLowerCase()] ?? `Аудиодорожка ${index + 1}`;
  return language;
}

/** Вид субтитров - у каждого зрителя свой, помнится на устройстве. */
export interface SubLook {
  size: 's' | 'm' | 'l';
  look: 'box' | 'shadow' | 'yellow' | 'cinema';
  high: boolean;
}

export const SUB_SIZES = [
  { id: 's', label: 'Мелкие' },
  { id: 'm', label: 'Средние' },
  { id: 'l', label: 'Крупные' },
] as const;

export const SUB_LOOKS = [
  { id: 'box', label: 'На плашке' },
  { id: 'shadow', label: 'С тенью' },
  { id: 'cinema', label: 'Обводка' },
  { id: 'yellow', label: 'Жёлтые' },
] as const;

const KEY = 'fsfy-subs-look';

export function savedLook(): SubLook {
  const base: SubLook = { size: 'm', look: 'box', high: false };
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? 'null');
    return saved && typeof saved === 'object' ? { ...base, ...saved } : base;
  } catch {
    return base;
  }
}

export function saveLook(look: SubLook) {
  try {
    localStorage.setItem(KEY, JSON.stringify(look));
  } catch {
    /* приватный режим */
  }
}

/** Субтитры дорожки: `[]` - сервер не отдал, `null` - не вышло достучаться (дорожку не выбираем). */
export async function loadSubtitles(fileId: number, id: string, fetcher: typeof fetch = fetch): Promise<Cue[] | null> {
  try {
    const r = await fetcher(`/api/files/${fileId}/subs/${id}`, { credentials: 'same-origin' });
    return r.ok ? parseVtt(await r.text()) : [];
  } catch {
    return null;
  }
}
