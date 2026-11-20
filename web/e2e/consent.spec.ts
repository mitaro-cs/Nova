import { request } from '@playwright/test';
import { join } from 'node:path';
import { expect, test } from './util-test';
import { env, expectClean, url } from './util';

// Политика конфиденциальности и согласие: страница с документом читается целиком, регистрация без галочки невозможна,
// прежний аккаунт видит окно согласия и до принятия никуда не попадает.

test('политика: отдельная страница с оглавлением и редакцией', async ({ page }) => {
  await page.goto(url('/legal/privacy'));
  await expect(page.getByRole('heading', { level: 1, name: 'Политика конфиденциальности' })).toBeVisible();
  await expect(page.getByText(/редакция \d+ от/)).toBeVisible();
  await expect(page.getByRole('complementary', { name: 'Оглавление' }).getByRole('link', { name: 'Ваши права' })).toBeVisible();
  await expect(page.getByRole('heading', { name: /Какие данные сохраняются/ })).toBeVisible();
  await expectClean(page, 'политика конфиденциальности');
  await page.goto(url('/legal/terms'));
  await expect(page.getByRole('heading', { level: 1, name: 'Правила использования' })).toBeVisible();
});

test('регистрация: без согласия кнопка закрыта, сервер тоже отказывает', async ({ browser }) => {
  const ctx = await browser.newContext({ storageState: { cookies: [], origins: [] } });
  const page = await ctx.newPage();
  await page.goto(env().publicURL + '/register');
  const box = page.getByRole('checkbox', { name: /согласен/ });
  await expect(box).not.toBeChecked();
  await expect(page.getByRole('button', { name: 'Создать аккаунт' })).toBeDisabled();
  await box.check();
  await expect(page.getByRole('button', { name: 'Создать аккаунт' })).toBeEnabled();
  const r = await page.request.post(env().publicURL + '/api/auth/register', {
    headers: { 'x-fsfy': '1' },
    data: { name: 'Без согласия', login: `nc${Date.now() % 100000}`, password: 'пароль-12345' },
  });
  expect(r.status()).toBe(400);
  expect((await r.json()).error).toBe('consent_required');
  await ctx.close();
});

test('прежний аккаунт: окно согласия, пока не приняты документы', async ({ browser }) => {
  // Новый человек, заведённый хостом, ещё не принимал документы - как все аккаунты до появления политики.
  const host = await request.newContext({
    baseURL: env().baseURL,
    storageState: join(import.meta.dirname, '.state.json'),
    extraHTTPHeaders: { 'x-fsfy': '1' },
  });
  const login = `old${Date.now() % 1000000}`;
  expect((await host.post('/api/admin/users', { data: { name: `Прежний ${login}`, login, password: 'пароль-12345' } })).ok()).toBe(true);
  await host.dispose();
  const ctx = await browser.newContext({ storageState: { cookies: [], origins: [] } });
  const page = await ctx.newPage();
  await page.goto(env().publicURL + '/login');
  await page.getByLabel(/Логин/).fill(login);
  await page.getByLabel('Пароль', { exact: true }).fill('пароль-12345');
  await page.getByRole('button', { name: 'Войти' }).click();
  const gate = page.getByRole('heading', { name: 'Согласие с политикой конфиденциальности' });
  await expect(gate).toBeVisible();
  const go = page.getByRole('button', { name: 'Согласен и продолжить' });
  await expect(go).toBeDisabled();
  await expectClean(page, 'окно согласия');
  await page.getByRole('checkbox', { name: /согласен/ }).check();
  await go.click();
  await expect(gate).toBeHidden();
  await ctx.close();
});
