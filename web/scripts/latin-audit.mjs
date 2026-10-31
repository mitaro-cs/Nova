// Видимая латиница на экранах (нужен интернет): node web/scripts/latin-audit.mjs <baseURL>
// Вход - через e2e/.state.json (запустите демо-сервер: node --experimental-strip-types с e2e/setup.ts).
// Подсказки `title` у карточек людей пропускаются: оригинал имени там по замыслу.
import { chromium } from '@playwright/test';

const base = process.argv[2];
if (!base) {
  console.error('Использование: node web/scripts/latin-audit.mjs <baseURL>');
  process.exit(2);
}
const pages = [
  ['главная', '/'],
  ['фильмы', '/?tab=movies'],
  ['сериалы', '/?tab=series'],
  ['обзор', '/discover'],
  ['фильм ленты: Первый человек', '/ext/tt1213641'],
  ['фильм ленты: Прибытие', '/ext/tt2543164'],
  ['сериал ленты: Во все тяжкие', '/ext/tt0903747'],
  ['человек: Нолан', '/person?name=Christopher%20Nolan&role=director&id=nm0634240'],
  ['актёр: Гослинг', '/person?name=Ryan%20Gosling&role=actor&id=nm0331516'],
  ['поиск', '/search?q=nolan'],
  ['профиль', '/profile'],
];
const ALLOW = /^(Nova|IMDb|HD|4K|VK|Rutube|YouTube|TMDB|TV|HDR|alpha|host)$/i;

const browser = await chromium.launch();
const ctx = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  storageState: new URL('../e2e/.state.json', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'),
  locale: 'ru-RU',
});
const page = await ctx.newPage();
for (const [name, path] of pages) {
  await page.goto(base + path, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(9000);
  const found = await page.evaluate(() => {
    const out = new Map();
    const add = (s, where) => {
      for (const w of s.match(/[A-Za-z][A-Za-z'’.-]{2,}/g) ?? []) if (!out.has(w)) out.set(w, `${where}: ${s.trim().slice(0, 70)}`);
    };
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const p = walker.currentNode.parentElement;
      if (!p || ['SCRIPT', 'STYLE', 'NOSCRIPT'].includes(p.tagName)) continue;
      const cs = getComputedStyle(p);
      if (cs.display === 'none' || cs.visibility === 'hidden') continue;
      add(walker.currentNode.nodeValue ?? '', p.tagName.toLowerCase());
    }
    for (const el of document.querySelectorAll('[alt],[placeholder],[aria-label]')) {
      for (const a of ['alt', 'placeholder', 'aria-label']) {
        const v = el.getAttribute(a);
        if (v) add(v, '@' + a);
      }
    }
    return [...out.entries()];
  });
  const bad = found.filter(([w]) => !ALLOW.test(w));
  console.log(`\n== ${name} (${path}) - латиница: ${bad.length}`);
  for (const [w, where] of bad.slice(0, 40)) console.log(`  ${w.padEnd(18)} ${where}`);
}
await browser.close();
