import { expect, test } from './util-test';
import { expectClean, url } from './util';

// «Управление» A1 «Зал» (27.11): пять разделов, Сводка - экран и места беты креслами, «Требует внимания», Ctrl K.
test('зал: разделы, экран, кресла, внимание', async ({ page }) => {
  await page.goto(url('/admin'));
  const rail = page.getByRole('navigation', { name: 'Разделы управления' });
  for (const name of ['Сводка', 'Зрители', 'Библиотека', 'Источники', 'Система']) await expect(rail.getByRole('link', { name: new RegExp(name) })).toBeVisible();
  await expect(rail.getByRole('link', { name: /Сводка/ })).toHaveAttribute('aria-current', 'page');
  const hall = page.getByRole('region', { name: 'Зал' });
  await expect(hall.getByText(/В эфире|Сейчас тихо/)).toBeVisible();
  await expect(hall.getByRole('img', { name: /Места беты: занято \d+ из 100/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: /Требует внимания/ })).toBeVisible();
  await expectClean(page, 'зал');

  // Вкладки раздела и прежние адреса.
  await rail.getByRole('link', { name: /Зрители/ }).click();
  await expect(page).toHaveURL(/tab=viewers/);
  await page.getByRole('navigation', { name: /Вкладки раздела/ }).getByRole('link', { name: 'Регистрация' }).click();
  await expect(page.getByLabel('Мест в бете')).toHaveValue('100');
  await page.goto(url('/admin?tab=folders'));
  await expect(rail.getByRole('link', { name: /Библиотека/ })).toHaveAttribute('aria-current', 'page');

  // Поиск по управлению.
  await page.keyboard.press('Control+k');
  const dlg = page.getByRole('dialog', { name: 'Найти в управлении' });
  await expect(dlg).toBeVisible();
  await dlg.getByLabel('Что найти').fill('ночной');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/tab=system&sub=server/);
  await expect(page.getByRole('heading', { name: 'Обслуживание' })).toBeVisible();
});
