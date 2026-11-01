import { describe, it, expect } from 'vitest';
import { remotePosition, needsSeek } from './sync';

describe('remotePosition', () => {
  it('идёт - добавляем прошедшее время, стоит - как есть', () => {
    expect(remotePosition({ playing: true, position: 100 }, 1000, 4000)).toBe(103);
    expect(remotePosition({ playing: false, position: 100 }, 1000, 4000)).toBe(100);
  });
});

describe('needsSeek', () => {
  it('небольшой разрыв не дёргает, большой - перематывает', () => {
    expect(needsSeek(100, 101, 'raw', false)).toBe(false);
    expect(needsSeek(100, 104, 'raw', false)).toBe(true);
  });

  it('у перепаковки порог 6 с (перемотка - новый поток)', () => {
    expect(needsSeek(100, 104, 'remux', false)).toBe(false);
    expect(needsSeek(100, 107, 'remux', false)).toBe(true);
  });

  it('пока видео уже перематывается - не трогаем', () => {
    expect(needsSeek(100, 200, 'raw', true)).toBe(false);
  });
});
