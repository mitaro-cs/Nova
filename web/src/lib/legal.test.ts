import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test } from 'vitest';
import { LEGAL_VERSION } from './legal';

// Редакция документов на сайте и на сервере - одно число: иначе регистрация не примет согласие или окно согласия зациклится.
test('LEGAL_VERSION сайта совпадает с сервером', () => {
  const rs = readFileSync(resolve(__dirname, '../../../server/src/api/account.rs'), 'utf8');
  const m = /pub const LEGAL_VERSION: i64 = (\d+);/.exec(rs);
  expect(m, 'в account.rs нет LEGAL_VERSION').toBeTruthy();
  expect(Number(m![1])).toBe(LEGAL_VERSION);
});
