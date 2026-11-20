<script lang="ts">
  import { api, post, errorText } from '$lib/api';
  import { extSheet } from '$lib/discover/sheet.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import AccessControlLab from './AccessControlLab.svelte';
  let {onchanged}: {onchanged?:()=>void} = $props();
  type Source={source:{id:string;name:string;url:string;kind:string};domain:string;configured:boolean;enabled:boolean;running:boolean;state:string;pages:number;message:string;verificationAvailable:boolean;sessionAllowed:boolean;counts:{found:number;imported:number;failed:number;pending:number};problems:{url:string;title:string;state:string;message:string}[]};
  let items=$state<Source[]>([]),error=$state(''),busy=$state('');
  const connected=$derived(items.filter(s=>s.configured&&s.enabled).length);
  const imported=$derived(items.reduce((n,s)=>n+s.counts.imported,0));
  const running=$derived(items.filter(s=>s.running).length);
  const kindLabel=(kind:string)=>kind==='movie'?'Фильмы':kind==='anime'?'Аниме':'Сериалы';
  const states:Record<string,string>={new:'Не импортирован',running:'Импортируем',paused:'Приостановлен',blocked:'Источник недоступен',partial:'Есть ошибки или непроверенные потоки',complete:'Обход завершён'};
  $effect(()=>{
    let live=true,previous=-1;
    const load=()=>api<{items:Source[]}>('/api/admin/site-sources').then(r=>{
      if(!live)return;items=r.items;const imported=items.reduce((n,s)=>n+s.counts.imported,0);
      if(previous>=0&&previous!==imported)extSheet.version++;previous=imported;
    },e=>{if(live)error=errorText(e)});
    void load();const timer=setInterval(()=>{if(items.some(s=>s.running))void load()},3000);
    return()=>{live=false;clearInterval(timer)};
  });
  async function reload(){items=(await api<{items:Source[]}>('/api/admin/site-sources')).items;}
  async function action(id:string,command:string){
    busy=id;error='';
    try{await post(`/api/admin/site-sources/${id}/${command}`,{});await reload();onchanged?.();}
    catch(e){error=errorText(e)}finally{busy=''}
  }
  async function assign(e:SubmitEvent,s:Source,url:string){
    e.preventDefault();const id=String(new FormData(e.currentTarget as HTMLFormElement).get('imdb')??'').trim();busy=s.source.id;error='';
    try{await post(`/api/admin/site-sources/${s.source.id}/match`,{url,imdbId:id});await post(`/api/admin/site-sources/${s.source.id}/import`,{});await reload();}
    catch(e){error=errorText(e)}finally{busy=''}
  }
  async function verify(id:string,command:string){busy=id;error='';try{await post(`/api/admin/site-sources/${id}/verify`,{action:command});await reload();}catch(e){error=errorText(e)}finally{busy=''}}
</script>

