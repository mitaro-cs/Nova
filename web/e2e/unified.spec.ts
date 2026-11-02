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

test('одна панель разделов на телефоне, планшете и компьютере', async ({ page }) => {
  await offline(page);
  await page.goto(url('/'));
  const links = bar(page).getByRole('link');
  // У вкладки «Профиль» бывает число (просьбы и сообщения) - название всё равно в тексте.
  await expect(links).toHaveText([/Главное/, /Что смотреть/, /Профиль/, /Коллекция/, /Поиск/]);
  await expect(bar(page)).toBeVisible();
  await expect(bar(page)).toHaveCSS('background-color', 'rgb(20, 20, 20)');
  // Панель - у нижнего края окна, вкладки на виду.
  const box = await bar(page).boundingBox();
  const height = page.viewportSize()!.height;
  expect(box!.y + box!.height).toBeGreaterThan(height - 2);
  for (const name of ['Главное', 'Что смотреть', 'Профиль', 'Коллекция', 'Поиск']) await expect(bar(page).getByRole('link', { name })).toBeVisible();
  // Аватарного меню в шапке больше нет: профиль - вкладка панели.
  await expect(page.getByRole('button', { name: /^Меню/ })).toHaveCount(0);
});

test('вкладки панели ведут на свои страницы', async ({ page }) => {
  await offline(page);
  await page.goto(url('/'));
  await bar(page).getByRole('link', { name: 'Коллекция' }).click();
  await expect(page).toHaveURL(/\/my$/);
  await expect(page.getByRole('heading', { name: 'Коллекция', level: 1 })).toBeVisible();
  await expect(page.locator('main').getByRole('link', { name: /Подборки/ })).toBeVisible();
  await bar(page).getByRole('link', { name: 'Что смотреть' }).click();
  await expect(page).toHaveURL(/\/feed$/);
  await bar(page).getByRole('link', { name: 'Профиль' }).click();
  await expect(page).toHaveURL(/\/people\/\d+$/);
  // Чаты, подборки и выход - на странице профиля (в меню аватара их больше нет).
  const quick = page.getByRole('navigation', { name: 'Быстрые разделы' });
  await expect(quick.getByRole('link', { name: /Чаты/ })).toBeVisible();
  await expect(quick.getByRole('link', { name: /Подборки/ })).toBeVisible();
});

test('полоса разделов и кнопки героя - на всех ширинах', async ({ page }) => {
  await offline(page);
  await page.goto(url('/?tab=movies'));
  await expect(page.getByRole('navigation', { name: 'Разделы кинотеатра' })).toBeVisible();
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

test('пустая библиотека: полоса разделов не уезжает под шапку', async ({ page }) => {
  await offline(page);
  await page.route('**/api/library', async (route) => {
    const res = await route.fetch();
    const json = await res.json();
    await route.fulfill({ response: res, json: Array.isArray(json) ? [] : { ...json, items: [], recent: [] } });
  });
  await page.goto(url('/?tab=movies'));
  const strip = await page.getByRole('navigation', { name: 'Разделы кинотеатра' }).boundingBox();
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

test('«Что смотреть»: кадр из фильма и краткая информация', async ({ page }) => {
  await offline(page);
  await page.goto(url('/feed'));
  const card = page.locator('.fc').first();
  await expect(card).toBeVisible();
  await expect(card.locator('h2')).not.toBeEmpty();
  await expect(card.locator('.facts')).toContainText(/\d{4}/);
  await expect(card.locator('.shot img')).toBeVisible();
});

test('«Что смотреть» на компьютере - на всю ширину, кадр слева', async ({ page }, info) => {
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
