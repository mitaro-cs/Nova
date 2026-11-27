import { href } from './nav';
import type { Health, Status } from './types';

// «Требует внимания» на Сводке: всё, что ждёт решения хоста, одним списком, важное сверху. У каждого пункта - кнопка
// туда, где это решается (или само действие: `act`).
export type Level = 'bad' | 'warn' | 'info';
export interface Attention {
  id: string;
  level: Level;
  title: string;
  text: string;
  label: string;
  href?: string;
  act?: 'backup';
}

const DAY = 86_400_000;
const ORDER: Record<Level, number> = { bad: 0, warn: 1, info: 2 };

function plural(n: number, one: string, few: string, many: string) {
  const a = n % 10, b = n % 100;
  return a === 1 && b !== 11 ? one : a >= 2 && a <= 4 && (b < 12 || b > 14) ? few : many;
}

export function attention(status: Status, health: Health | null, wishes: number, now = Date.now()): Attention[] {
  const out: Attention[] = [];
  for (const s of health?.sources ?? []) {
    if (s.state === 'down') out.push({ id: 'src-' + s.site, level: 'bad', title: `${s.label} выключен ненадолго`, text: 'Три ошибки подряд - видео ищется в других источниках, через 10 минут попробуем снова', label: 'Источники', href: href('sources', 'video') });
  }
  if (!status.ffmpeg || !status.ffprobe) out.push({ id: 'ffmpeg', level: 'bad', title: 'Нет ffmpeg', text: 'Без него не играют MKV и AVI и не делаются кадры перемотки', label: 'Указать путь', href: href('system', 'server') });
  const sys = status.system;
  if (sys?.diskFree != null && sys.diskFree < 10 * 1024 ** 3) out.push({ id: 'disk', level: 'bad', title: 'Мало места на диске', text: 'Меньше 10 ГБ - кэш видео и копии базы могут не поместиться', label: 'Показатели', href: href('summary', 'stats') });
  if (status.scan.error) out.push({ id: 'scan', level: 'bad', title: 'Папки не прочитались', text: status.scan.error, label: 'Папки', href: href('library', 'folders') });
  const seats = status.seats;
  if (seats && seats.left <= 0) out.push({ id: 'seats', level: 'warn', title: `Все ${seats.total} мест беты заняты`, text: 'Регистрация закрыта сама. Можно добавить мест или освободить, удалив аккаунт', label: 'Регистрация', href: href('viewers', 'signup') });
  else if (seats && seats.left <= Math.max(5, Math.round(seats.total * 0.1))) out.push({ id: 'seats', level: 'warn', title: `Осталось ${seats.left} ${plural(seats.left, 'место', 'места', 'мест')} беты`, text: `Занято ${seats.used} из ${seats.total}. Когда займут все, регистрация закроется сама`, label: 'Регистрация', href: href('viewers', 'signup') });
  if (wishes > 0) out.push({ id: 'wishes', level: 'warn', title: `${wishes} ${plural(wishes, 'просьба ждёт', 'просьбы ждут', 'просьб ждут')} ответа`, text: 'Зрители просят добавить фильмы', label: 'Открыть', href: href('library', 'wishes') });
  if (status.lastBackup === null || (status.lastBackup !== undefined && now - status.lastBackup > 7 * DAY)) {
    const days = status.lastBackup ? Math.floor((now - status.lastBackup) / DAY) : null;
    out.push({ id: 'backup', level: 'warn', title: days === null ? 'Копий базы ещё нет' : `Последняя копия базы - ${days} ${plural(days, 'день', 'дня', 'дней')} назад`, text: 'В базе зрители, отметки, отзывы и подборки', label: 'Сделать копию', act: 'backup' });
  }
  if (status.unmatched > 0) out.push({ id: 'unmatched', level: 'info', title: `${status.unmatched} ${plural(status.unmatched, 'фильм', 'фильма', 'фильмов')} без описания`, text: 'Описание не нашлось по имени файла - можно указать фильм вручную', label: 'Показать', href: href('library', 'catalog') + '&filter=none' });
  if (status.update?.state === 'available' && status.update.latest) out.push({ id: 'update', level: 'info', title: `Вышла Nova ${status.update.latest}`, text: 'Перед установкой копия базы сделается сама', label: 'Обновить', href: href('system', 'update') });
  if (!status.publicUrl) out.push({ id: 'net', level: 'info', title: 'Nova открыта только дома', text: 'Друзья не из вашей сети не зайдут, пока не включен доступ из интернета', label: 'Включить', href: href('system', 'net') });
  const recent = (health?.errors ?? []).filter((e) => e.level === 'ERROR' && now - e.at < DAY).length;
  if (recent) out.push({ id: 'errors', level: 'info', title: `${recent} ${plural(recent, 'ошибка', 'ошибки', 'ошибок')} за сутки`, text: 'Недавние ошибки - в показателях', label: 'Показать', href: href('summary', 'stats') });
  return out.sort((a, b) => ORDER[a.level] - ORDER[b.level]);
}
