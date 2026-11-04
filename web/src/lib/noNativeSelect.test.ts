import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Выпадающие списки - только свои (`ui/Select.svelte`): системный браузер рисует по-своему, серым и с рамкой,
// и выбивается из дизайна (владелец, 3 ноября).
const root = fileURLToPath(new URL('..', import.meta.url));

function svelte(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return svelte(path);
    return name.endsWith('.svelte') ? [path] : [];
  });
}

describe('выпадающие списки', () => {
  it('нет системного <select>', () => {
    const found = svelte(root).filter((f) => /<select[\s>]/.test(readFileSync(f, 'utf8')));
    expect(found).toEqual([]);
  });
});
