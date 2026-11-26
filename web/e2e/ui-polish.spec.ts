import {expect,test} from './util-test';
import {env,url,expectClean} from './util';
import {mkdirSync} from 'node:fs';
import {resolve} from 'node:path';

test('подсказки режиссёра появляются с первой буквы',async({page})=>{
  await page.goto(url('/profile'));
  const input=page.getByRole('combobox',{name:'Режиссёр'});
  await input.fill('К');
  const list=page.getByRole('listbox',{name:'Варианты режиссёров'});
  await expect(list.getByRole('option',{name:'Кристофер Нолан'})).toBeVisible();
  await list.getByRole('option',{name:'Кристофер Нолан'}).click();
  await expect(input).toHaveValue('Кристофер Нолан');
  await expect(list).toHaveCount(0);
});

test('занятые логин и имя объясняются рядом с полями',async({page})=>{
  const me=await(await page.request.get(url('/api/me'))).json();
  await page.context().clearCookies();
  await page.goto(url('/register'));
  await page.getByLabel('Как вас зовут').fill(me.user.name);
  await page.getByLabel('Логин (латиница, цифры)').fill(me.user.login);
  await expect(page.getByRole('alert').filter({hasText:'Этот логин уже занят'})).toBeVisible();
  await expect(page.getByRole('alert').filter({hasText:'Это имя уже используется'})).toBeVisible();
});

test('единая нижняя панель находится у края и меню профиля доступно',async({page},info)=>{
  test.skip(info.project.name==='desktop','На ПК навигация находится в шапке');
  let shape='';
  for(const path of [`/people/${env().people.me}`,'/feed','/discover']){
    await page.goto(url(path));
    const bar=page.getByRole('navigation',{name:'Разделы',exact:true});await expect(bar).toBeVisible();
    const metrics=await bar.evaluate(el=>{const r=el.getBoundingClientRect(),s=getComputedStyle(el);return{bottom:r.bottom,height:innerHeight,shape:`${s.borderRadius}|${s.backgroundColor}`};});
    expect(Math.abs(metrics.bottom-metrics.height)).toBeLessThan(2);
    if(shape)expect(metrics.shape).toBe(shape);shape=metrics.shape;
  }
  await page.goto(url(`/people/${env().people.me}`));
  // Профиль: быстрые разделы всегда на виду (без свёрнутого меню).
  const quick=page.getByRole('navigation',{name:'Быстрые разделы'});
  await quick.scrollIntoViewIfNeeded();await expect(quick.getByRole('link',{name:/Чаты/})).toBeVisible();
  const shots=resolve('../docs/design/ui-polish');mkdirSync(shots,{recursive:true});await page.screenshot({path:resolve(shots,`navigation-${info.project.name}.png`)});
});

test('постер загружается с устройства без ожидания IMDb',async({page},info)=>{
  // Картинки IMDb подгружаются сразу при открытии (21.11), но своя загрузка их не ждёт: ответ IMDb держим, пока
  // свой постер не загрузится.
  let release=()=>{};const held=new Promise<void>(r=>(release=r));
  await page.route('**/imdb-art',async route=>{await held;await route.fulfill({json:{items:[]}}).catch(()=>{});});
  const library=await(await page.request.get(url('/api/library'))).json();
  const image=library.items.find((c:{poster:string})=>c.poster)?.poster;
  expect(image).toBeTruthy();const bytes=await(await page.request.get(url(image))).body();
  await page.goto(url('/ext/tt1196946'));
  const mobile=page.getByRole('button',{name:'Ещё действия'});
  if((page.viewportSize()?.width ?? 1440)<=640){
    await mobile.click();await page.getByRole('dialog').getByRole('button',{name:'Постер и фон',exact:true}).click();
  }else{
    await page.getByRole('button',{name:'Постер и фон',exact:true}).filter({visible:true}).first().click();
  }
  const dialog=page.getByRole('dialog',{name:'Постер и фон'});await expect(dialog).toBeVisible();
  const uploaded=page.waitForResponse(r=>r.request().method()==='PUT'&&r.url().endsWith('/image/poster'));
  await dialog.getByLabel('Свой постер',{exact:true}).setInputFiles({name:'my-poster.jpg',mimeType:'image/jpeg',buffer:bytes});
  expect((await uploaded).ok()).toBe(true);await expect(dialog).toBeHidden();release();
  await expectClean(page,'свой постер');
  const shots=resolve('../docs/design/ui-polish');mkdirSync(shots,{recursive:true});await page.screenshot({path:resolve(shots,`title-${info.project.name}.png`)});
});

test('плеер: скорость в настройках, без галочек, реальное усиление до 200%',async({page,browserName})=>{
  test.skip(browserName==='webkit','Playwright WebKit на Windows без медиадвижка');
  await page.addInitScript(()=>{
    const Native=window.AudioContext;
    window.AudioContext=class extends Native{override createGain(){const node=super.createGain();(window as Window & {testGain?:GainNode}).testGain=node;return node;}};
  });
  const detail=await(await page.request.get(url(`/api/titles/${env().ids['Тьма']}`))).json();
  await page.goto(url(`/watch/${detail.files[0].id}`));
  await expect.poll(()=>page.locator('video').evaluate(v=>(v as HTMLVideoElement).readyState)).toBeGreaterThanOrEqual(2);
  if(await page.locator('video').evaluate(v=>(v as HTMLVideoElement).paused))await page.getByRole('button',{name:'Смотреть',exact:true}).click();
  const controls=page.getByRole('group',{name:'Управление просмотром'});
  await expect(controls.getByRole('button',{name:'Скорость',exact:true})).toHaveCount(0);
  await expect(controls.getByRole('button',{name:'Горячие клавиши'})).toHaveCount(0);
  await controls.getByRole('button',{name:'Настройки плеера'}).click();
  await page.getByRole('menuitem',{name:/Скорость/}).click();
  await expect(page.getByRole('menu').locator('button svg')).toHaveCount(0);
  await page.getByRole('menuitemradio',{name:'Обычная',exact:true}).click();
  await controls.getByRole('button',{name:'Настройки плеера'}).click();
  const slider=page.getByRole('slider',{name:'Громкость до 200%'});
  await slider.evaluate(el=>{(el as HTMLInputElement).value='2';el.dispatchEvent(new Event('input',{bubbles:true}));});
  await expect.poll(()=>page.evaluate(()=>(window as Window & {testGain?:GainNode}).testGain?.gain.value)).toBe(2);
  expect(await page.locator('video').evaluate(v=>(v as HTMLVideoElement).volume)).toBe(1);
});

test('медленный замер сети не задерживает запрос самого видео',async({page},info)=>{
  test.skip(info.project.name!=='desktop','Одна проверка порядка запросов на ПК');
  await page.addInitScript(()=>localStorage.removeItem('nova-speed'));
  let probeFinished=false,mediaRequested=false;
  page.on('requestfailed',r=>{if(r.url().includes('/api/speedtest'))probeFinished=true;});
  page.on('request',r=>{if(/\/api\/files\/\d+\/(raw|hls|copy|play)(\/|\?|$)/.test(r.url()))mediaRequested=true;});
  await page.route('**/api/speedtest?*',async route=>{await new Promise(r=>setTimeout(r,4000));probeFinished=true;await route.fulfill({status:200,body:Buffer.alloc(1024)}).catch(()=>{});});
  const detail=await(await page.request.get(url(`/api/titles/${env().ids['Тьма']}`))).json();
  await page.goto(url(`/watch/${detail.files[0].id}`));
  await expect.poll(()=>mediaRequested).toBe(true);
  expect(probeFinished).toBe(false);
});
