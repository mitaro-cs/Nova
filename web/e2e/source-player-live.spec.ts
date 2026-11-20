import { test, expect } from '@playwright/test';
import { url } from './util';

test('страница фильма воспроизводит видео внутри Nova', async ({ page }) => {
  test.skip(!process.env.NOVA_LIVE_PLAYER, 'Проверка требует доступного внешнего сайта');
  test.setTimeout(90_000);
  const domain = 'hd1.garri-potter-lordfilms.com';
  const headers = { 'X-Fsfy': '1' };
  const saved = await (await page.request.get(url('/api/admin/catalogs'))).json();
  try {
    const setting = await page.request.put(url('/api/admin/catalogs'), {
      headers, data: { items: [{ domain, kind: 'movie', enabled: true }] },
    });
    expect(setting.ok()).toBeTruthy();
    const added = await page.request.post(url('/api/links'), {
      headers, data: { url: `https://${domain}/125-garri-potter-i-filosofskij-kamen.html`, label: 'Гарри Поттер' },
    });
    expect(added.ok()).toBeTruthy();
    const { id } = await added.json();
    const playlist = page.waitForResponse(response =>
      response.ok() && /mpegurl/i.test(response.headers()['content-type'] ?? ''),
      { timeout: 45000 });
    await page.goto(url('/watch/link/' + id), { waitUntil: 'commit' });
    const inner = page.frameLocator('iframe[title="Просмотр на странице источника"]').frameLocator('iframe[src*="newplayjj"]');
    await expect(inner.getByRole('button', { name: /Дублированный/ }).first()).toBeVisible({ timeout: 45000 });
    await playlist;
    const play = inner.getByRole('button', { name: /Воспроизвести/ }).first();
    if (await play.isVisible()) await play.click();
    await expect.poll(() => inner.locator('video').evaluateAll(videos =>
      videos.some(el => el instanceof HTMLVideoElement && el.videoWidth > 0 && el.currentTime > 1 && !el.paused)
    ), { timeout: 25000 }).toBe(true);
  } finally {
    await page.request.put(url('/api/admin/catalogs'), { headers, data: saved });
  }
});
