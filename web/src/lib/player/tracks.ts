// Дорожки: субтитры (загрузка, разбор, вид) и варианты оформления. Звуковая дорожка - это запрос
// потока с другим `audio` (цепочка `plan.ts`), отдельного кода не требует.
import { parseVtt, type Cue } from './vtt';

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
