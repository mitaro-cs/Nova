import { expect, test } from './util-test';
import { expectClean, url } from './util';

// Главная «Витрина» (23.11): «Топ-10 Nova за неделю» с контурными цифрами - под «Продолжить просмотр».
test('главная: топ-10 недели', async ({ page }) => {
  const lib = await (await page.request.get(url('/api/library'))).json();
  const items = (Array.isArray(lib) ? lib : lib.items).slice(0, 4).map((card: unknown, i: number) => ({ card, viewers: 4 - i, at: Date.now(), label: '' }));
  await page.route('**/api/top/week', (r) => r.fulfill({ json: items }));
  await page.goto(url('/'));
  const top = page.getByRole('region', { name: 'Топ-10 Nova за неделю' });
  await expect(top).toBeVisible();
  await expect(top.getByRole('listitem')).toHaveCount(4);
  await expect(top.getByRole('listitem').first()).toContainText('1');
  await expectClean(page, 'главная с топ-10');
  if (process.env.SHOT) { await top.scrollIntoViewIfNeeded(); await page.waitForTimeout(800); await page.screenshot({ path: process.env.SHOT }); }
});
