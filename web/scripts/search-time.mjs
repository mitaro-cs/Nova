// Время автопоиска видео, как его делает страница фильма (нужен интернет):
//   node web/scripts/search-time.mjs <baseURL>
// Вход - через e2e/.state.json (запустите демо-сервер: node --experimental-strip-types с e2e/setup.ts).
// Сначала LordFilm; пусто - остальные источники параллельно (запасной ход). Печатает время до первой
// находки и общее время, число находок и откуда они.
import { chromium } from '@playwright/test';

const base = process.argv[2];
if (!base) {
  console.error('Использование: node web/scripts/search-time.mjs <baseURL>');
  process.exit(2);
}
const OTHERS = ['rutube', 'vk', 'archive', 'peertube', 'commons'];
const cases = [
  ['Дюна 2021 (популярный)', 'Дюна 2021', 40],
  ['Первый человек 2018', 'Первый человек 2018', 40],
  ['Прибытие 2016', 'Прибытие 2016', 40],
  ['Minecraft в кино 2025 (новый)', 'Minecraft в кино 2025', 40],
  ['Дьявол носит Prada 2 (2026, новый)', 'Дьявол носит Prada 2 2026', 40],
  ['Ghost Soldier (малоизвестный)', 'Ghost Soldier', 40],
  ['Some Kind of Heaven (2020, нишевый)', 'Some Kind of Heaven 2020', 40],
  ['Во все тяжкие (сериал)', 'Во все тяжкие', 15],
];

const browser = await chromium.launch();
const ctx = await browser.newContext({
  storageState: new URL('../e2e/.state.json', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'),
  locale: 'ru-RU',
});
const page = await ctx.newPage();
await page.goto(base + '/');
for (const [label, q, min] of cases) {
  const r = await page.evaluate(
    async ([q, min, others]) => {
      const t0 = performance.now();
      const get = async (provider, withMin) => {
        const p = new URLSearchParams({ q, provider });
        if (withMin) p.set('min', String(min * 60));
        try {
          const res = await fetch('/api/links/search?' + p, { headers: { 'X-Fsfy': '1' } });
          const items = await res.json();
          return Array.isArray(items) ? items : [];
        } catch {
          return [];
        }
      };
      const first = await get('lordfilm', false);
      const lordMs = Math.round(performance.now() - t0);
      if (first.length) return { lordMs, firstMs: lordMs, totalMs: lordMs, n: first.length, from: 'lordfilm' };
      let firstMs = null;
      const lists = await Promise.all(
        others.map(async (o) => {
          const items = await get(o, true);
          if (items.length && firstMs === null) firstMs = Math.round(performance.now() - t0);
          return items.map((i) => i.site);
        }),
      );
      const sites = lists.flat();
      return { lordMs, firstMs, totalMs: Math.round(performance.now() - t0), n: sites.length, from: [...new Set(sites)].join(',') };
    },
    [q, min, OTHERS],
  );
  console.log(
    `${label.padEnd(38)} LordFilm ${String(r.lordMs).padStart(5)} мс | первая находка ${r.firstMs === null ? '  нет' : String(r.firstMs).padStart(5) + ' мс'} | всего ${String(r.totalMs).padStart(5)} мс | находок ${r.n} [${r.from}]`,
  );
}
await browser.close();
