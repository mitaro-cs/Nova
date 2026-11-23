import { expect, test } from './util-test';
import { expectClean, url } from './util';

// «Управление → Обзор»: показатели, понятные подписи и два действия хозяина (кэш видео, копия базы).

test('показатели обзора: цифры, активность, диск, качество', async ({ page }) => {
  await page.goto(url('/admin?tab=overview'));
  const kpis = page.getByLabel('Главные цифры');
  await expect(kpis).toBeVisible();
  await expect(kpis).toContainText('Фильмов');
  await expect(kpis).toContainText('Сериалов');
  await expect(kpis).toContainText('Часов кино');
  await expect(kpis).toContainText('Смотрят сейчас');
  await expect(page.getByLabel('Активность', { exact: true })).toBeVisible();
  await expect(page.getByLabel('Самое смотримое')).toBeVisible();
  await expect(page.getByLabel('Место на диске')).toBeVisible();
  await expect(page.getByLabel('Качество файлов')).toBeVisible();
  await expect(page.getByLabel('Зрители и общение')).toBeVisible();
  // Лампочки состояния в шапке: сервер работает.
  await expect(page.getByLabel('Состояние Nova').getByText('Сервер')).toBeVisible();
  await expectClean(page, 'обзор с показателями');
});

test('метрики с сервера: числа сходятся с библиотекой', async ({ page }) => {
  await page.goto(url('/admin?tab=overview'));
  const data = await page.evaluate(async () => {
    const [m, s] = await Promise.all([fetch('/api/admin/metrics?tz=180').then((r) => r.json()), fetch('/api/admin/status').then((r) => r.json())]);
    return { m, s };
  });
  expect(data.m.library.movies + data.m.library.series).toBe(data.s.titles);
  expect(data.m.activity.days).toHaveLength(14);
  expect(data.m.activity.hours).toHaveLength(24);
  expect(Object.keys(data.m.disk)).toEqual(expect.arrayContaining(['db', 'images', 'hls', 'copy', 'backups']));
});

test('копия базы делается по кнопке', async ({ page }) => {
  await page.goto(url('/admin?tab=overview'));
  await page.getByRole('button', { name: /Копия базы сейчас/ }).click();
  await expect(page.getByText(/Копия базы готова/)).toBeVisible();
});

test('кнопка очистки кэша согласована с его размером', async ({ page }) => {
  // Другие тесты гоняют видео, и фоновая перепаковка дописывает кэш в любой момент: нельзя требовать «кэша нет».
  // Проверяем связь: кэша нет - кнопка недоступна, есть - доступна (на то же значение, что отдал сервер).
  const clear = page.getByRole('button', { name: /Очистить кэш видео/ });
  await expect(async () => {
    await page.goto(url('/admin?tab=overview'));
    await expect(clear).toBeVisible();
    const m = await page.evaluate(() => fetch('/api/admin/metrics?tz=0').then((r) => r.json()));
    const empty = m.disk.hls + m.disk.copy + m.disk.cacheOther === 0;
    if (empty) await expect(clear).toBeDisabled({ timeout: 2_000 });
    else await expect(clear).toBeEnabled({ timeout: 2_000 });
  }).toPass({ timeout: 25_000 });
});

test('вкладки управления: у каждой своя шапка с подсказками без флажков', async ({ page }) => {
  await page.goto(url('/admin?tab=folders'));
  const head = page.locator('.tab-head');
  await expect(head.getByRole('heading', { name: 'Папки' })).toBeVisible();
  // На компьютере список подсказок скрыт (дизайн «База»), описание раздела видно везде.
  await expect(head.locator('p').first()).toBeVisible();
  // В подсказках больше нет значка-галочки.
  await expect(head.locator('svg[aria-hidden] path[d^="M20 6 9 17"]')).toHaveCount(0);
  await expectClean(page, 'папки');
});
