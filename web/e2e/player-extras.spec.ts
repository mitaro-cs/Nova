import type { Page } from '@playwright/test';
import { expect, test } from './util-test';
import { env, url, playerButton } from './util';

// Плеер, раунд 23.11 (выбор владельца): одно меню вкладками, ночной звук, ссылка на момент (`?t=`), окно «Отметить
// момент», экран конца фильма с оценкой. Видео демо короткое (6 с).

async function fileOf(page: Page, name: string): Promise<number> {
  await page.goto(url('/'));
  const d = await page.evaluate(async (id) => (await fetch('/api/titles/' + id)).json(), env().ids[name]);
  return d.files[0].id;
}
const state = (page: Page) =>
  page.evaluate(() => {
    const v = document.querySelector('video');
    return { t: v?.currentTime ?? 0, paused: v?.paused ?? true, ended: v?.ended ?? false, ready: v?.readyState ?? 0 };
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


test('плеер: одно меню вкладками и ночной звук', async ({ page }) => {
  const id = await fileOf(page, 'Ла-Ла Ленд');
  await page.goto(url(`/watch/${id}`));
  await expect.poll(async () => (await state(page)).ready).toBeGreaterThanOrEqual(3);
  await playerButton(page, 'Качество');
  const tabs = page.getByRole('tablist', { name: 'Настройки звука и картинки' });
  await expect(tabs.getByRole('tab', { name: 'Качество' })).toHaveAttribute('aria-selected', 'true');
  await tabs.getByRole('tab', { name: 'Скорость' }).click();
  await expect(tabs.getByRole('tab', { name: 'Скорость' })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('menuitemradio', { name: 'Обычная' })).toBeVisible();
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Настройки плеера' }).click();
  const night = page.getByRole('menuitemcheckbox', { name: /Ночной звук/ });
  await expect(night).toHaveAttribute('aria-checked', 'false');
  await night.click();
  // Меню остаётся открытым: переключатель сразу показывает новое состояние.
  await expect(night).toHaveAttribute('aria-checked', 'true');
  expect(await page.evaluate(() => localStorage.getItem('nova-night-audio'))).toBe('1');
});

test('плеер: ссылка на момент открывает видео с этой секунды, окно «Отметить момент»', async ({ page }) => {
  const id = await fileOf(page, 'Ла-Ла Ленд');
  await page.goto(url(`/watch/${id}?t=3`));
  await expect.poll(async () => (await state(page)).t).toBeGreaterThanOrEqual(2.9);
  await playerButton(page, 'Отметить момент');
  const sheet = page.getByRole('dialog', { name: 'Отметить момент' });
  await expect(sheet).toContainText('Момент 0:0');
  await expect(sheet.getByRole('textbox', { name: 'Подпись к моменту' })).toBeVisible();
  // Пробел в подписи - буква, а не пауза плеера.
  await sheet.getByRole('textbox', { name: 'Подпись к моменту' }).fill('вот эта сцена');
  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);
  expect(page.url()).toContain(`/watch/${id}`);
});

test('плеер: в конце фильма - оценка и «что дальше»', async ({ page }) => {
  const id = await fileOf(page, 'Ла-Ла Ленд');
  await page.goto(url(`/watch/${id}?t=4`));
  await expect.poll(async () => (await state(page)).ready).toBeGreaterThanOrEqual(3);
  if ((await state(page)).paused) await page.getByRole('button', { name: 'Смотреть', exact: true }).first().click();
  const end = page.getByRole('dialog', { name: 'Фильм закончился' });
  await expect(end).toBeVisible({ timeout: 15000 });
  await expect(end.getByRole('heading', { name: /Как вам/ })).toBeVisible();
  await expect(end.getByRole('button', { name: 'Смотреть снова' })).toBeVisible();
});

test('плеер: в титрах - «Завершить» ведёт к концу фильма, досмотренное сразу «Просмотрено»', async ({ page }) => {
  const id = await fileOf(page, 'Ла-Ла Ленд');
  // Титры с 2-й секунды (как нашла бы фоновая задача); сервер отвечает, что фильм только что стал «Просмотрено».
  await page.route(`**/api/files/${id}/info`, async (r) => {
    const res = await r.fetch();
    const j = await res.json();
    await r.fulfill({ response: res, json: { ...j, markers: { intro: null, credits: 2, manual: false } } });
  });
  await page.route(`**/api/progress/${id}`, (r) =>
    r.request().method() === 'PUT' ? r.fulfill({ json: { ok: true, done: true, watched: true } }) : r.fallback(),
  );
  await page.goto(url(`/watch/${id}?t=2.2`));
  const card = page.getByRole('group', { name: 'Начались титры' });
  const skip = card.getByRole('button', { name: 'Завершить' });
  await expect(skip).toBeVisible();
  await expect(page.getByRole('status').filter({ hasText: 'Отмечено: просмотрено' })).toBeVisible();
  await skip.click();
  await expect(page.getByRole('dialog', { name: 'Фильм закончился' })).toBeVisible();
  await expect(skip).toHaveCount(0);
});
