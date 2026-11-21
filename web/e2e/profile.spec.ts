import { expect, test } from './util-test';
import { env, expectClean, stranger, url } from './util';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

// Личный профиль: вкладки, новые виджеты статистики, приватность разделов, любимые актёры и актрисы.

test('профиль по макету: показатели, ободок, меню и адаптивная вёрстка', async ({ page }, info) => {
  const id=env().people.me;
  const profile=await (await page.request.get(url(`/api/members/${id}`))).json();
  profile.person.name='Омар';
  profile.stats={...profile.stats,hours:47,movies:63,episodes:458,series:3,reviews:35,friends:7,outside:10,watched:521,
    genres:['Драма','Приключения','Боевик','Криминал'].map(genre=>({genre,count:10}))};
  profile.decor.bio=null;
  profile.decor.ribbon=profile.watched.filter((c:{poster:string|null})=>c.poster).map((c:{poster:string})=>c.poster);
  const watching={id:'tt14452776',kind:'series',title:'Медведь',original:'The Bear',year:2022,rating:8.5,poster:profile.decor.ribbon[0]??null,genres:['Драма'],local:null,mark:'watching',wished:false};
  profile.watching=[{at:Date.now(),label:'Сезон 1 · серия 1',ext:watching}];
  await page.route(`**/api/members/${id}?*`,r=>r.fulfill({json:profile}));
  await page.goto(url(`/people/${id}`));
  // Профиль «Дневник» (макет Y, 21.11): шапка с именем и цифрами, справа - сейчас смотрит, достижения, быстрые разделы.
  await expect(page.getByRole('heading',{name:'Омар',level:1})).toBeVisible();
  await expect.poll(()=>page.getByLabel('Цифры').locator('b').first().textContent()).toBe('63');
  const side=page.getByRole('complementary',{name:'Коротко'});
  await expect(side.getByRole('region',{name:'Сейчас смотрит'})).toContainText('Медведь');
  await expect(side.getByLabel('Значки')).toContainText('Марафонец');
  await expect(side.getByLabel('Значки')).toContainText('Исследователь');
  const quick=side.getByRole('navigation',{name:'Быстрые разделы'});
  await expect(quick.getByRole('link',{name:/Чаты/})).toBeVisible();
  await expect(quick.getByRole('button',{name:/Экскурсия/})).toBeVisible();
  await expectClean(page,'профиль по макету');
  const shots=resolve('../docs/design/profile');mkdirSync(shots,{recursive:true});
  await page.screenshot({path:resolve(shots,`${info.project.name}.png`),fullPage:true});
  await page.screenshot({path:resolve(shots,`${info.project.name}-preview.png`)});
  if(info.project.name==='phone') {
    for(const width of [320,360,430]) {
      await page.setViewportSize({width,height:900});
      await expectClean(page,`профиль по макету: ${width}px`);
    }
  }
});

test('аватар-персонаж выбирается и сохраняется', async ({page})=>{
  const before=await (await page.request.get(url('/api/me'))).json();
  const avatar=before.user.profile.avatar;
  try {
    await page.goto(url('/profile'));
    const picker=page.getByRole('radiogroup',{name:'Аватар',exact:true});
    await expect(picker.getByRole('radio')).toHaveCount(12);
    await picker.getByRole('radio',{name:'Робот',exact:true}).click();
    await expect(picker.getByRole('radio',{name:'Робот',exact:true})).toHaveAttribute('aria-checked','true');
    await page.reload();
    await expect(page.getByRole('radio',{name:'Робот',exact:true})).toHaveAttribute('aria-checked','true');
    await expectClean(page,'выбор аватара');
  } finally {
    expect((await page.request.patch(url('/api/me'),{headers:{'X-Fsfy':'1'},data:{avatar}})).ok()).toBe(true);
  }
});

test('профиль: вкладки, статистика и переключатель «кто видит»', async ({ page }) => {
  await page.goto(url(`/people/${env().people.me}`));
  const tabs = page.getByRole('tablist', { name: 'Разделы профиля' });
  await expect(tabs.getByRole('tab', { name: /Профиль/ })).toHaveAttribute('aria-selected', 'true');
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
  await expect(page.getByRole('list', { name: 'Просмотренное' })).toBeVisible();
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
  await expect(page.getByRole('tabpanel').getByRole('note').filter({ hasText: 'Скрыто владельцем' })).toBeVisible();
  await expect(page.getByRole('radiogroup', { name: 'Кто видит этот раздел' })).toHaveCount(0);
  await tabs.getByRole('tab', { name: /Отзывы/ }).click();
  await expect(page.getByRole('tabpanel').getByRole('note').filter({ hasText: 'Скрыто владельцем' })).toBeVisible();
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