<section aria-label="Источники из вашего списка" class="presets">
  <div class="intro">
    <div><span class="eyebrow">ВАША МЕДИАТЕКА · ВНЕШНИЕ ИСТОЧНИКИ</span><h4>Больше историй.<br />В одном кинотеатре.</h4><p class="muted small">Подключите источник и запустите импорт. Фильмы, сезоны и озвучки появятся в каталоге Nova.</p></div>
    <div class="overview" aria-label="Сводка источников"><div><b>{connected}<small> / {items.length}</small></b><span>подключено</span></div><div><b>{imported}</b><span>импортировано</span></div><div><b>{running}</b><span>в работе</span></div></div>
  </div>
  <div class="section-title"><h5>Источники из вашего списка</h5><span class="muted small">{items.length} источников</span></div>
  <div class="sources">
    {#each items as s (s.source.id)}
      <article aria-label={s.source.name} class:active={s.configured&&s.enabled}>
        <div class="identity"><span class="source-icon"><Icon name={s.source.kind==='movie'?'film':s.source.kind==='anime'?'sparkle':'tv'} size={23} stroke={1.6} /></span><div><span class="category">{kindLabel(s.source.kind)}</span><b>{s.source.name}</b></div></div>
        <a href={s.source.url} target="_blank" rel="noopener noreferrer" class="muted small">{s.domain}</a>
        <span class="source-state" class:working={s.running} class:attention={s.state==='blocked'||s.state==='partial'} role="status"><i></i>{!s.enabled&&s.configured?'Выключен':states[s.state]??s.state}</span>
        <div class="numbers"><div><b>{s.counts.found}</b><span>найдено</span></div><div><b>{s.counts.imported}</b><span>в каталоге</span></div></div>
        {#if s.counts.failed||s.pages}<p class="muted small">Страниц: {s.pages}{#if s.counts.failed} · Требуют внимания: {s.counts.failed}{/if}</p>{/if}
        <div class="actions">
          {#if !s.configured || !s.enabled}
            <button class="btn sm primary" disabled={!!busy} onclick={()=>action(s.source.id,'connect')}><Icon name="plus" size={15} />{s.configured?'Включить':'Подключить'}</button>
          {:else if s.running}
            <button class="btn sm" disabled={!!busy} onclick={()=>action(s.source.id,'stop')}><Icon name="pause" size={14} />Приостановить</button>
          {:else}
            <button class="btn sm" disabled={!!busy} onclick={()=>action(s.source.id,'import')}><Icon name="download" size={15} />{s.state==='paused'||s.state==='blocked'?'Продолжить импорт':'Импортировать сайт'}</button>
          {/if}
          {#if busy===s.source.id}<span class="muted small">Применяем…</span>{/if}
        </div>
        {#if s.message}<p class="muted small" role="status">{s.message}</p>{/if}
        {#if s.verificationAvailable}
          <details><summary>Проверка человека и доступ к сайту</summary>
            <p class="muted small">Откройте отдельное окно и пройдите проверку вручную. Затем разрешите Nova использовать cookies только этого сайта до закрытия приложения. Они не передаются зрителям и не экспортируются в настройки.</p>
            <div class="actions"><button class="btn sm" disabled={!!busy} onclick={()=>verify(s.source.id,'open')}>Открыть проверку</button><button class="btn sm" disabled={!!busy} onclick={()=>verify(s.source.id,'use')}>Использовать сессию</button>{#if s.sessionAllowed}<button class="btn sm ghost" disabled={!!busy} onclick={()=>verify(s.source.id,'forget')}>Забыть сессию</button>{/if}</div>
          </details>
        {/if}
        {#if s.problems.length}
          <details><summary>Причины и тайтлы ({s.problems.length})</summary>
            {#each s.problems as p (p.url)}
              <div class="problem"><b>{p.title}</b><p class="muted small">{p.message}</p><a class="small" href={p.url} target="_blank" rel="noopener noreferrer">Страница источника</a>
                {#if p.state==='unmatched'}<form onsubmit={e=>assign(e,s,p.url)} class="actions"><input class="input" name="imdb" aria-label="IMDb для {p.title}" placeholder="tt1234567" required pattern={'tt[0-9]{7,10}'} /><button class="btn sm" disabled={!!busy}>Назначить тайтл</button></form>{/if}
              </div>
            {/each}
          </details>
        {/if}
      </article>
    {/each}
  </div>
  <p class="footnote"><Icon name="info" size={16} />Импорт работает, пока Nova открыта. Если сайт недоступен или требует проверку человека, причина появится в его карточке. Доступность всех потоков зависит от источника.</p>
  <AccessControlLab />
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  h4,h5,p{margin:0}.presets{display:grid;gap:20px}.intro{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:28px;align-items:center;padding:30px;border:1px solid var(--line);border-radius:24px;background:linear-gradient(125deg,var(--surface-2),var(--surface))}
  .eyebrow{font-size:10px;font-weight:600;letter-spacing:.14em;color:var(--muted)}h4{font-size:clamp(26px,3vw,38px);line-height:1.08;letter-spacing:-.04em;margin:12px 0} .intro p{max-width:390px;line-height:1.6}
  .overview{display:flex;gap:24px}.overview div,.numbers div{display:grid;gap:5px}.overview b{font-size:30px;line-height:1;font-variant-numeric:tabular-nums;letter-spacing:-.04em}.overview b small{font-size:15px;color:var(--muted);font-weight:400;letter-spacing:0}.overview span,.numbers span{font-size:11px;color:var(--muted)}
  .section-title{display:flex;justify-content:space-between;gap:12px;align-items:center}h5{font-size:15px;font-weight:600}
  .sources{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,320px),1fr));gap:12px}
  article{display:flex;flex-direction:column;align-items:flex-start;gap:14px;background:var(--surface-2);border:1px solid var(--line);padding:22px;border-radius:20px;min-width:0;overflow-wrap:anywhere}article.active{border-color:color-mix(in srgb,var(--text) 25%,transparent)}
  .identity{display:flex;align-items:center;gap:12px;width:100%}.identity>div{display:grid;gap:4px;min-width:0}.identity b{font-size:15px;line-height:1.3}.source-icon{display:grid;place-items:center;width:46px;height:46px;flex:none;background:var(--surface);border:1px solid var(--line);border-radius:14px}.category{font-size:10px;color:var(--muted);text-transform:uppercase;letter-spacing:.1em}
  .source-state{display:flex;align-items:center;gap:7px;font-size:11px;color:var(--muted);min-height:28px}.source-state i{width:6px;height:6px;border-radius:50%;background:currentColor;flex:none;opacity:.55}.source-state.working{color:var(--text)}.source-state.attention{color:var(--error,#e49aab)}
  .numbers{display:grid;grid-template-columns:1fr 1fr;width:100%;padding:14px 0;border-top:1px solid var(--line);gap:12px}.numbers b{font-size:24px;line-height:1;font-variant-numeric:tabular-nums;letter-spacing:-.03em}
  .actions{display:flex;align-items:center;flex-wrap:wrap;gap:8px}article>.actions{margin-top:auto;padding-top:4px}
  .problem{display:grid;gap:8px;padding:12px 0;border-top:1px solid var(--line)}.problem input{flex:1 1 140px;min-width:0}
  details{min-width:0;width:100%;font-size:12px}summary{cursor:pointer;padding:6px 0}a{overflow-wrap:anywhere;font-size:11px}.footnote{display:flex;align-items:flex-start;gap:10px;font-size:12px;line-height:1.6;color:var(--muted);max-width:740px}
  @media(max-width:760px){.intro{grid-template-columns:1fr;padding:22px;gap:24px}.overview{justify-content:space-between;gap:16px;border-top:1px solid var(--line);padding-top:20px}.overview b{font-size:26px}.sources{grid-template-columns:repeat(auto-fit,minmax(min(100%,280px),1fr))}article{padding:20px}.section-title>span{display:none}}
</style>
