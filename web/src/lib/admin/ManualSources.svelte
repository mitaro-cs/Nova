<script lang="ts">
  import { api, errorText, put } from '$lib/api';
  import { session } from '$lib/session.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';
  type Catalog = { domain:string; name:string; kind:string; manifest:string|null; search:string|null; enabled:boolean };
  let items=$state<Catalog[]|null>(null), address=$state(''), kind=$state('all'), error=$state(''), busy=$state(false), notice=$state('');
  const host=$derived(session.me?.user.role==='host');
  $effect(()=>{let live=true;api<{items:Catalog[]}>('/api/admin/catalogs').then(r=>{if(live)items=r.items},e=>{if(live)error=errorText(e)});return()=>{live=false};});
  async function save(next:Catalog[],message:string){
    if(busy)return;busy=true;error='';notice='';
    try{const result=await put<{items:Catalog[]}>('/api/admin/catalogs',{items:next});items=result.items;notice=message;}
    catch(e){error=errorText(e)}finally{busy=false}
  }
  async function add(e:SubmitEvent){
    e.preventDefault();if(!items||busy)return;
    let parsed:URL;try{parsed=new URL(address.trim().includes('://')?address.trim():`https://${address.trim()}`);}catch{error='Введите адрес сайта, например https://old.yummyani.me/';return;}
    const domain=parsed.hostname.replace(/^www\./,'').toLowerCase();
    if(!['http:','https:'].includes(parsed.protocol)||!domain.includes('.')){error='Введите корректный адрес сайта';return;}
    if(items.some(i=>i.domain===domain)){error='Этот сайт уже добавлен';return;}
    const search=domain==='old.yummyani.me'?`https://${domain}/search?word={q}`:`https://${domain}/index.php?do=search&subaction=search&story={q}`;
    await save([...items,{domain,name:domain,kind,manifest:null,search,enabled:true}],'Сайт добавлен');
    if(!error)address='';
  }
  function move(i:number,d:number){if(!items)return;const n=i+d;if(n<0||n>=items.length)return;const next=[...items];[next[i],next[n]]=[next[n],next[i]];void save(next,'Порядок сохранён');}
</script>

<section class="card pane" aria-labelledby="manual-sources">
  <h3 id="manual-sources">Источники видео</h3>
  <p class="muted">Добавьте сайты вручную. Nova ищет видео при просмотре; источники выше в списке имеют приоритет.</p>
  {#if host}
    <form class="add-source" onsubmit={add}>
      <label class="field"><span>Адрес сайта</span><input class="input" type="text" bind:value={address} placeholder="https://old.yummyani.me/" required /></label>
      <div class="field"><span>Раздел</span><Select label="Раздел" value={kind} onchange={(value)=>kind=value} options={[{value:'all',label:'Всё'},{value:'movie',label:'Фильмы'},{value:'series',label:'Сериалы'},{value:'anime',label:'Аниме'}]} /></div>
      <button class="btn primary" disabled={busy || !address.trim()}><Icon name="plus" size={15} /> Добавить</button>
    </form>
    <p class="muted hint">Например: old.yummyani.me, mg.lordfilm.md, lordserials.fan</p>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  {#if items === null}<p class="muted">Загружаем источники…</p>
  {:else if !items.length}<p class="muted">Источники пока не добавлены.</p>
  {:else}<ul class="source-list">{#each items as item,i (item.domain)}<li><div class="identity"><Icon name="globe" size={18}/><span><b>{item.name || item.domain}</b><small>{item.domain} · {item.kind === 'movie'?'фильмы':item.kind === 'series'?'сериалы':item.kind === 'anime'?'аниме':'все разделы'}</small></span></div>{#if host}<div class="actions"><button class="btn sm" type="button" disabled={busy||i===0} aria-label="Поднять {item.domain}" onclick={()=>move(i,-1)}>↑</button><button class="btn sm" type="button" disabled={busy||i===items.length-1} aria-label="Опустить {item.domain}" onclick={()=>move(i,1)}>↓</button><button class="btn sm" type="button" disabled={busy} onclick={()=>save(items!.map(x=>x.domain===item.domain?{...x,enabled:!x.enabled}:x),item.enabled?'Источник выключен':'Источник включён')}>{item.enabled?'Вкл.':'Выкл.'}</button><button class="btn sm" type="button" disabled={busy} aria-label="Удалить {item.domain}" onclick={()=>save(items!.filter(x=>x.domain!==item.domain),'Источник удалён')}>Удалить</button></div>{/if}</li>{/each}</ul>{/if}
</section>
<style>
  .pane{display:grid;gap:14px}.pane h3,.pane p{margin:0}.add-source{display:grid;grid-template-columns:minmax(0,1fr) minmax(130px,180px) auto;align-items:end;gap:10px}.field{display:grid;gap:7px;font-size:12px}.field .input{width:100%;min-height:44px}.add-source>.btn{min-height:44px}.hint{font-size:12px}.source-list{list-style:none;padding:0;margin:0;display:grid}.source-list li{display:flex;justify-content:space-between;align-items:center;gap:12px;padding:14px 0;border-top:1px solid var(--line)}.identity{display:flex;gap:10px;align-items:center;min-width:0}.identity span{display:grid;gap:3px;overflow-wrap:anywhere}.identity small{color:var(--muted);font-size:12px}.actions{display:flex;gap:6px;flex-wrap:wrap}.error{color:var(--error,#f99)}@media(max-width:700px){.add-source{grid-template-columns:1fr 1fr}.add-source .field:first-child{grid-column:1 / -1}.source-list li{align-items:flex-start;flex-direction:column}}
</style>
