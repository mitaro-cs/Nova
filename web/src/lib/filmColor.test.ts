import { beforeEach, describe, expect, it, vi } from 'vitest';
import { filmTones, setFilm } from './filmColor';

function hslHex(h: number, s: number, l: number): string {
  const sat = s / 100;
  const lit = l / 100;
  const a = sat * Math.min(lit, 1 - lit);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    const v = lit - a * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    return Math.round(v * 255).toString(16).padStart(2, '0');
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

function luminance(hex: string): number {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const [r, g, b] = c.map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

describe('filmTones', () => {
  it('заливка читается с белым текстом на всех оттенках', () => {
    for (let h = 0; h < 360; h++) {
      for (const s of [20, 38, 70]) {
        const t = filmTones(hslHex(h, s, 30))!;
        expect(t.on).toBe('#ffffff');
        expect(contrast(t.fill, '#ffffff')).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('текстовый оттенок читается на панели #141414', () => {
    for (let h = 0; h < 360; h++) {
      expect(contrast(filmTones(hslHex(h, 30, 30))!.text, '#141414')).toBeGreaterThanOrEqual(4.5);
    }
  });

  it('нет цвета, мусор и серый дают null', () => {
    for (const v of [null, undefined, '', 'red', '#12', hslHex(0, 5, 30)]) expect(filmTones(v)).toBeNull();
  });
});

describe('setFilm', () => {
  const props = new Map<string, string>();
  beforeEach(() => {
    props.clear();
    vi.stubGlobal('document', {
      documentElement: {
        style: {
          setProperty: (k: string, v: string) => props.set(k, v),
          removeProperty: (k: string) => props.delete(k),
        },
      },
    });
  });

  it('ставит токены, отмена возвращает белые', () => {
    const off = setFilm('#5a2a2a');
    expect(props.get('--film')).toMatch(/^#/);
    expect(props.get('--film-text')).toMatch(/^#/);
    off();
    expect(props.size).toBe(0);
  });

  it('отмена старого экрана не снимает цвет нового', () => {
    const old = setFilm('#5a2a2a');
    setFilm('#2a5a2a');
    const now = props.get('--film');
    old();
    expect(props.get('--film')).toBe(now);
  });

  it('цвет без оттенка снимает прежний', () => {
    setFilm('#5a2a2a');
    setFilm(null);
    expect(props.size).toBe(0);
  });
});
