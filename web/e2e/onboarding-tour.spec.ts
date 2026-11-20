import { expect, test } from './util-test';
import { url } from './util';

// Знакомство: любой фильм можно найти поиском. Экскурсия: сама открывает разделы и подсвечивает нужное место.

const film = { id: 'tt7654321', kind: 'movie', title: 'Тестовый фильм', original: null, year: 2024, rating: 8, poster: null, genres: ['Фантастика'], local: null, mark: null, wished: false };
const found = { ...film, id: 'tt1111111', title: 'Найденный поиском' };

test('знакомство: фильм выбирается через поиск по каталогу', async ({ page }) => {
  await page.addInitScript(() => sessionStorage.setItem('nova-onboarding:1', '1'));
  await page.route(/\/api\/discover(?:\?|$)/, (route) => route.fulfill({ json: { rows: [{ key: 'popular_movies', title: 'Популярное', items: [film] }] } }));
  await page.route('**/api/discover/search?*', (route) => route.fulfill({ json: [found] }));
  const saved: string[] = [];
  await page.route('**/api/me/favorites', (route) => {
    saved.push(route.request().postDataJSON().key);
    return route.fulfill({ json: { favorites: saved } });
  });
  await page.goto(url('/'));
  const guide = page.getByRole('dialog', { name: 'Знакомство с Nova' });
  await expect(guide).toBeVisible();
  await guide.getByRole('button', { name: 'Продолжить' }).click();
  await guide.getByRole('button', { name: 'Продолжить' }).click();
  await guide.getByRole('searchbox', { name: 'Найти фильм' }).fill('найден');
  await guide.getByRole('button', { name: 'Найденный поиском' }).click();
  // Выбранный фильм виден плашкой, его можно убрать.
  await expect(guide.getByRole('button', { name: /Найденный поиском/ }).first()).toBeVisible();
  await guide.getByRole('button', { name: 'Начать смотреть' }).click();
  await expect.poll(() => saved).toContain('x:tt1111111');
  await expect(guide).toHaveCount(0);
});

test('экскурсия сама открывает разделы и подсвечивает место', async ({ page }) => {
  await page.route('**/api/links/search**', (route) => route.fulfill({ json: [] }));
  await page.route('**/api/discover/title/*/trailer/hd', (route) => route.fulfill({ status: 200, contentType: 'video/mp4', body: '' }));
  await page.route('**/api/discover/title/*/trailer', (route) => route.fulfill({ json: { linkId: null } }));
  await page.goto(url('/search'));
  const tour = page.getByRole('dialog', { name: 'Экскурсия по Nova' });
  // Слушатель события ставится, когда страница запустилась (не сразу после загрузки): повторяем, пока экскурсия не откроется.
  await expect(async () => {
    await page.evaluate(() => window.dispatchEvent(new Event('nova-tour')));
    await expect(tour).toBeVisible({ timeout: 600 });
  }).toPass({ timeout: 10000 });
  await expect(tour.getByRole('heading', { name: 'Добро пожаловать в Nova' })).toBeVisible();
  // Шаг «Главная: баннер» - страница сама перешла на главную и подсвечен баннер.
  await tour.getByRole('button', { name: 'Дальше' }).click();
  await expect(tour.getByRole('heading', { name: 'Главная: баннер' })).toBeVisible();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('.spot')).toBeVisible();
  // «Что смотреть» - страница ленты и подсвеченная карточка.
  // Пока шаг сменяется, на миг видны обе подписи: ждём, пока останется одна, и только потом жмём дальше.
  for (const step of ['Разделы кинотеатра', 'Панель разделов', 'Что смотреть']) {
    await tour.getByRole('button', { name: 'Дальше' }).last().click();
    await expect(tour.getByRole('heading', { name: step })).toHaveCount(1);
  }
  await expect(tour.getByRole('heading', { name: 'Что смотреть' })).toBeVisible();
  await expect(page).toHaveURL(/\/feed$/);
  await expect(page.locator('.spot')).toBeVisible();
  // Стрелка влево возвращает на шаг назад, Esc закрывает.
  await page.keyboard.press('ArrowLeft');
  await expect(tour.getByRole('heading', { name: 'Панель разделов' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(tour).toHaveCount(0);
});
