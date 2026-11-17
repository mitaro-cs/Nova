import {expect,test} from './util-test';
import {url} from './util';

test('выбранное администратором видео открывается первым',async({page})=>{
 const id='tt7654325';const card={id,kind:'movie',title:'Тестовый фильм',original:null,year:2020,rating:8,poster:null,genres:[],local:null,mark:null,wished:false};
 const link={id:42,kind:'direct',url:'https://example.com/video.mp4',embed:null,label:'Источник',thumb:null,duration:500,titleId:null,extId:id,addedBy:null,author:'',addedAt:0,season:null,episode:null,progress:null,preferred:true};
 const detail={card,overview:'',runtime:100,votes:0,backdrop:null,posterLarge:null,tagline:null,age:null,releaseDate:null,countries:[],languages:[],aka:[],people:[],gallery:[],similar:[],localCard:null,lists:[],seasons:[],friends:[],links:[link]};
 await page.route(`**/api/discover/title/${id}**`,route=>route.fulfill({json:new URL(route.request().url()).pathname.endsWith('/reviews')?{average:null,votes:0,total:0,items:[],canWrite:false}:detail}));
 await page.route('**/api/franchise?*',route=>route.fulfill({json:[]}));
 await page.goto(url(`/ext/${id}`));
 await expect(page.getByRole('link',{name:/Смотреть/}).filter({visible:true}).first()).toHaveAttribute('href','/watch/link/42');
});

test('приоритетный источник открывается и у локального фильма',async({page})=>{
 const {env}=await import('./util'); const titleId=env().ids['Ла-Ла Ленд'];
 const link={id:421,kind:'direct',url:'https://example.com/video.mp4',embed:null,label:'Источник',thumb:null,duration:500,titleId,extId:null,addedBy:null,author:'',addedAt:0,season:null,episode:null,progress:null,preferred:true};
 await page.route(`**/api/links?titleId=${titleId}`,route=>route.fulfill({json:[link]}));
 await page.goto(url(`/title/${titleId}`));
 await expect(page.getByRole('link',{name:/Смотреть/}).filter({visible:true}).first()).toHaveAttribute('href','/watch/link/421');
});
