import { expect, test } from './util-test';
import { env, expectClean, url } from './util';

// Сначала комната, потом видео (владелец, 21.11): «Создать комнату» сразу открывает пустую комнату,
// первое видео создателя включается у всех.
test('пустая комната: создаётся сразу, первое видео включает плеер', async ({ page }) => {
  await page.goto(url(`/people/${env().people.me}`));
  await page.getByRole('navigation', { name: 'Быстрые разделы' }).getByRole('button', { name: /Создать комнату/ }).click();
  await expect(page).toHaveURL(/\/room\/[a-z0-9]+$/);
  await expect(page.getByRole('heading', { name: 'Комната готова', level: 1 })).toBeVisible();
  await expectClean(page, 'пустая комната');
  await page.route('**/room-empty.mp4', (r) => r.abort());
  await page.getByLabel('Добавить видео').fill('https://example.com/room-empty.mp4');
  await page.getByRole('button', { name: 'Включить' }).click();
  await expect(page.getByRole('application', { name: 'Видеоплеер' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Комната готова' })).toHaveCount(0);
});
