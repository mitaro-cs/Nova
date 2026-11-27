import { describe, expect, it } from 'vitest';
import { seatMap } from './seats';

const NOW = 1_800_000_000_000;
const u = (id: number, seen: number | null, role = 'viewer', disabled = false) => ({ id, name: 'u' + id, role, disabled, seenAt: seen });

describe('места беты креслами', () => {
  it('по порядку регистрации, хост не занимает места', () => {
    const m = seatMap([u(3, NOW), u(1, NOW, 'host'), u(2, NOW - 40 * 86_400_000)], { used: 2, total: 100 }, [{ userId: 3 }], NOW);
    expect(m.rows).toHaveLength(10);
    expect(m.cols).toBe(10);
    expect(m.rows[0].slice(0, 3).map((s) => s.state)).toEqual(['idle', 'now', 'free']);
    expect(m.rows[0][1].label).toBe('1 ряд, 2 место · u3 · смотрит');
    expect(m.used).toBe(2);
    expect(m.now).toBe(1);
  });
  it('без лимита - зал по числу зрителей, больше 100 - ряды по 20, не больше 300', () => {
    expect(seatMap([u(2, NOW)], null, [], NOW).shown).toBe(10);
    const big = seatMap([], { used: 0, total: 500 }, [], NOW);
    expect(big.cols).toBe(20);
    expect(big.shown).toBe(300);
    expect(big.hidden).toBe(200);
  });
});
