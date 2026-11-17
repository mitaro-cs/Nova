<script lang="ts">
  import { replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api';
  import Poster from '$lib/discover/Poster.svelte';
  import type { Card, ExtCard } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';
  import Sheet from '$lib/ui/Sheet.svelte';
  import Wide from '$lib/ui/Wide.svelte';
  import { untrack } from 'svelte';
  import { catalog, loadCatalog } from '$lib/catalog.svelte';
  import { matches } from '$lib/library';
  import { isAnime } from '$lib/library';
  import { genreRu } from '$lib/genres';
  import Loading from '$lib/ui/Loading.svelte';
  import { personHref } from '$lib/people';

  // Поиск: своё (сразу) и IMDb - всё остальное (страница фильма - /ext). Поле - своё состояние:
  // раньше оно бралось из адреса и при быстром наборе теряло буквы (адрес отставал от ввода).
  let text = $state(untrack(() => page.url.searchParams.get('q') ?? ''));
  let items = $state<Card[] | null>(null);
  let found = $state<ExtCard[] | null>(null);
  // Запрос - жанр («фантастика»): сервер присылает его в заголовке, подпись ряда - по нему.
  let genreName = $state<string | null>(null);
  let searching = $state(false);
  const query = $derived(text.trim());
  let kind = $state<'all' | 'movie' | 'series' | 'anime' | 'people'>('all');
  const scope = $derived(kind === 'people' ? 'people' : 'titles');
  let persons = $state<{id:string; name:string; description?:string; role:string}[]>([]);
  let year = $state('');
  let rating = $state('');
  let genre = $state('');
  let sort = $state('relevance');
  let failure = $state('');
  function reset() { year = ''; rating = ''; genre = ''; sort = 'relevance'; }
  const hue = (i: number) => [212, 28, 268, 165, 8, 192][i % 6];
  const cache = new Map<string, ExtCard[]>();
  const RATINGS = [{ value: '', label: 'Любой' }, ...[5, 6, 7, 8, 9].map((n) => ({ value: String(n), label: `${n} и выше` }))];
  const SORTS = [
    { value: 'relevance', label: 'По совпадению' },
    { value: 'rating', label: 'По рейтингу' },
    { value: 'year', label: 'Сначала новые' },
    { value: 'title', label: 'По названию' },
  ];
  const genres = $derived([...new Set([...(items ?? []).flatMap((c) => c.genres), ...(found ?? []).flatMap((c) => c.genres)].map(genreRu))].sort((a,b) => a.localeCompare(b,'ru')));
  function refine<T extends Card | ExtCard>(cards: T[]): T[] {
    const rows = cards.filter((c) => (kind === 'all' || (kind === 'anime' ? isAnime(c) : c.kind === kind)) && (!year || c.year === Number(year)) && (!rating || c.rating != null && c.rating >= Number(rating)) && (!genre || c.genres.some((g) => genreRu(g) === genre)));
    if (sort !== 'relevance') rows.sort((a,b) => sort === 'title' ? a.title.localeCompare(b.title,'ru') : sort === 'year' ? (b.year ?? 0)-(a.year ?? 0) : (b.rating ?? 0)-(a.rating ?? 0));
    return rows;
  }
  const localResults = $derived(refine(items ?? []));
  const externalResults = $derived(refine(found ?? []));
  $effect(() => { loadCatalog(); });

  // Пока поле пусто - «Советуем посмотреть» (подборки IMDb) и «Категории» (жанры). На телефоне (`compact`)
  // фильтры прячутся за значком справа в поле.
  const compact = typeof matchMedia !== 'undefined' && matchMedia('(max-width: 640px)').matches;
  let showFilters = $state(false);
  type Pick = { key: string; title: string; items: ExtCard[] };
  let picks = $state<Pick[]>([]);
  const CATEGORIES = ['Боевики', 'Комедии', 'Драмы', 'Триллеры', 'Фантастика', 'Ужасы', 'Мелодрамы', 'Детективы', 'Приключения', 'Фэнтези', 'Мультфильмы', 'Криминал', 'Документальные', 'Военные'];
  $effect(() => {
    let live = true;
    api<{ rows: Pick[] }>('/api/discover').then((f) => {
      if (!live) return;
      // У подборок часто один и тот же первый фильм: каждой берём первый постер, которого ещё не было.
      const used = new Set<string>();
      picks = f.rows.flatMap((r) => {
        const first = r.items.find((e) => e.poster && !used.has(e.poster));
        if (!first) return [];
        used.add(first.poster!);
        return [{ ...r, items: [first, ...r.items.filter((e) => e !== first)] }];
      }).slice(0, 8);
    }, () => {});
    return () => { live = false; };
  });

  $effect(() => {
    const q = query;
    const target = scope;
    persons = [];
    items = catalog.items?.filter((c) => matches(c, q)) ?? null;
    found = null;
    searching = false;
    failure = '';
    if (!q) return;
    const hit = cache.get(q.toLocaleLowerCase('ru'));
    if (hit && target !== 'people') { found = hit; return; }
    let live = true;
    const controller = new AbortController();
    let updateTimer = 0;
    const t = setTimeout(() => {
      try {
        replaceState(`/search?q=${encodeURIComponent(q)}`, {});
      } catch {
        // роутер ещё не готов - адрес не важен
      }
      if (!catalog.items) api<Card[]>(`/api/titles?sort=rating&q=${encodeURIComponent(q)}`, { signal: controller.signal }).then(
        (r) => live && (items = r),
        () => live && (items = []),
      );
      if (q.length < 2) return void (found = []);
      searching = true;
      if (target === 'people') {
        api<typeof persons>(`/api/discover/search?q=${encodeURIComponent(q)}&scope=people`, { signal: controller.signal }).then((r) => { if (live) persons = r; }, () => { if (live) failure = 'Не удалось загрузить людей кино'; }).finally(() => { if (live) { searching = false; found = []; } });
        return;
      }
      let refreshing=false;
      const load=()=>api<ExtCard[]>('/api/discover/search?q='+encodeURIComponent(q),{signal:controller.signal,headersReceived:(h)=>{refreshing=h.get('x-nova-refreshing')==='1';const g=h.get('x-nova-genre');genreName=g?decodeURIComponent(g.replace(/\+/g,' ')):null;}})
        .then((r)=>{if(!live)return;const incoming=r.filter((e)=>!e.local);const old=found??[];const map=new Map(incoming.map((e)=>[e.id,e]));const seen=new Set(old.map((e)=>e.id));found=[...old.map((e)=>map.get(e.id)??e),...incoming.filter((e)=>!seen.has(e.id))];if(cache.size>=24)cache.delete(cache.keys().next().value!);if(!refreshing)cache.set(q.toLocaleLowerCase('ru'),found);if(refreshing)updateTimer=window.setTimeout(load,2500);},()=>{if(live){found=found??[];failure='Не удалось связаться с каталогом. Попробуйте ещё раз.';}})
        .finally(()=>{if(live)searching=false;});
      void load();
    }, 250);
    return () => {
      live = false;
      controller.abort();
      clearTimeout(updateTimer);
      clearTimeout(t);
    };
  });
</script>

<div class="search-root">
<Sheet title="Что посмотрим?" subtitle="Ваша коллекция и мир кино в одном поиске.">
  <div class="rw-block panel">
  <form class="big" role="search" onsubmit={(e) => e.preventDefault()}>
    <Icon name="search" size={22} />
    <!-- svelte-ignore a11y_autofocus -->
    <input type="search" placeholder={compact ? 'Фильмы, персоны' : 'Название фильма или сериала'} bind:value={text} autofocus={!compact} aria-label="Поиск" />
    <button type="button" class="tune" aria-label="Фильтры" aria-expanded={showFilters} onclick={() => (showFilters = !showFilters)}><Icon name="sliders" size={22} /></button>
  </form>
  <nav class="filters" class:open={showFilters} aria-label="Тип результатов">
    {#each [{ id: 'all', label: 'Всё' }, { id: 'movie', label: 'Фильмы' }, { id: 'series', label: 'Сериалы' }, { id: 'anime', label: 'Аниме' }, { id: 'people', label: 'Люди кино' }] as option (option.id)}
      <button class="chip" class:on={kind === option.id} onclick={() => kind = option.id as typeof kind}><Icon name={option.id === 'all' ? 'grid' : option.id === 'movie' ? 'film' : option.id === 'series' ? 'tv' : option.id === 'people' ? 'users' : 'star'} size={14} />{option.label}</button>
    {/each}
  </nav>
  <div class="refine" class:open={showFilters} aria-label="Фильтры поиска">
    <label>Год<input type="text" inputmode="numeric" maxlength="4" placeholder="Любой" bind:value={year} /></label>
    <label>Рейтинг<Select label="Рейтинг" variant="inline" bind:value={rating} options={RATINGS} /></label>
    <label>Жанр<Select label="Жанр" variant="inline" bind:value={genre} options={[{ value: '', label: 'Все жанры' }, ...genres.map((g) => ({ value: g, label: g }))]} /></label>
    <label>Порядок<Select label="Порядок" variant="inline" bind:value={sort} options={SORTS} /></label>
    <button type="button" class="chip reset" onclick={reset}>Сбросить</button>
  </div>
  </div>
  {#if failure}<p class="rw-block" role="status">{failure}</p>{/if}
  {#if searching}<div class="rw-block"><Loading compact label="Ищем в русских и международных каталогах" /></div>{/if}
  {#if !query}
    {#if picks.length}
      <section class="suggest" aria-label="Советуем посмотреть">
        <div class="s-head"><h2>Советуем посмотреть</h2><a href="/discover">Все</a></div>
        <div class="tiles">
          {#each picks.slice(0, 3) as r, i (r.key)}
            <a class="tile" href="/discover?chart={r.key}" style:background="linear-gradient(160deg, hsl({hue(i)} 36% 26%), hsl({hue(i) + 18} 26% 8%))"><span class="tag">Подборка IMDb</span><span class="cap">{r.title}</span></a>
          {/each}
        </div>
      </section>
    {/if}
    <section class="suggest" aria-label="Категории">
      <div class="s-head"><h2>Категории</h2></div>
      <div class="cats">{#each CATEGORIES as g (g)}<button type="button" onclick={() => (text = g)}>{g}</button>{/each}</div>
    </section>
  {/if}
  {#if query}
    {#if kind === 'people'}
      <div class="rw-block"><div class="persons">{#each persons as p (p.id)}<a href={personHref({name:p.name,job:p.role,id:p.id})}><Icon name="user" size={28} /><div><b>{p.name}</b><p class="muted">{p.description ?? 'Фильмография и биография'}</p></div><Icon name="right" size={16} /></a>{/each}</div></div>
      {#if !searching && !failure && !persons.length}<div class="empty">Никого не нашли. Попробуйте полное имя.</div>{/if}
    {:else}
    {#if localResults.length}
      <section class="rw-block" aria-label="На Nova">
        <h2 class="rw-h2">На Nova</h2>
        <div class="grid">
          {#each localResults as card, i (card.id)}<Wide {card} {i} />{/each}
        </div>
      </section>
    {/if}
    {#if externalResults.length}
      <section class="rw-block" aria-label="Все фильмы и сериалы">
        <h2 class="rw-h2">{genreName ? `Жанр: ${genreName} - самое известное` : 'Все фильмы и сериалы'}</h2>
        <div class="posters">{#each externalResults as card, i (card.id)}<Poster {card} {i} />{/each}</div>
      </section>
    {:else if items === null || found === null || searching}
      <div class="rw-block"><div class="posters">{#each Array(6) as _, i (i)}<span class="ph skeleton"></span>{/each}</div></div>
    {:else if !localResults.length}
      <div class="rw-block"><div class="empty"><Icon name="search" size={40} /> По запросу «{query}» ничего нет</div></div>
    {/if}
    {/if}
  {/if}
</Sheet>
</div>

<style>
  .persons { display: grid; grid-template-columns: repeat(auto-fit,minmax(min(100%,300px),1fr)); gap: 16px; }
  .persons a { display: flex; gap: 16px; align-items: center; border: 1px solid var(--line); border-radius: 18px; padding: 20px; background: var(--surface-2); }
  .persons a div { min-width: 0; flex: 1; }.persons p { margin: 8px 0 0; font-size: 12px; }
  .filters { display: flex; gap: 8px; margin-bottom: 20px; }
  .filters { flex-wrap: wrap; }
  .filters button { gap: 7px; }
  .refine { display: flex; flex-wrap: wrap; gap: 12px; }
  .refine label { display: flex; align-items: center; gap: 8px; padding: 8px 12px; border: 1px solid var(--line); border-radius: 12px; color: var(--muted); font-size: 12px; min-width: 0; }
  .refine input { background: var(--surface-3); color: #fff; border: 0; padding: 5px; font: inherit; max-width: 160px; width: 65px; }
  /* Телефон: подпись над списком, значение в плашке читается целиком. */
  @media(max-width:600px) {
    .refine { gap: 8px; }
    .refine label { flex: 1 1 40%; flex-direction: column; align-items: stretch; gap: 6px; }
    .refine label :global(.select.inline) { display: block; width: 100%; }
    .refine input { width: 100%; max-width: none; box-sizing: border-box; padding: 6px 10px; border-radius: 10px; height: 30px; }
  }
  .big {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 56px;
    padding: 0 18px;
    margin-bottom: 20px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    color: var(--muted);
  }
  .big:focus-within {
    border-color: var(--accent-2);
  }
  .big input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    font-size: 18px;
    color: var(--text);
  }
  .posters {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(var(--pw) - 8px), 1fr));
    gap: 20px 14px;
  }
  .ph {
    aspect-ratio: 2 / 3;
    border-radius: 12px;
  }
  @media (max-width: 600px) {
    .posters {
      grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
      gap: 16px 10px;
    }
  }
  .tune { display: none; }
  .suggest { display: block; margin-top: 26px; }
  .s-head { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 14px; }
  .s-head h2 { margin: 0; font-size: 20px; font-weight: 700; letter-spacing: -0.01em; }
  .s-head a { color: #fff; font-size: 16px; font-weight: 600; }
  .tiles { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
  .tile { display: flex; flex-direction: column; justify-content: flex-end; height: 230px; box-sizing: border-box; padding: 24px; border: 1px solid #ffffff1a; border-radius: 28px; color: #fff; transition: transform .45s var(--ease); }
  .tile:hover { transform: translateY(-4px); }
  .tag { font-size: 12px; font-weight: 600; letter-spacing: 0.1em; text-transform: uppercase; color: #e4e4e7; }
  .cap { display: -webkit-box; margin-top: 8px; overflow: hidden; font-size: clamp(20px, 2vw, 26px); font-weight: 700; letter-spacing: -0.02em; line-height: 1.12; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; }
  @media (max-width: 640px) {
    .tiles { display: flex; margin-inline: calc(-1 * var(--gutter)); padding-inline: var(--gutter); overflow-x: auto; scrollbar-width: none; }
    .tiles::-webkit-scrollbar { display: none; }
    .tile { flex: none; width: 250px; height: 170px; }
  }
  .cats { display: flex; flex-wrap: wrap; gap: 10px; }
  .cats button { height: 46px; padding: 0 20px; border: 1px solid #ffffff1a; border-radius: 23px; background: #141416; color: #fff; font: inherit; font-size: 16px; font-weight: 600; cursor: pointer; }
  /* Дизайн «База»: чёрная страница, поле и фильтры - одна панель #101012. */
  @media (min-width: 641px) {
    .search-root :global(.rw-head h1) { font-size: clamp(40px, 5vw, 64px); letter-spacing: -0.04em; line-height: 1.02; }
    .search-root :global(.rw-plate) { background: transparent; border: 0; }
    .panel { margin-bottom: 8px; padding: 20px; border: 1px solid #ffffff1a; border-radius: 32px; background: #101012; }
    .big { height: 64px; margin-bottom: 16px; background: #1c1c1f; border-color: #ffffff24; }
    .big input { font-size: 19px; }
    .refine label { border-radius: 999px; background: #141416; border-color: #ffffff24; padding: 8px 14px; }
    .filters { margin-bottom: 14px; }
  }
  .reset { align-self: center; }
  @media (min-width: 1001px) { .posters { grid-template-columns: repeat(6, minmax(0, 1fr)); } }
  /* Телефон: поле 44 px с радиусом 8 и значком фильтров, дальше - блоки подборок и категорий на чёрном;
     заголовка страницы нет. */
  @media (max-width: 640px) {
    .search-root :global(.rw-head) { display: none; }
    .search-root :global(.rw-page) { padding-top: calc(env(safe-area-inset-top) + 20px); }
    .search-root :global(.rw-plate) { background: #000; padding-block: 0; }
    .search-root :global(.rw-block) { border-bottom: 0; padding-block: 0 20px; }
    .big { height: 44px; padding: 0 6px 0 14px; margin-bottom: 20px; border: 0; border-radius: 8px; background: #2c2c2e; }
    .big input { font-size: 17px; }
    .tune { display: grid; place-items: center; width: 40px; height: 32px; padding: 0 0 0 8px; border: 0; border-left: 1px solid #ffffff2e; border-radius: 0; background: none; color: #c7c7cc; cursor: pointer; }
    .filters:not(.open), .refine:not(.open) { display: none; }
  }
</style>
