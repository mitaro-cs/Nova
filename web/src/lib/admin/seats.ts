// Места беты креслами: по порядку регистрации (по id), хост места не занимает. Больше 100 мест - ряды по 20,
// показываем не больше 300 кресел.
export type SeatState = 'now' | 'taken' | 'idle' | 'off' | 'free';
export interface Seat {
  state: SeatState;
  label: string;
}

const IDLE = 30 * 86_400_000;
const MAX = 300;

export function seatMap(
  users: { id: number; name: string; role: string; disabled: boolean; seenAt: number | null }[],
  seats: { used: number; total: number } | null,
  watching: { userId?: number }[],
  now = Date.now(),
) {
  const viewers = users.filter((u) => u.role !== 'host').sort((a, b) => a.id - b.id);
  const total = seats?.total || Math.max(10, Math.ceil(viewers.length / 10) * 10);
  const shown = Math.min(Math.max(total, viewers.length), MAX);
  const cols = shown > 100 ? 20 : 10;
  const live = new Set(watching.map((w) => w.userId).filter((x): x is number => x != null));
  const rows: Seat[][] = [];
  for (let i = 0; i < shown; i++) {
    const r = Math.floor(i / cols), c = i % cols;
    const u = viewers[i];
    let state: SeatState = 'free';
    if (u) state = live.has(u.id) ? 'now' : u.disabled ? 'off' : !u.seenAt || now - u.seenAt > IDLE ? 'idle' : 'taken';
    const who = !u ? 'свободно' : `${u.name}${state === 'now' ? ' · смотрит' : state === 'idle' ? ' · давно не заходил' : state === 'off' ? ' · доступ закрыт' : ''}`;
    (rows[r] ??= []).push({ state, label: `${r + 1} ряд, ${c + 1} место · ${who}` });
  }
  return { rows, cols, total, shown, hidden: Math.max(0, total - shown), used: viewers.length, now: viewers.filter((u) => live.has(u.id)).length };
}
