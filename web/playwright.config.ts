import { defineConfig, devices } from '@playwright/test';

// Сквозные проверки против собранного сервера (`cargo build --release -p fsfy-server`) на
// демо-данных из e2e/setup.ts: телефон, планшет и компьютер.
// В CI основной набор - Chromium (телефон и компьютер): быстро и без WebKit. Видео-сценарии (нужен Chromium с H.264)
// идут отдельным необязательным job с E2E_MEDIA=1; WebKit и планшет - локально или E2E_ALL=1.
const MEDIA = ['**/player-controls.spec.ts', '**/player-speed.spec.ts', '**/hls-beta9.spec.ts', '**/party-beta9.spec.ts', '**/media-beta7.spec.ts', '**/mobile-beta8.spec.ts', '**/source-player-live.spec.ts'];
const media = !!process.env.E2E_MEDIA;
const all = !process.env.CI || !!process.env.E2E_ALL;

export default defineConfig({
  testDir: 'e2e',
  globalSetup: './e2e/setup.ts',
  fullyParallel: false,
  workers: 1,
  // В CI - два повтора: на общих машинах GitHub случаются медленные кадры (WebKit, переходы).
  retries: process.env.CI ? 2 : 0,
  testMatch: media ? MEDIA : undefined,
  testIgnore: media ? undefined : MEDIA,
  timeout: 30_000,
  expect: { timeout: 7_000 },
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    // PW_CHROMIUM - путь к готовому Chromium (песочницы без доступа к cdn.playwright.dev); в CI не задан.
    launchOptions: process.env.PW_CHROMIUM ? { executablePath: process.env.PW_CHROMIUM } : {},
    storageState: 'e2e/.state.json',
    locale: 'ru-RU',
    timezoneId: 'Europe/Moscow',
    trace: process.env.CI ? 'on-first-retry' : 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    ...(all ? [{ name: 'iphone-webkit', use: { ...devices['iPhone 13'], browserName: 'webkit' as const } }] : []),
    { name: 'phone', use: { ...devices['iPhone 13'], browserName: 'chromium' } },
    ...(all ? [{ name: 'tablet', use: { viewport: { width: 820, height: 1180 }, hasTouch: true } }] : []),
    { name: 'desktop', use: { viewport: { width: 1440, height: 900 } } },
  ],
});
