import type { Page } from '@playwright/test';
import { expect, test } from './util-test';
import { env, url } from './util';

// «Датчик целостности»: на каждом экране (телефон, планшет, компьютер) нет ошибок JavaScript и
// консоли, нет ответов 4xx/5xx на запросы к самому серверу, нет битых видимых картинок, страница не
// шире экрана и не пустая, видимый заголовок h1 - не больше одного. Нашли новое - чиним, а не
// добавляем в исключения.

const PAGES = [
  ['/', 'главная'],
  ['/?tab=movies', 'фильмы'],
  ['/?tab=series', 'сериалы'],
  ['/?tab=anime', 'аниме'],
  ['/search?q=на', 'поиск'],
  ['/profile', 'профиль'],
  ['/lists', 'списки'],
  ['/discover', 'обзор'],
  ['/ext/tt1196946', 'фильм из IMDb'],
  ['/admin', 'управление'],
  ['/admin?tab=folders', 'папки'],
  ['/admin?tab=catalog', 'каталог'],
  ['/admin?tab=people', 'люди (управление)'],
  ['/admin?tab=access', 'доступ'],
  ['/admin?tab=settings', 'настройки'],
  ['/admin?tab=version', 'версия'],
  ['/people', 'люди'],
  ['/chats', 'чаты'],
] as const;

type Finding = string;

function watch(page: Page) {
  const found: Finding[] = [];
  const origin = new URL(page.url() === 'about:blank' ? env().baseURL : page.url()).origin;
  page.on('console', (m) => {
    // 404 и подобное ловит проверка ответов ниже, без дубля.
    if (m.type() === 'error' && !m.text().startsWith('Failed to load resource')) found.push(`консоль: ${m.text().slice(0, 140)}`);
  });
  page.on('pageerror', (e) => found.push(`JS: ${e.message.slice(0, 140)}`));
  page.on('response', (r) => {
    const u = r.url();
    if (r.status() >= 400 && (u.startsWith(env().baseURL) || u.startsWith(env().publicURL) || u.startsWith(origin))) {
      found.push(`${r.status()} ${new URL(u).pathname}`);
    }
  });
  return found;
}

async function inspect(page: Page, found: Finding[]) {
  const dom = await page.evaluate(() => {
    const visible = (el: Element) => (el as HTMLElement).checkVisibility({ checkVisibilityCSS: true, checkOpacity: true });
    const root = document.documentElement;
    // html и body обрезают лишнее справа (overflow-x: clip): на время замера обрезку снимаем.
    root.style.overflowX = 'visible';
    document.body.style.overflowX = 'visible';
    return {
      wider: root.scrollWidth - root.clientWidth,
      broken: [...document.images]
        .filter((i) => i.complete && i.naturalWidth === 0 && i.currentSrc && visible(i))
        .map((i) => i.currentSrc.slice(-60)),
      text: document.body.innerText.trim().length,
      h1: [...document.querySelectorAll('h1')].filter(visible).length,
    };
  });
  if (dom.wider > 1) found.push(`страница шире экрана на ${dom.wider} px`);
  for (const b of dom.broken) found.push(`битая картинка: ${b}`);
  if (dom.text < 20) found.push('страница пустая');
  if (dom.h1 > 1) found.push(`видимых h1: ${dom.h1}`);
}

async function check(page: Page, path: string, what: string) {
  // Поиск видео ходит на внешние сайты: без интернета (CI) сервер честно отвечает 400 «источники не
  // отвечают». Датчик проверяет наш код, а не сеть - поиск подменяем пустым ответом.
  await page.route('**/api/links/search**', (route) => route.fulfill({ json: [] }));
  const found = watch(page);
  await page.goto(url(path));
  await page.waitForLoadState('networkidle', { timeout: 8000 }).catch(() => {});
  await page.waitForTimeout(1500);
  await inspect(page, found);
  expect([...new Set(found)], `${what} (${path})`).toEqual([]);
}

for (const [path, what] of PAGES) {
  test(`целостность: ${what}`, async ({ page }) => {
    await check(page, path, what);
  });
}

// Сам датчик: заведомые нарушения должны быть пойманы (иначе зелёный обход ничего не доказывает).
test('целостность: датчик ловит 404, ошибку консоли, битую картинку и переполнение', async ({ page }) => {
  await page.goto(url('/'));
  const found = watch(page);
  await page.evaluate(async () => {
    await fetch('/api/discover/title/bad/reviews');
    console.error('проверка датчика');
    const img = document.createElement('img');
    img.src = '/img/no-such-image.jpg';
    img.width = 20;
    img.height = 20;
    document.body.append(img);
    const wide = document.createElement('div');
    wide.style.cssText = 'width:5000px;height:2px';
    document.body.append(wide);
    await new Promise((r) => setTimeout(r, 800));
  });
  await inspect(page, found);
  const text = found.join('\n');
  expect(text).toContain('404 /api/discover/title/bad/reviews');
  expect(text).toContain('консоль: проверка датчика');
  expect(text).toContain('битая картинка');
  expect(text).toContain('страница шире экрана');
});

test('целостность: страницы фильма и людей', async ({ page }) => {
  const { people, ids } = env();
  for (const [path, what] of [
    [`/title/${ids['Ла-Ла Ленд']}`, 'фильм'],
    [`/title/${ids['Тьма']}`, 'сериал'],
    [`/people/${people.anna}`, 'страница Анны'],
    [`/chats/${people.anna}`, 'переписка'],
  ] as const) {
    await check(page, path, what);
  }
});
