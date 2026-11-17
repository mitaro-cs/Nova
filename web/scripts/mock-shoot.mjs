// Снимки макетов docs/superpowers/mockups/index.html в 4 размерах.
// Запуск из web/: node scripts/mock-shoot.mjs [папка_результата]
// Нужен chromium: npx playwright install chromium
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';

const out = resolve(process.argv[2] ?? '../docs/superpowers/mockups/shots');
mkdirSync(out, { recursive: true });
const page_url = 'file:///' + resolve('../docs/superpowers/mockups/index.html').replace(/\\/g, '/');

const VIEWPORTS = {
  phone: { width: 390, height: 844 },
  tablet: { width: 768, height: 1024 },
  desk: { width: 1440, height: 900 },
  tv: { width: 1920, height: 1080 },
};

// [screen, variant, viewport]
const SHOTS = [
  ['account', 'a', 'phone'], ['account', 'a', 'desk'],
  ['account', 'b', 'phone'], ['account', 'b', 'desk'],
  ['settings', '', 'desk'], ['settings', '', 'phone'],
  ['player', 'desk', 'desk'], ['player', 'phone', 'phone'], ['player', 'tv', 'tv'],
  ['profile', 'desk', 'desk'], ['profile', 'phone', 'phone'],
  ['search', 'suggest', 'phone'], ['search', 'typo', 'phone'], ['search', 'nores', 'phone'], ['search', 'empty', 'phone'],
  ['chat', '', 'desk'],
  ['states', '', 'desk'], ['states', '', 'phone'],
  ['readme', '', 'desk'],
];

const browser = await chromium.launch();
for (const [s, v, vp] of SHOTS) {
  const ctx = await browser.newContext({ viewport: VIEWPORTS[vp], deviceScaleFactor: vp === 'phone' ? 2 : 1 });
  const page = await ctx.newPage();
  const url = `${page_url}?s=${s}${v ? '&v=' + v : ''}`;
  await page.goto(url);
  await page.waitForTimeout(300);
  const name = `${s}${v ? '-' + v : ''}-${vp}.png`;
  await page.screenshot({ path: join(out, name), fullPage: s !== 'player' });
  console.log(name);
  await ctx.close();
}
await browser.close();
console.log('готово:', out);
