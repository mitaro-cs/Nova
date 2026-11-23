import type { Page } from '@playwright/test';
import { expect, test } from './util-test';
import { url } from './util';
import { expectClean } from './util';

// Единый стиль: одна панель разделов на всех экранах, одни и те же герой и ряды, постеры растут вместе с экраном.

async function offline(page: Page) {
  await page.route('**/api/links/search**', (route) => route.fulfill({ json: [] }));
  await page.route('**/api/discover/title/*/trailer/hd', (route) => route.fulfill({ status: 200, contentType: 'video/mp4', body: '' }));
  await page.route('**/api/discover/title/*/trailer', (route) => route.fulfill({ json: { linkId: null } }));
}

const bar = (page: Page) => page.getByRole('navigation', { name: 'Разделы', exact: true });

const desk = (page: Page) => page.viewportSize()!.width > 1000;
const top = (page: Page) => page.getByRole('navigation', { name: 'Разделы кинотеатра' });
/** Перейти в раздел: на телефоне и планшете - нижней панелью, на компьютере - шапкой («Витрина»). */
async function goTo(page: Page, name: 'Моё' | 'Подбор' | 'Каталог' | 'Профиль') {
  if (!desk(page)) return bar(page).getByRole('link', { name }).click();
  if (name === 'Профиль') return page.getByRole('link', { name: 'Мой профиль' }).click();
  return top(page).getByRole('link', { name }).click();
}

