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

test('экскурсия «попробуйте сами»: подсвечивает место, нажатие на него ведёт дальше', async ({ page }, info) => {
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
  // Шаг 1: страница сама перешла на главную и подсвечена витрина.
  await expect(tour.getByRole('heading', { name: 'Витрина' })).toBeVisible();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('.spot')).toBeVisible();
  // Шаг 2 по «Дальше»: экскурсия сама открыла фильм и подсветила звёзды.
  await tour.getByRole('button', { name: 'Дальше' }).click();
  await expect(tour.getByRole('heading', { name: 'Оцените фильм' })).toHaveCount(1);
  await expect(page).toHaveURL(/\/(title|ext)\//);
  // На телефоне звёзды спрятаны в меню «…» - подсвечен раздел оценок, без подсказки про звезду.
  if (info.project.name === 'desktop') await expect(tour.getByText('Нажмите на звезду')).toBeVisible();
  // Нажатие на подсвеченное проходит к странице и само ведёт дальше. Ждём, пока подсветка доедет до места.
  let box = (await page.locator('.spot').boundingBox())!;
  await expect(async () => {
    const next = (await page.locator('.spot').boundingBox())!;
    const still = Math.abs(next.y - box.y) < 1 && Math.abs(next.x - box.x) < 1;
    box = next;
    expect(still).toBe(true);
  }).toPass({ intervals: [300], timeout: 6000 });
  await page.mouse.click(box.x + 30, box.y + box.height / 2);
  await expect(tour.getByRole('heading', { name: 'Смотреть' })).toHaveCount(1);
  // Стрелка влево возвращает на шаг назад, Esc закрывает.
  await page.keyboard.press('ArrowLeft');
  await expect(tour.getByRole('heading', { name: 'Оцените фильм' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(tour).toHaveCount(0);
});
