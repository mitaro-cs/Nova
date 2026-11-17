import {expect,test} from './util-test';
import {url} from './util';

test('при ошибке фона показывается постер, а при ошибке постера — название', async ({page}, info) => {
  test.skip(info.project.name !== 'desktop', 'Широкая карточка фильма');
  const id='tt7654325';
  const card={id,kind:'movie',title:'Тестовый фильм',original:null,year:2020,rating:null,poster:'/broken-poster.jpg',genres:[],local:null,mark:null,wished:false};
  const detail={card,overview:'',runtime:null,votes:0,backdrop:'/broken-background.jpg',posterLarge:'/broken-poster.jpg',tagline:null,age:null,releaseDate:null,countries:[],languages:[],aka:[],people:[],gallery:[],similar:[],localCard:null,lists:[],seasons:[],friends:[],links:[]};
  await page.route(`**/api/discover/title/${id}**`,r=>r.fulfill({json:new URL(r.request().url()).pathname.endsWith('/reviews')?{average:null,votes:0,total:0,items:[],canWrite:false}:detail}));
  await page.route('**/broken-background.jpg',r=>r.abort());
  await page.route('**/broken-poster.jpg',r=>r.abort());
  await page.goto(url(`/ext/${id}`));
  await expect(page.locator('.rwh .poster span')).toHaveText('Тестовый фильм');
  await expect(page.locator('.rwh .shot .ph')).toBeVisible();
});
