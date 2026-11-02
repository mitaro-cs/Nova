// Цвет фильма: `accent` постера тёмный и приглушённый, для кнопки и текста он слабый. Из его оттенка получаем
// заливку (белый текст читается) и светлый тон для текста на тёмном. Без зависимостей: это идёт в главную
// (бюджет JS) и на телевизор (Chrome 69, поэтому готовый hex, а не color-mix).
export type FilmTones = { fill: string; on: string; text: string };

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

function hex(h: number, s: number, l: number): string {
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    return Math.round((l - a * Math.max(-1, Math.min(k - 3, 9 - k, 1))) * 255)
      .toString(16)
      .padStart(2, '0');
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

function whiteContrast(color: string): number {
  const [r, g, b] = [1, 3, 5]
    .map((i) => parseInt(color.slice(i, i + 2), 16) / 255)
    .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 1.05 / (0.2126 * r + 0.7152 * g + 0.0722 * b + 0.05);
}

/** `null`: цвета нет, он не разбирается или постер чёрно-белый - тогда интерфейс остаётся белым. */
export function filmTones(accent: string | null | undefined): FilmTones | null {
  if (!accent || !/^#[0-9a-f]{6}$/i.test(accent)) return null;
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(accent.slice(i, i + 2), 16) / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  const light = (max + min) / 2;
  const sat = d === 0 ? 0 : d / (1 - Math.abs(2 * light - 1));
  if (sat < 0.1) return null;
  const sector = max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  const hue = sector * 60;
  const s = clamp(sat + 0.25, 0.55, 0.75);
  let l = 0.42;
  while (l > 0.1 && whiteContrast(hex(hue, s, l)) < 4.5) l -= 0.02;
  return { fill: hex(hue, s, l), on: '#ffffff', text: hex(hue, clamp(sat + 0.3, 0.55, 0.85), 0.74) };
}

let owner = 0;

function clear() {
  const style = document.documentElement.style;
  for (const name of ['--film', '--on-film', '--film-text']) style.removeProperty(name);
}

/** Поставить цвет экрана. Возвращает отмену: она снимает цвет, только если новый экран не успел поставить свой. */
export function setFilm(accent: string | null | undefined): () => void {
  const mine = ++owner;
  const tones = filmTones(accent);
  if (tones) {
    const style = document.documentElement.style;
    style.setProperty('--film', tones.fill);
    style.setProperty('--on-film', tones.on);
    style.setProperty('--film-text', tones.text);
  } else clear();
  return () => {
    if (mine === owner) clear();
  };
}
