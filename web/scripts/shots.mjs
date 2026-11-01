// Снимки экранов на четырёх ширинах для сравнения «до» и «после»:
//   node web/scripts/shots.mjs <baseURL> <метка> [часть-адреса ...]
// Вход - через e2e/.state.json (демо-сервер: node --experimental-strip-types с e2e/setup.ts).
// Снимки - в web/.shots/<метка>/<экран>-<ширина>.png (папка в .gitignore).
import { chromium } from '@playwright/test';
import { mkdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const [base, tag, ...only] = process.argv.slice(2);
if (!base || !tag) {
  console.error('Использование: node web/scripts/shots.mjs <baseURL> <метка> [часть-адреса ...]');
  process.exit(2);
}
const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const env = JSON.parse(readFileSync(here('../e2e/.env.json'), 'utf8'));
const film = env.ids['Ла-Ла Ленд'];
const PAGES = [
  ['home', '/'], ['movies', '/?tab=movies'], ['discover', '/discover'], ['search', '/search?q=на'],
  ['profile', '/profile'], ['profile-account', '/profile?tab=account'], ['people', '/people'],
  ['person-page', `/people/${env.people.me}`], ['chats', '/chats'], ['lists', '/lists'],
  ['admin', '/admin'], ['admin-folders', '/admin?tab=folders'], ['admin-catalog', '/admin?tab=catalog'],
  ['admin-people', '/admin?tab=people'], ['admin-access', '/admin?tab=access'], ['admin-settings', '/admin?tab=settings'],
  ['admin-version', '/admin?tab=version'], ['film', `/title/${film}`], ['ext', '/ext/tt1196946'],
];
const SIZES = [[1440, 900], [1024, 768], [768, 1024], [390, 844]];
const dir = here(`../.shots/${tag}`);
mkdirSync(dir, { recursive: true });
const browser = await chromium.launch();
const ctx = await browser.newContext({ storageState: here('../e2e/.state.json'), locale: 'ru-RU' });
const page = await ctx.newPage();
for (const [name, path] of PAGES) {
  if (only.length && !only.some((o) => name.includes(o) || path.includes(o))) continue;
  for (const [w, h] of SIZES) {
    await page.setViewportSize({ width: w, height: h });
    await page.goto(base + path, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(2200);
    await page.screenshot({ path: `${dir}/${name}-${w}.png` });
  }
  console.log('снято:', name);
}
await browser.close();
console.log('готово:', dir);
