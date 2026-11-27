import { describe, expect, it } from 'vitest';
import { attention } from './attention';
import type { Health, Status } from './types';

const NOW = Date.UTC(2026, 10, 27, 18);
function status(over: Partial<Status> = {}): Status {
  return {
    version: '2.1.11-beta', update: null, titles: 10, files: 10, size: 1, metaPending: 0, unmatched: 0, unprobed: 0,
    scan: { running: false, at: null, error: null, progress: null, last: null }, meta: { running: false, error: null, done: 0 },
    ffmpeg: true, ffprobe: true, watching: [], canPick: true, publicUrl: 'https://nova.example', inbox: null,
    import: { running: false, filesTotal: 0, filesDone: 0, bytesTotal: 0, bytesDone: 0, current: null, error: null },
    seats: { used: 10, total: 100, left: 90 }, lastBackup: NOW - 86_400_000, ...over,
  };
}

describe('требует внимания', () => {
  it('всё в порядке - пусто', () => {
    expect(attention(status(), null, 0, NOW)).toEqual([]);
  });
  it('важное сверху, у копии базы - действие', () => {
    const h: Health = { version: '', uptimeSec: 0, dbBytes: 0, errors: [], sources: [{ site: 'rutube', label: 'Rutube', state: 'down', fails: 3, ok_ms: null }] };
    const list = attention(status({ unmatched: 7, lastBackup: NOW - 6 * 86_400_000 - 1 }), h, 2, NOW);
    expect(list.map((x) => x.id)).toEqual(['src-rutube', 'wishes', 'unmatched']);
    const old = attention(status({ lastBackup: NOW - 9 * 86_400_000 }), null, 0, NOW);
    expect(old[0]).toMatchObject({ id: 'backup', act: 'backup', title: 'Последняя копия базы - 9 дней назад' });
    expect(attention(status({ lastBackup: null }), null, 0, NOW)[0].title).toBe('Копий базы ещё нет');
  });
  it('места беты: мало и закончились', () => {
    expect(attention(status({ seats: { used: 96, total: 100, left: 4 } }), null, 0, NOW)[0].title).toBe('Осталось 4 места беты');
    expect(attention(status({ seats: { used: 100, total: 100, left: 0 } }), null, 0, NOW)[0].title).toBe('Все 100 мест беты заняты');
  });
  it('без доступа из интернета - подсказка', () => {
    expect(attention(status({ publicUrl: null }), null, 0, NOW).map((x) => x.id)).toEqual(['net']);
  });
});