test('разделы: внизу на телефоне и планшете, сверху на компьютере', async ({ page }) => {
  await offline(page);
  await page.goto(url('/'));
  const { width, height } = page.viewportSize()!;
  if (width > 1000) {
    // Компьютер («Витрина»): знак, разделы текстом, поиск, колокольчик и профиль - всё в шапке; нижней панели нет.
    await expect(bar(page)).toBeHidden();
    // Четыре раздела (S1, 23.11): Фильмы, Сериалы и Аниме - полосой внутри Каталога.
    const labels = [/Главная/, /Подбор/, /Каталог/, /Моё/];
    await expect(top(page).getByRole('link')).toHaveText(labels);
    await expect(page.getByRole('searchbox', { name: 'Быстрый поиск' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Мой профиль' })).toBeVisible();
    // Выбранный раздел подчёркнут одной чертой, она стоит под ним.
    const on = await top(page).locator('a.on').boundingBox();
    const ink = await top(page).locator('.ink').boundingBox();
    expect(Math.abs(ink!.x - on!.x)).toBeLessThan(3);
    expect(Math.abs(ink!.width - on!.width)).toBeLessThan(3);
  } else {
    const links = bar(page).getByRole('link');
    // У вкладки «Профиль» бывает число (просьбы и сообщения) - название всё равно в тексте.
    await expect(links).toHaveText([/Главная/, /Подбор/, /Каталог/, /Моё/, /Профиль/]);
    // Фон панели - сплошной (цвет дизайна менялся), главное - что она не прозрачная.
    await expect(bar(page)).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
    const box = await bar(page).boundingBox();
    expect(box!.y + box!.height).toBeGreaterThan(height - 2);
    for (const name of ['Главная', 'Подбор', 'Каталог', 'Моё', 'Профиль']) await expect(bar(page).getByRole('link', { name })).toBeVisible();
  }
  // Аватарного меню в шапке больше нет: профиль - ссылка.
  await expect(page.getByRole('button', { name: /^Меню/ })).toHaveCount(0);
});

test('разделы ведут на свои страницы', async ({ page }) => {
  await offline(page);
  await page.goto(url('/'));
  await goTo(page, 'Моё');
  await expect(page).toHaveURL(/\/my$/);
  await expect(page.getByRole('heading', { name: 'Моё', level: 1 })).toBeVisible();
  await expect(page.locator('main').getByRole('link', { name: /Подборки/ })).toBeVisible();
  await goTo(page, 'Подбор');
  await expect(page).toHaveURL(/\/feed$/);
  await expect(page.getByRole('heading', { name: 'Подбор', level: 1 })).toBeVisible();
  await goTo(page, 'Каталог');
  await expect(page).toHaveURL(/\/discover$/);
  // Внутри Каталога - полоса «Всё · Фильмы · Сериалы · Аниме».
  await page.getByRole('navigation', { name: 'Каталог' }).getByRole('link', { name: 'Сериалы' }).click();
  await expect(page).toHaveURL(/tab=series$/);
  await goTo(page, 'Профиль');
  await expect(page).toHaveURL(/\/people\/\d+$/);
  // Чаты, подборки и выход - на странице профиля (в меню аватара их больше нет).
  // Чаты, подборки и экскурсия - в быстрых разделах на странице профиля (на всех ширинах).
  const quick = page.getByRole('navigation', { name: 'Быстрые разделы' });
  await expect(quick.getByRole('link', { name: /Чаты/ })).toBeVisible();
  await expect(quick.getByRole('link', { name: /Подборки/ })).toBeVisible();
});
test('полоса разделов и кнопки героя - на всех ширинах', async ({ page }) => {
  await offline(page);
  await page.goto(url('/?tab=movies'));
  await expect(page.getByRole('navigation', { name: 'Каталог' })).toBeVisible();
  const hero = page.getByRole('region', { name: 'Рекомендуем' });
  await expect(hero.getByRole('link', { name: /Смотреть|Продолжить/ })).toBeVisible();
  await expect(hero.getByRole('button', { name: 'Не интересно' })).toBeVisible();
});

test('кнопка героя белая, цвета фильма нет', async ({ page }) => {
  await offline(page);
  await page.goto(url('/'));
  const hero = page.getByRole('region', { name: 'Рекомендуем' });
  const button = hero.getByRole('link', { name: /Смотреть|Продолжить/ });
  await expect(button).toBeVisible();
  await expect(button).toHaveCSS('background-color', 'rgb(255, 255, 255)');
  expect(await page.evaluate(() => document.documentElement.getAttribute('style') ?? '')).not.toContain('--film');
});

test('пустая библиотека: полоса каталога не уезжает под шапку', async ({ page }) => {
  await offline(page);
  await page.route('**/api/library', async (route) => {
    const res = await route.fetch();
    const json = await res.json();
    await route.fulfill({ response: res, json: Array.isArray(json) ? [] : { ...json, items: [], recent: [] } });
  });
  await page.goto(url('/?tab=movies'));
  const strip = await page.getByRole('navigation', { name: 'Каталог' }).boundingBox();
  const header = await page.locator('header').boundingBox();
  expect(strip && header && strip.y >= header.y + header.height - 1).toBe(true);
});

test('пустой поиск - подборки и категории', async ({ page }) => {
  await offline(page);
  await page.goto(url('/search'));
  await expect(page.getByRole('region', { name: 'Категории' })).toBeVisible();
});

test('постеры растут вместе с экраном', async ({ page }) => {
  await offline(page);
  await page.goto(url('/my'));
  const tile = page.locator('.tile .art').first();
  await expect(tile).toBeVisible();
  const width = (await tile.boundingBox())!.width;
  const vw = page.viewportSize()!.width;
  const expected = Math.min(320, Math.max(148, 125 + 0.06 * vw));
  expect(Math.abs(width - expected), `постер ${width} px при ширине ${vw}`).toBeLessThan(3);
  await expectClean(page, 'коллекция');
});

test('«Подбор»: кадр из фильма и краткая информация', async ({ page }) => {
  await offline(page);
  await page.goto(url('/feed'));
  const card = page.locator('.fc').first();
  await expect(card).toBeVisible();
  await expect(card.locator('h2')).not.toBeEmpty();
  await expect(card.locator('.facts')).toContainText(/\d{4}/);
  await expect(card.locator('.shot img')).toBeVisible();
});

test('«Подбор» на компьютере - на всю ширину, кадр слева', async ({ page }, info) => {
  test.skip(info.project.name !== 'desktop', 'только компьютер');
  await offline(page);
  await page.goto(url('/feed'));
  const card = page.locator('.fc').first();
  await expect(card).toBeVisible();
  const box = await card.boundingBox();
  expect(box!.width).toBeGreaterThan(page.viewportSize()!.width * 0.8);
  const shot = await card.locator('.shot').boundingBox();
  const text = await card.locator('.info').boundingBox();
  expect(shot!.x + shot!.width).toBeLessThanOrEqual(text!.x);
});

test('подборки: страница называется «Подборки»', async ({ page }) => {
  await offline(page);
  await page.goto(url('/lists'));
  await expect(page.getByRole('heading', { name: 'Подборки', level: 1 })).toBeVisible();
});
