import {test,expect} from './util-test';
import {url,expectClean,choose} from './util';

test('источники добавляются и удаляются вручную без импорта',async({page},info)=>{
  const headers={'X-Fsfy':'1'};
  const previous=await(await page.request.get(url('/api/admin/catalogs'))).json();
  try{
    expect((await page.request.put(url('/api/admin/catalogs'),{headers,data:{items:[]}})).ok()).toBe(true);
    await page.goto(url('/admin?tab=sources'));
    const panel=page.getByRole('region',{name:'Источники видео'});
    await expect(panel.getByText('Источники пока не добавлены.')).toBeVisible();
    await expect(page.getByText('Источники из вашего списка')).toHaveCount(0);
    await expect(page.getByText('Продолжить импорт')).toHaveCount(0);
    for(const [address,kind] of [['https://old.yummyani.me/','Аниме'],['https://mg.lordfilm.md/filmy/','Фильмы'],['https://lordserials.fan/','Сериалы']]){
      await panel.getByLabel('Адрес сайта').fill(address);
      await choose(page,'Раздел',kind,panel);
      await panel.getByRole('button',{name:'Добавить',exact:true}).click();
      await expect(panel.getByRole('status')).toHaveText('Сайт добавлен');
    }
    await expect(panel.locator('li')).toHaveCount(3);
    await panel.getByRole('button',{name:'Удалить mg.lordfilm.md'}).click();
    await expect(panel.locator('li')).toHaveCount(2);
    const saved=await(await page.request.get(url('/api/admin/catalogs'))).json();
    expect(saved.items.map((x:{domain:string})=>x.domain)).toEqual(['old.yummyani.me','lordserials.fan']);
    if(info.project.name!=='desktop'){
      const bar=page.getByRole('navigation',{name:'Разделы',exact:true});
      const metrics=await bar.evaluate(el=>({bottom:el.getBoundingClientRect().bottom,height:innerHeight,color:getComputedStyle(el).backgroundColor,radius:getComputedStyle(el).borderRadius}));
      expect(Math.abs(metrics.bottom-metrics.height)).toBeLessThan(2);expect(metrics.color).toBe('rgb(0, 0, 0)');expect(metrics.radius).toBe('0px');
    }
    await expectClean(page,'ручные источники');
  }finally{expect((await page.request.put(url('/api/admin/catalogs'),{headers,data:previous})).ok()).toBe(true);}
});
