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

test('«Спросите Nova»: настроения понимаются и показываются, недолго - про длительность', async ({ page }) => {
  await page.route('**/api/discover', (route) => route.fulfill({ json: { rows: [] } }));
  await page.goto(url('/feed'));
  const moods = page.getByRole('group', { name: 'Настроение' });
  await moods.getByRole('button', { name: 'Посмеяться' }).click();
  await expect(moods.getByRole('button', { name: 'Посмеяться' })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.got')).toContainText('смешное');
  const ask = page.getByRole('textbox', { name: 'Что хотите посмотреть' });
  await ask.fill('пощекотать нервы, но недолго');
  await expect(page.locator('.got')).toContainText('страшное');
  await expect(page.locator('.got')).toContainText('недлинное');
  await expect(page.locator('.got')).not.toContainText('в названии');
  await expect(page.getByRole('region', { name: 'Жанры' }).getByRole('heading', { name: 'Жанры' })).toBeVisible();
  await expectClean(page, 'подбор: настроение');
});

test('возврат на раздел вкладкой - на то же место, а не наверх', async ({ page }, info) => {
  test.skip(info.project.name !== 'desktop', 'на компьютере вкладки разделов в шапке');
  await page.goto(url('/feed'));
  await page.waitForLoadState('networkidle');
  await page.mouse.wheel(0, 900);
  await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(400);
  const was = await page.evaluate(() => scrollY);
  const nav = page.getByRole('navigation', { name: 'Разделы кинотеатра' });
  await nav.getByRole('link', { name: 'Главная' }).click();
  await expect(page).toHaveURL(/\/$/);
  await nav.getByRole('link', { name: 'Подбор' }).click();
  await expect(page).toHaveURL(/\/feed$/);
  await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(was - 60);
});

test('с фильма назад на главную - на то же место (герой дорастает сверху, страница не уезжает)', async ({ page }, info) => {
  test.skip(info.project.name !== 'desktop', 'на касании прокручивается body, проверяем на компьютере');
  await page.goto(url('/'));
  await page.waitForLoadState('networkidle');
  // Прокручиваем так, чтобы на экране оказалась карточка своего фильма ниже героя.
  const href = await page.evaluate(() => {
    const links = [...document.querySelectorAll<HTMLAnchorElement>('main a[href^="/title/"]')];
    const a = links.find((x) => x.getBoundingClientRect().top + scrollY > innerHeight * 0.9) ?? links.at(-1);
    if (!a) return null;
    scrollTo({ top: Math.max(0, a.getBoundingClientRect().top + scrollY - innerHeight / 2), behavior: 'instant' as ScrollBehavior });
    return a.getAttribute('href');
  });
  expect(href).toBeTruthy();
  await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(200);
  const was = await page.evaluate(() => scrollY);
  await page.locator(`main a[href="${href}"]`).filter({ visible: true }).first().click();
  await expect(page).toHaveURL(/\/title\//);
  await page.goBack();
  await expect(page).toHaveURL(/\/$/);
  await page.waitForTimeout(1500);
  expect(Math.abs((await page.evaluate(() => scrollY)) - was)).toBeLessThan(40);
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
