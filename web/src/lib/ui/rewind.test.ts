import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Контракт общего слоя Rewind: токены и классы, на которые опираются экраны и страница фильма.
const css = readFileSync(fileURLToPath(new URL('./rewind.css', import.meta.url)), 'utf8');
const film = readFileSync(fileURLToPath(new URL('../title/rewind-sheet.css', import.meta.url)), 'utf8');

describe('rewind.css', () => {
  it('токены плиты', () => {
    for (const token of ['--sheet:', '--sheet-2:', '--sheet-3:', '--sheet-line:', '--sheet-text-2:', '--sheet-text-3:']) {
      expect(css, token).toContain(token);
    }
  });

  it('классы страницы и плиты', () => {
    for (const cls of ['.rw-page', '.rw-head', '.rw-plate', '.rw-block', '.rw-h2']) {
      expect(css, cls).toContain(cls);
    }
  });

  it('внутри плиты переопределены палитра и линия', () => {
    const plate = css.slice(css.indexOf('.rw-plate {'));
    expect(plate).toContain('--surface: var(--sheet-2)');
    expect(plate).toContain('--line: var(--sheet-line)');
  });

  it('страница фильма берёт цвета плиты из токенов, а не своих значений', () => {
    expect(film).toContain('var(--sheet)');
    expect(film).not.toMatch(/background:\s*#212121/);
  });
});
