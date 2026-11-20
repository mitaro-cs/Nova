import { expect, test } from './util-test';
import { expectClean, url } from './util';

// Живая схема цепочки в «Обзоре»: шаги и статусы приходят от сервера, интерфейс только показывает.
const chain = {
  ok: false,
  verdict: 'Сайт не отдал поток',
  host: 'plplayer.online',
  steps: [
    { key: 'parse', title: 'Ссылка', note: 'Узнаём сайт и вид видео по адресу', status: 'ok', ms: 0, detail: 'Вид: site, сайт: plplayer.online, сезон 2, серия 5, озвучка 89' },
    { key: 'public', title: 'Публичный адрес', note: 'Пускаем только внешние сайты', status: 'ok', ms: 1, detail: 'Публичный адрес: plplayer.online' },
    { key: 'master', title: 'Поток сайта', note: 'Спрашиваем у сайта настоящий адрес видео', status: 'fail', ms: 1740, detail: 'Плеер недоступен из этой сети (регион)' },
    { key: 'playlist', title: 'Плейлист', note: 'Читаем плейлист', status: 'skip', ms: 0, detail: 'Не проверялось: выше была ошибка' },
    { key: 'segment', title: 'Кусок видео', note: 'Берём первый кусок', status: 'skip', ms: 0, detail: 'Не проверялось: выше была ошибка' },
  ],
};

test('схема цепочки: поиск, шаги и ручная ссылка с сезоном', async ({ page }) => {
  await page.route(/\/api\/discover(?:\?|$)/, (r) => r.fulfill({ json: { rows: [], updatedAt: null } }));
  await page.route('**/api/friends/activity', (r) => r.fulfill({ json: [] }));
  await page.route('**/api/links/search?*', (r) =>
    r.fulfill({ json: [{ url: 'https://plplayer.online/s/636?season=2&episode=5#nova-season=2&episode=5&nova-voice=89', site: 'plplayer', label: 'Дороро' }] }),
  );
  let sent: string | null = null;
  await page.route('**/api/stream/diagnose', (r) => {
    sent = (r.request().postDataJSON() as { url: string }).url;
    return r.fulfill({ json: chain });
  });

  await page.goto(url('/discover'));
  const block = page.getByRole('region', { name: 'Как работает видео' });
  await expect(block).toBeVisible();

  await block.getByRole('searchbox', { name: 'Найти видео для проверки' }).fill('дороро');
  await block.getByRole('button', { name: 'Найти', exact: true }).click();
  await expect(block.getByRole('button', { name: /Дороро/ })).toBeVisible();
  await block.getByRole('button', { name: /Дороро/ }).click();

  await expect(block).toContainText('Сайт не отдал поток');
  await expect(block.locator('ol.steps li')).toHaveCount(5);
  await expect(block.locator('ol.steps li.fail')).toContainText('Плеер недоступен из этой сети');
  await expectClean(page, 'схема цепочки');

  await block.getByRole('tab', { name: 'По ссылке' }).click();
  await block.getByLabel('Ссылка на видео').fill('https://plplayer.online/s/636?season=2&episode=5');
  await block.getByLabel('Сезон').fill('2');
  await block.getByLabel('Серия').fill('5');
  await block.getByLabel('Озвучка (id)').fill('89');
  await block.getByRole('button', { name: 'Проверить' }).click();
  await expect.poll(() => sent ?? '').toContain('#nova-season=2&episode=5&nova-voice=89');
});
