import { expect, test } from './util-test';
import { env, expectClean, stranger, url } from './util';

// Личный профиль: вкладки, новые виджеты статистики, приватность разделов, любимые актёры и актрисы.

test('профиль: вкладки, статистика и переключатель «кто видит»', async ({ page }) => {
  await page.goto(url(`/people/${env().people.me}`));
  const tabs = page.getByRole('tablist', { name: 'Разделы профиля' });
  await expect(tabs.getByRole('tab', { name: /Обзор/ })).toHaveAttribute('aria-selected', 'true');
  // На «Обзоре» переключателя нет: он только у закрываемых разделов.
  await expect(page.getByRole('radiogroup', { name: 'Кто видит этот раздел' })).toHaveCount(0);

  await tabs.getByRole('tab', { name: /Статистика/ }).click();
  await expect(page).toHaveURL(/tab=stats/);
  for (const name of ['Время просмотра', 'Когда смотрит', 'Дни недели', 'Оценки', 'Эпохи', 'По месяцам']) {
    await expect(page.getByRole('region', { name })).toBeVisible();
  }
  const pick = page.getByRole('radiogroup', { name: 'Кто видит этот раздел' });
  await expect(pick).toBeVisible();
  await expectClean(page, 'профиль: статистика');

  // Закрыть и открыть обратно: настройка сохраняется на сервере.
  await pick.getByRole('radio', { name: 'Друзья' }).click();
  await expect(pick.getByRole('radio', { name: 'Друзья' })).toHaveAttribute('aria-checked', 'true');
  await page.reload();
  await expect(page.getByRole('radiogroup', { name: 'Кто видит этот раздел' }).getByRole('radio', { name: 'Друзья' })).toHaveAttribute('aria-checked', 'true');
  await page.getByRole('radiogroup', { name: 'Кто видит этот раздел' }).getByRole('radio', { name: 'Все' }).click();
  await expect(page.getByRole('radiogroup', { name: 'Кто видит этот раздел' }).getByRole('radio', { name: 'Все' })).toHaveAttribute('aria-checked', 'true');

  await page.getByRole('tab', { name: /Просмотрено/ }).click();
  await expect(page).toHaveURL(/tab=watched/);
  await expect(page.getByRole('region', { name: 'Лента просмотренного' })).toBeVisible();
  await page.getByRole('tab', { name: /Отзывы/ }).click();
  await expect(page).toHaveURL(/tab=reviews/);
  await expectClean(page, 'профиль: отзывы');
});

test('профиль: закрытое владельцем - замок, без цифр', async ({ page }, info) => {
  const closed = await stranger(`Закрытый ${info.project.name} ${Date.now() % 100000}`);
  await closed.c.patch('/api/me/profile', { data: { visStats: 'me', visWatched: 'me', visReviews: 'friends' } });
  await page.goto(url(`/people/${closed.id}`));
  await expect(page.getByText('Цифры скрыты')).toBeVisible();
  // Чужой профиль: переключателя нет, а в закрытых разделах - «Скрыто владельцем».
  const tabs = page.getByRole('tablist', { name: 'Разделы профиля' });
  await tabs.getByRole('tab', { name: /Статистика/ }).click();
  await expect(page.getByRole('note').filter({ hasText: 'Скрыто владельцем' })).toBeVisible();
  await expect(page.getByRole('radiogroup', { name: 'Кто видит этот раздел' })).toHaveCount(0);
  await tabs.getByRole('tab', { name: /Отзывы/ }).click();
  await expect(page.getByRole('note').filter({ hasText: 'Скрыто владельцем' })).toBeVisible();
  await expectClean(page, 'профиль: закрытый');
  await closed.c.dispose();
});

test('профиль: любимые актёры и актрисы - добавить и убрать', async ({ page }) => {
  const name = 'Тестовая Актриса';
  await page.goto(url(`/people/${env().people.me}`));
  const box = page.getByRole('region', { name: 'Любимые актёры и актрисы' });
  await box.getByRole('button', { name: 'Изменить' }).click();
  await box.getByLabel('Имя актёра или актрисы').fill(name);
  await box.getByRole('button', { name: 'Добавить' }).click();
  await expect(box.getByRole('link', { name })).toBeVisible();
  await expectClean(page, 'профиль: актёры');
  // Сохранилось на сервере.
  await page.reload();
  await expect(page.getByRole('region', { name: 'Любимые актёры и актрисы' }).getByRole('link', { name })).toBeVisible();
  const again = page.getByRole('region', { name: 'Любимые актёры и актрисы' });
  await again.getByRole('button', { name: 'Изменить' }).click();
  await again.getByRole('button', { name: `Убрать ${name}` }).click();
  await expect(again.getByRole('link', { name })).toHaveCount(0);
});
