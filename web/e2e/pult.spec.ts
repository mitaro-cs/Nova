import { expect, test } from './util-test';
import { expectClean, url } from './util';

// «Пульт · Прямой эфир» (26.11): первый экран управления - эфир сверху, живые плитки ниже, плитка ведёт в раздел.
test('пульт: эфир, зрители, огоньки и плитки', async ({ page }) => {
  await page.goto(url('/admin'));
  await expect(page.getByRole('heading', { name: 'Пульт' })).toBeVisible();
  const live = page.getByRole('region', { name: 'Прямой эфир' });
  await expect(live).toBeVisible();
  await expect(live.getByText(/Сейчас в эфире|Сейчас тихо/)).toBeVisible();
  await expect(live.getByRole('link', { name: 'Добавить фильмы' })).toBeVisible();
  await expect(page.getByRole('list', { name: 'Состояние' }).getByRole('link', { name: 'Сервер' })).toBeVisible();
  for (const name of ['Диск', 'Неделя', 'Источники видео', 'Свежее в библиотеке']) {
    await expect(page.getByRole('heading', { name, exact: true })).toBeVisible();
  }
  await expectClean(page, 'пульт');
  await page.getByRole('link', { name: /Свежее в библиотеке/ }).click();
  await expect(page).toHaveURL(/tab=catalog/);
});
