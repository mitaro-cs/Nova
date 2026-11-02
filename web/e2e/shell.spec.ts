import { expect, test } from './util-test';
import { url } from './util';

// Телефон и планшет: «оболочка». Страница не прокручивается сама, прокручивается body на весь экран, поэтому нижняя
// панель разделов прибита к краю экрана и не ездит; масштабировать интерфейс щипком нельзя.

test('масштабирование запрещено в разметке страницы', async ({ page }) => {
  await page.goto(url('/'));
  const meta = await page.locator('meta[name="viewport"]').getAttribute('content');
  expect(meta).toContain('user-scalable=no');
  expect(meta).toContain('maximum-scale=1');
});

test('нижняя панель не двигается при прокрутке', async ({ page }, info) => {
  test.skip(info.project.name === 'desktop', 'оболочка - только для касания (телефон и планшет)');
  await page.goto(url('/my'));
  const bar = page.locator('nav.tabs');
  await expect(bar).toBeVisible();
  const height = page.viewportSize()!.height;
  const bottom = async () => Math.round((await bar.boundingBox())!.y + (await bar.boundingBox())!.height);
  expect(await bottom()).toBe(height);
  // Прокручиваем body до самого низа: окно не едет, панель на месте.
  await page.evaluate(() => document.body.scrollTo(0, document.body.scrollHeight));
  await page.waitForTimeout(300);
  expect(await page.evaluate(() => scrollY)).toBe(0);
  expect(await bottom()).toBe(height);
  const state = await page.evaluate(() => ({ html: getComputedStyle(document.documentElement).overflow, body: getComputedStyle(document.body).position }));
  expect(state).toEqual({ html: 'hidden', body: 'fixed' });
});

test('щипок не масштабирует: жесты Safari и многопальцевое касание отменяются', async ({ page }, info) => {
  test.skip(info.project.name === 'desktop', 'только для касания');
  await page.goto(url('/'));
  // Запрет ставится, когда страница запустилась (после загрузки файла, не сразу): ждём, а не проверяем один раз.
  await expect
    .poll(() =>
      page.evaluate(() => {
        const gesture = new Event('gesturestart', { cancelable: true });
        document.dispatchEvent(gesture);
        return gesture.defaultPrevented;
      }),
    )
    .toBe(true);
});

test('новая страница открывается сверху, а по «Назад» возвращает место', async ({ page }, info) => {
  test.skip(info.project.name === 'desktop', 'оболочка - только для касания');
  await page.goto(url('/'));
  // Главная достраивается (ряды приходят позже): прокручиваем, пока страница не станет достаточно длинной.
  await expect
    .poll(async () => {
      await page.evaluate(() => document.body.scrollTo(0, 500));
      return page.evaluate(() => document.body.scrollTop);
    })
    .toBeGreaterThan(100);
  await page.locator('nav.tabs').getByRole('link', { name: 'Коллекция' }).click();
  await expect(page).toHaveURL(/\/my$/);
  await expect.poll(() => page.evaluate(() => document.body.scrollTop)).toBe(0);
  await page.goBack();
  await expect(page).toHaveURL(/\/$/);
  await expect.poll(() => page.evaluate(() => document.body.scrollTop), { timeout: 4000 }).toBeGreaterThan(100);
});
