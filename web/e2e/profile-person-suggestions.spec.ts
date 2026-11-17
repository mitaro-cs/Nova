import { expect, test } from './util-test';
import { env, url } from './util';

test('профиль занимает широкий экран, а поиск предлагает русские имена', async ({ page }, info) => {
  await page.goto(url(`/people/${env().people.me}`));
  const profile = page.locator('.pf');
  await expect(profile).toBeVisible();
  if (info.project.name === 'desktop') {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await expect.poll(async () => (await profile.boundingBox())?.width ?? 0).toBeGreaterThan(1800);
  }
  const editor = page.getByRole('region', {name: 'Любимые актёры и актрисы'});
  await editor.getByRole('button', {name: 'Изменить'}).click();
  const input = editor.getByRole('combobox', {name: 'Имя актёра или актрисы'});
  await input.fill('Скорсезе');
  const suggestion = editor.getByRole('option', {name: 'Мартин Скорсезе'});
  await expect(suggestion).toBeVisible();
  await suggestion.click();
  await expect(input).toHaveValue('Мартин Скорсезе');
});
