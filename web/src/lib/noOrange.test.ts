import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Единый стиль: акцент интерфейса - белый. Оранжевый из образца Кинопоиска и цвет фильма (`--film*`, `setFilm`)
// владелец отклонил (2 ноября) - обратно не возвращаются.
const root = fileURLToPath(new URL('..', import.meta.url));

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return files(path);
    return /\.(svelte|css|ts)$/.test(name) && !name.endsWith('noOrange.test.ts') ? [path] : [];
  });
}

describe('акцент только белый', () => {
  it('нет --m-accent, #ff5500, #f7931e, --film, setFilm', () => {
    const found = files(root).filter((f) => /--m-accent|#ff5500|#f7931e|--film|--on-film|setFilm/i.test(readFileSync(f, 'utf8')));
    expect(found).toEqual([]);
  });
});
