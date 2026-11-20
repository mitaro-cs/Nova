import { expect, test } from './util-test';
import { expectClean, url } from './util';

// «Что смотреть» (афиша) и «Управление → Мастер»: проверки без интернета и без зависимости от случайных данных.

test('афиша: запрос словами не ломает страницу, пустой ответ объяснён', async ({ page }) => {
  await page.route('**/api/discover', (route) => route.fulfill({ json: { rows: [] } }));
  await page.goto(url('/feed'));
  const ask = page.getByRole('textbox', { name: 'Что хотите посмотреть' });
  await expect(ask).toBeVisible();
  await ask.fill('qzxwv');
  await expect(page.getByText('ничего не нашли')).toBeVisible();
  await ask.fill('короткое');
  await expect(ask).toHaveValue('короткое'); // найдётся или нет - зависит от демо-данных, страница должна остаться целой
  await expectClean(page, 'афиша с запросом');
});

test('мастер: шаги переключаются нажатием и цифрой, замер не зацикливается', async ({ page }) => {
  await page.goto(url('/admin?tab=setup'));
  const steps = page.getByRole('list', { name: 'Шаги настройки' });
  await expect(steps.getByRole('button')).toHaveCount(5);
  await steps.getByRole('button', { name: /Доступ/ }).click();
  await expect(page.getByRole('heading', { name: /Кто и откуда/ })).toBeVisible();
  await page.keyboard.press('5');
  await expect(page.getByRole('heading', { name: 'Всё готово' })).toBeVisible();
  await expect(page.getByLabel('Проверки')).toContainText('Скорость до этого устройства');
  await expectClean(page, 'мастер настройки, итог');
});
