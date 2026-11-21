import { expect, test } from './util-test';
import { env, url } from './util';

// Медленный интернет у зрителя: плеер замеряет скорость до хоста и сразу начинает с
// автокачества, а не с «Оригинала», который будет останавливаться.
test('медленный интернет: старт с автокачества', async ({ page, context }, info) => {
  test.skip(info.project.name === 'iphone-webkit', 'ограничение сети - только в Chromium (CDP)');
  const title = await (await page.request.get(url(`/api/titles/${env().ids['Начало']}`))).json();
  await page.goto(url('/'));
  // ~0,3 Мбит/с: файл в демо-данных требует больше - «Оригинал» тут застревал бы.
  const cdp = await context.newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 40, downloadThroughput: 38_000, uploadThroughput: 200_000 });
  // Минималистичный плеер не пишет подсказку о скорости: проверяем сам выбор - замер сохранён и поток пошёл
  // через адаптивный HLS (`/hls/`), а не «Оригинал».
  const asked: string[] = [];
  page.on('request', (r) => asked.push(new URL(r.url()).pathname));
  await page.goto(url(`/watch/${title.files[0].id}`));
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('nova-speed') ?? 'null')?.mbit ?? null), { timeout: 15_000 }).toBeLessThan(1);
  await expect.poll(() => asked.some((p) => p.includes('/hls/')), { timeout: 15_000 }).toBe(true);
  expect(asked.some((p) => /\/copy\//.test(p))).toBe(false);
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });

  // Отчёт плеера хосту (скорость, качество, остановки) проверяет server/tests/speed.rs: здешний
  // Chromium не играет H.264, до сохранения места дело не доходит.
});
