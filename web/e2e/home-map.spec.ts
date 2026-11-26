import { expect, test } from './util-test';
import { expectClean, url } from './util';

// «Карта дома» (23.11): первый экран управления - вся система схемой, карточка открывает свои настройки справа.
test('карта дома: части системы, состояние и настройки по нажатию', async ({ page }) => {
  await page.goto(url('/admin?tab=map'));
  await expect(page.getByRole('heading', { name: 'Карта дома' })).toBeVisible();
  for (const name of ['Папки с фильмами', 'Описания и видео', 'Nova на этом компьютере', 'Интернет', 'Зрители']) {
    await expect(page.getByRole('button', { name: new RegExp(name) }).first()).toBeVisible();
  }
  await expect(page.getByRole('region', { name: 'Требует внимания' })).toBeVisible();
  await expectClean(page, 'карта дома');
  // Карточка «Интернет» открывает настройки доступа справа, Esc закрывает.
  await page.getByRole('button', { name: /Интернет/ }).first().click();
  const panel = page.getByRole('dialog', { name: 'Вход из интернета' });
  await expect(panel).toBeVisible();
  await expect(panel.getByText('Только дома')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(panel).toBeHidden();
  // Папки - список папок с фильмами.
  await page.getByRole('button', { name: /Папки с фильмами/ }).click();
  await expect(page.getByRole('dialog', { name: 'Папки с фильмами' })).toBeVisible();
});
