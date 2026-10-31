import { expect, test } from './util-test';
import { url } from './util';

// Пустой результат поиска видео объясняет, что искали и кто из источников отвечает, и даёт повторить.
test('нет источника: видно, что искали, кто не ответил, и можно повторить', async ({ page }) => {
  const id = 'tt7654330';
  const card = { id, kind: 'movie', title: 'Редкий фильм', original: 'Rare Film', year: 2024, rating: 7, poster: null, genres: [], local: null, mark: null, wished: false };
  const detail = { card, overview: 'Описание.', runtime: 100, votes: 10, backdrop: null, posterLarge: null, tagline: null, age: null, releaseDate: null, countries: [], languages: [], aka: [], people: [], gallery: [], similar: [], localCard: null, lists: [], seasons: [], friends: [], links: [] };
  await page.route(`**/api/discover/title/${id}**`, (route) => {
    const path = new URL(route.request().url()).pathname;
    return route.fulfill({ json: path.endsWith('/reviews') ? { average: null, votes: 0, total: 0, items: [], canWrite: false } : path.endsWith('/trailer') ? { linkId: null } : detail });
  });
  await page.route('**/api/franchise?*', (route) => route.fulfill({ json: [] }));
  await page.route('**/api/sources/status', (route) =>
    route.fulfill({
      json: [
        { site: 'lordfilm', label: 'LordFilm', state: 'down' },
        { site: 'rutube', label: 'Rutube', state: 'ok' },
      ],
    }),
  );
  let searches = 0;
  await page.route('**/api/links/search?*', (route) => {
    if (new URL(route.request().url()).searchParams.get('mode') === 'trailer') return route.fulfill({ json: [] });
    searches++;
    return route.fulfill({ json: [] });
  });
  await page.goto(url(`/ext/${id}`));
  const block = page.getByRole('region', { name: 'Другие источники' });
  await expect(block).toBeVisible();
  await expect(block.getByText('Искали: «Редкий фильм», «Rare Film» (2024).')).toBeVisible();
  await expect(block.getByText('LordFilm - не отвечает, проверим позже')).toBeVisible();
  await expect(block.getByText('Rutube - отвечает')).toBeVisible();
  const before = searches;
  await block.getByRole('button', { name: 'Повторить' }).click();
  await expect.poll(() => searches).toBeGreaterThan(before);
});
