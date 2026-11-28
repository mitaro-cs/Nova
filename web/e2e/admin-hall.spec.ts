import { expect, test } from './util-test';
import { expectClean, url } from './util';

// «Управление» как сайт Nova (28.11): разделы текстом с чертой, Сводка - карусель эфира, дела карточками, бета
// лицами, неделя цифрами; Зрители - карточками с окном человека; Ctrl K.
test('сводка: разделы, эфир, дела, бета', async ({ page }) => {
  await page.goto(url('/admin'));
  const secs = page.getByRole('navigation', { name: 'Разделы управления' });
  for (const name of ['Сводка', 'Зрители', 'Библиотека', 'Источники', 'Система']) await expect(secs.getByRole('link', { name: new RegExp(name) })).toBeVisible();
  await expect(secs.getByRole('link', { name: /Сводка/ })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('region', { name: 'Эфир' })).toBeVisible();
  await expect(page.getByRole('heading', { name: /Требует внимания/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: /Бета/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Неделя в Nova' })).toBeVisible();
  await expectClean(page, 'сводка');

  // Зрители - карточки, окно человека.
  await secs.getByRole('link', { name: /Зрители/ }).click();
  await expect(page).toHaveURL(/tab=viewers/);
  const cards = page.getByRole('list', { name: 'Зрители' });
  await expect(cards.getByRole('button').first()).toBeVisible();
  await expectClean(page, 'зрители');
  await cards.getByRole('button', { name: /Анна/ }).click();
  const dlg = page.getByRole('dialog', { name: /Анна/ });
  await expect(dlg.getByText('Может скачивать')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(dlg).toBeHidden();

  // Вкладки и прежние адреса.
  await page.getByRole('navigation', { name: /Вкладки раздела/ }).getByRole('link', { name: 'Регистрация' }).click();
  await expect(page.getByLabel('Мест в бете')).toHaveValue('100');
  await page.goto(url('/admin?tab=folders'));
  await expect(secs.getByRole('link', { name: /Библиотека/ })).toHaveAttribute('aria-current', 'page');

  // Поиск по управлению.
  await page.keyboard.press('Control+k');
  const pal = page.getByRole('dialog', { name: 'Найти в управлении' });
  await expect(pal).toBeVisible();
  await pal.getByLabel('Что найти').fill('ночной');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/tab=system&sub=server/);
  await expect(page.getByRole('heading', { name: 'Обслуживание' })).toBeVisible();
});
