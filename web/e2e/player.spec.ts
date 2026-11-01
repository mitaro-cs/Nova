import type { Page } from '@playwright/test';
import { expect, test } from './util-test';
import { env, url } from './util';

// Плеер на настоящем видео демо-данных: запуск, управление, переход по цепочке режимов при сбое,
// понятная ошибка, качество, серии и «Следующая серия». Видео короткое (6 с).

async function seriesFiles(page: Page): Promise<number[]> {
  await page.goto(url('/'));
  const title = env().ids['Тьма'];
  const d = await page.evaluate(async (id) => (await fetch('/api/titles/' + id)).json(), title);
  return d.files.map((f: { id: number }) => f.id);
}

const state = (page: Page) =>
  page.evaluate(() => {
    const v = document.querySelector('video');
    return { t: v?.currentTime ?? 0, paused: v?.paused ?? true, ended: v?.ended ?? false };
  });

// Playwright WebKit на Windows собран без медиа-движка: даже корректный WAV даёт MEDIA_ERR_SRC_NOT_SUPPORTED
// (а canPlayType отвечает «probably»). Играть там нечему - тесты плеера пропускаем, а не считаем сбоем.
test.beforeEach(async ({ page, browserName }) => {
  if (browserName !== 'webkit') return;
  await page.goto('about:blank');
  const works = await page.evaluate(
    (src) =>
      new Promise<boolean>((done) => {
        const a = document.createElement('audio');
        a.addEventListener('loadedmetadata', () => done(true));
        a.addEventListener('error', () => done(false));
        setTimeout(() => done(false), 3000);
        a.src = src;
      }),
    'data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEAESsAABErAAABAAgAZGF0YQAAAAA=',
  );
  test.skip(!works, 'WebKit без медиа-движка (Playwright на Windows): воспроизвести нечего');
});

// Видео ещё может не появиться на странице: это 0, а не ошибка.
const ready = (page: Page) => page.evaluate(() => document.querySelector('video')?.readyState ?? 0);

// Запустить, если ещё не идёт (разрешён ли автозапуск без нажатия, зависит от политики браузера).
async function play(page: Page) {
  if ((await state(page)).paused) await page.getByRole('button', { name: 'Смотреть', exact: true }).first().click();
}

test('плеер: запускается по нажатию, пауза и перемотка с клавиатуры', async ({ page }) => {
  const [first] = await seriesFiles(page);
  await page.goto(url(`/watch/${first}`));
  // Видео готово. Браузер мог не дать запустить само (тогда ждёт нажатия) - режим от этого не меняется.
  await expect.poll(() => ready(page)).toBeGreaterThanOrEqual(3);
  await play(page);
  await expect.poll(async () => (await state(page)).t).toBeGreaterThan(0.5);
  await page.keyboard.press('KeyK');
  await expect.poll(async () => (await state(page)).paused).toBe(true);
  await page.keyboard.press('KeyK');
  await page.keyboard.press('Digit5');
  await expect.poll(async () => (await state(page)).t).toBeGreaterThan(2.5);
  // Меню закрывается по Esc, не уводя со страницы.
  await page.getByRole('button', { name: 'Настройки плеера' }).click();
  await expect(page.getByText('Настройки просмотра')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.locator('.panel')).toHaveCount(0);
  expect(page.url()).toContain(`/watch/${first}`);
});

test('плеер: прямой файл не загрузился - сам переходит на адаптивный поток и играет', async ({ page }) => {
  const [first] = await seriesFiles(page);
  await page.route('**/api/files/*/raw', (route) => route.abort());
  const hls: string[] = [];
  page.on('request', (r) => r.url().includes('/hls/') && hls.push(r.url()));
  await page.goto(url(`/watch/${first}`));
  await expect.poll(() => hls.length, { timeout: 20_000 }).toBeGreaterThan(0);
  await expect.poll(() => ready(page), { timeout: 20_000 }).toBeGreaterThanOrEqual(3);
  await play(page);
  await expect.poll(async () => (await state(page)).t, { timeout: 15_000 }).toBeGreaterThan(0.5);
  await expect(page.locator('.err')).toHaveCount(0);
});

test('плеер: ничего не играет - понятная ошибка с кнопками, «Повторить» запускает заново', async ({ page }) => {
  test.setTimeout(120_000);
  const [first] = await seriesFiles(page);
  await page.route('**/api/files/*/raw', (route) => route.abort());
  await page.route('**/api/files/*/hls/**', (route) => route.fulfill({ status: 404, body: 'нет' }));
  await page.goto(url(`/watch/${first}`));
  const err = page.locator('.err');
  await expect(err).toBeVisible({ timeout: 90_000 });
  await expect(err).toContainText('Видео не');
  await expect(err.getByRole('button', { name: 'Назад' })).toBeVisible();
  await expect(err.getByRole('button', { name: 'Повторить' })).toBeVisible();
  await expect(err.getByRole('link', { name: 'Скачать' })).toBeVisible();
  // Сеть вернулась - «Повторить» запускает с первого варианта.
  await page.unroute('**/api/files/*/raw');
  await page.unroute('**/api/files/*/hls/**');
  await err.getByRole('button', { name: 'Повторить' }).click();
  await expect(err).toHaveCount(0);
  await expect.poll(() => ready(page), { timeout: 15_000 }).toBeGreaterThanOrEqual(3);
});

test('плеер: смена качества не теряет место, список серий открывает другую серию', async ({ page }) => {
  const [first, second] = await seriesFiles(page);
  await page.goto(url(`/watch/${first}`));
  await expect.poll(() => ready(page)).toBeGreaterThanOrEqual(3);
  await play(page);
  await expect.poll(async () => (await state(page)).t).toBeGreaterThan(1);
  const before = (await state(page)).t;
  await page.getByRole('button', { name: 'Качество' }).click();
  await page.getByRole('menuitemradio', { name: /Авто/ }).click();
  await expect(page.getByRole('button', { name: 'Качество' })).toContainText('Авто');
  await expect.poll(async () => (await state(page)).t, { timeout: 15_000 }).toBeGreaterThanOrEqual(before);
  await page.getByRole('button', { name: 'Сезоны и серии' }).click();
  await expect(page.locator('.episode-option')).toHaveCount(2);
  await page.locator('.episode-option').nth(1).click();
  await expect(page).toHaveURL(new RegExp(`/watch/${second}$`));
});

test('плеер: в конце серии - карточка «Следующая серия» с отсчётом и переход', async ({ page }) => {
  test.setTimeout(60_000);
  const [first, second] = await seriesFiles(page);
  await page.goto(url(`/watch/${first}`));
  await expect.poll(() => ready(page)).toBeGreaterThanOrEqual(3);
  await play(page);
  await page.keyboard.press('Digit9');
  await expect(page.getByText('Следующая серия')).toBeVisible({ timeout: 15_000 });
  await expect(page).toHaveURL(new RegExp(`/watch/${second}$`), { timeout: 20_000 });
});
