import { describe, expect, it } from 'vitest';
import { isLetterboxed } from './letterbox';

/** Картинка w × h: строки, для которых `row(y)` вернул число, - ровные, иначе - шум. */
function image(w: number, h: number, fill: (y: number) => number | null) {
  const d = new Uint8ClampedArray(w * h * 4);
  for (let y = 0; y < h; y++) {
    const flat = fill(y);
    for (let x = 0; x < w; x++) {
      const v = flat ?? ((x * 37 + y * 11) % 2 ? 40 : 200);
      const i = (y * w + x) * 4;
      d[i] = d[i + 1] = d[i + 2] = v;
      d[i + 3] = 255;
    }
  }
  return d;
}

describe('постер на ровных полосах', () => {
  it('широкий кадр посередине 2:3 - с полосами', () => {
    const w = 48;
    const h = 72;
    const d = image(w, h, (y) => (y < 26 || y >= 46 ? 30 : null));
    expect(isLetterboxed(d, w, h)).toBe(true);
  });

  it('обычный постер - нет', () => {
    const w = 48;
    const h = 72;
    expect(isLetterboxed(image(w, h, () => null), w, h)).toBe(false);
  });

  it('тёмный постер с текстом снизу - нет (полоса только сверху)', () => {
    const w = 48;
    const h = 72;
    expect(isLetterboxed(image(w, h, (y) => (y < 40 ? 20 : null)), w, h)).toBe(false);
  });

  it('высокая вставка между ровными полями - нет (кадр не широкий)', () => {
    const w = 48;
    const h = 72;
    expect(isLetterboxed(image(w, h, (y) => (y < 10 || y >= 62 ? 20 : null)), w, h)).toBe(false);
  });
});
