<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api';
  import Poster from '$lib/discover/Poster.svelte';
  import { extSheet } from '$lib/discover/sheet.svelte';
  import { plural } from '$lib/fmt';
  import NewList from '$lib/social/NewList.svelte';
  import type { MarkItem, Marks, SharedList } from '$lib/types';
  import { artW } from '$lib/ui/art';
  import Avatar from '$lib/ui/Avatar.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';

  // «Подборки» (бывшее «Моё», выбор владельца 28.11): сверху три отметки плитками с веером постеров - Буду смотреть,
  // Смотрю, Просмотрено; выбранная открыта сеткой с фильтром, поиском и порядком. Ниже - общие подборки с друзьями
  // обложками из четырёх постеров (участники на обложке, «Новое», если добавил кто-то другой за три дня).
  type Key = 'want' | 'watching' | 'watched';
  const TILES: { key: Key; label: string }[] = [
    { key: 'want', label: 'Буду смотреть' },
    { key: 'watching', label: 'Смотрю' },
    { key: 'watched', label: 'Просмотрено' },
  ];
  type Kind = 'all' | 'movie' | 'series' | 'anime';
  const KINDS: [Kind, string][] = [['all', 'Все'], ['movie', 'Фильмы'], ['series', 'Сериалы'], ['anime', 'Аниме']];
  const PAGE = 24;

  let marks = $state<Marks | null>(null);
  let lists = $state<SharedList[] | null>(null);
  let failed = $state(false);
  let retry = $state(0);
  let creating = $state(false);
  let kind = $state<Kind>('all');
  let q = $state('');
  let order = $state<'new' | 'rating' | 'title'>('new');
  let shown = $state(PAGE);

  const tab = $derived<Key>((['want', 'watching', 'watched'] as Key[]).find((k) => k === page.url.searchParams.get('m')) ?? 'want');
  // Не дольше 20 с: на медленной связи лучше «Повторить», чем вечная заглушка.
  $effect(() => {
    void extSheet.version;
    void extSheet.marks;
    void retry;
    let live = true;
    failed = false;
    const ctl = new AbortController();
    const t = setTimeout(() => ctl.abort(), 20000);
    Promise.all([
      api<Marks>('/api/me/marks', { signal: ctl.signal }),
      api<SharedList[]>('/api/lists', { signal: ctl.signal }).catch(() => [] as SharedList[]),
    ])
      .then(([m, l]) => {
        if (!live) return;
        marks = m;
        lists = l;
        // Ничего не хочу, но что-то смотрю - открыть «Смотрю».
        if (!page.url.searchParams.get('m') && !m.want.length && m.watching.length) pick('watching');
      }, () => live && (failed = true))
      .finally(() => clearTimeout(t));
    return () => {
      live = false;
      clearTimeout(t);
    };
  });

  function pick(k: Key) {
    shown = PAGE;
    goto(k === 'want' ? '/my' : `/my?m=${k}`, { replace: true, reset: false });
  }

  const low = (s: string) => s.toLowerCase().replaceAll('ё', 'е');
  // Отметка - фильм с сервиса (`card`) или из ленты IMDb (`ext`).
  type Info = { title: string; original?: string | null; kind: string; anime?: boolean; rating?: number | null; poster: string | null };
  const info = (m: MarkItem) => (m.ext ?? m.card) as unknown as Info;
  const keyOf = (m: MarkItem) => (m.ext ? `e${m.ext.id}` : `t${m.card?.id}`);
  const items = $derived.by<MarkItem[]>(() => {
    const all = (marks?.[tab] ?? []).filter((m) => m.ext || m.card);
    const t = low(q.trim());
    const out = all.filter((m) => {
      const c = info(m);
      return (kind === 'all' ? true : kind === 'anime' ? !!c.anime : c.kind === kind && !c.anime) && (!t || low(c.title).includes(t) || low(c.original ?? '').includes(t));
    });
    if (order === 'rating') return out.slice().sort((a, b) => (info(b).rating ?? 0) - (info(a).rating ?? 0));
    if (order === 'title') return out.slice().sort((a, b) => info(a).title.localeCompare(info(b).title, 'ru'));
    return out.slice().sort((a, b) => b.at - a.at);
  });
  const fan = (k: Key) => (marks?.[k] ?? []).filter((m) => (m.ext ?? m.card) && info(m).poster).slice(0, 3);
  const sub = (k: Key) => {
    const n = marks?.[k].length ?? 0;
    if (!n) return k === 'want' ? 'Отмечайте «Хочу» на фильмах' : k === 'watching' ? 'Начатое появится здесь' : 'Досмотренное соберётся здесь';
    const first = marks![k].find((m) => m.ext || m.card);
    return k === 'watching' && first ? `Последнее - «${info(first).title}»` : `${n} ${plural(n, 'фильм и сериал', 'фильма и сериала', 'фильмов и сериалов')}`;
  };
  const DAY = 86_400_000;
</script>

<svelte:head><title>Подборки - Nova</title></svelte:head>

<main class="sel">
  <header class="top">
    <div>
      <h1>Подборки</h1>
      <p>Ваши отметки и общие подборки с друзьями</p>
    </div>
    <button class="btn primary" onclick={() => (creating = true)}><Icon name="plus" size={16} /> Новая подборка</button>
  </header>

  {#if failed && !marks}
    <p class="note">Не удалось загрузить. Проверьте связь.</p>
    <button class="btn again" onclick={() => retry++}>Повторить</button>
  {:else if marks === null}
    <div class="skeleton" aria-busy="true"></div>
  {:else}
    <div class="tiles" role="tablist" aria-label="Мои отметки">
      {#each TILES as t (t.key)}
        <button class="tile" type="button" role="tab" aria-selected={tab === t.key} onclick={() => pick(t.key)}>
          <span class="fan" aria-hidden="true">{#each fan(t.key) as m (keyOf(m))}<img src={artW(info(m).poster, 185)} alt="" loading="lazy" decoding="async" />{/each}</span>
          <span class="n">{marks[t.key].length}</span>
          <b>{t.label}</b>
          <span class="sub">{sub(t.key)}</span>
        </button>
      {/each}
    </div>

    <div class="marks" role="tabpanel" aria-label={TILES.find((t) => t.key === tab)?.label}>
      {#if marks[tab].length}
        <div class="tools">
          <div class="chips" role="group" aria-label="Что показать">
            {#each KINDS as [k, label] (k)}<button type="button" class="chip" aria-pressed={kind === k} onclick={() => ((kind = k), (shown = PAGE))}>{label}</button>{/each}
          </div>
          <span class="sp"></span>
          <Select variant="sm" fit label="Порядок" value={order} onchange={(v) => (order = v as typeof order)} options={[{ value: 'new', label: 'Сначала новые' }, { value: 'rating', label: 'По оценке' }, { value: 'title', label: 'По названию' }]} />
          <input class="input find" type="search" bind:value={q} placeholder="Найти в списке" aria-label="Найти в списке" />
        </div>
        {#if items.length}
          <div class="grid">
            {#each items.slice(0, shown) as m, n (keyOf(m))}{#if m.ext}<Poster card={m.ext} marks i={n < 12 ? n : null} />{:else}<Poster local={m.card} marks i={n < 12 ? n : null} />{/if}{/each}
          </div>
          {#if items.length > shown}<div class="more"><button class="btn" onclick={() => (shown += PAGE * 2)}>Показать ещё {Math.min(items.length - shown, PAGE * 2)}</button></div>{/if}
        {:else}
          <p class="note">Ничего не нашлось.</p>
        {/if}
      {:else}
        <p class="note">{tab === 'want' ? 'Здесь будет то, что вы отметите «Хочу».' : tab === 'watching' ? 'Здесь будет то, что вы начали смотреть.' : 'Здесь будет досмотренное.'} Начните с <a href="/discover">каталога</a>.</p>
      {/if}
    </div>

    <section aria-labelledby="shared-h">
      <h2 class="row" id="shared-h">Общие подборки <small>с друзьями</small></h2>
      <div class="lists">
        {#each lists ?? [] as l (l.id)}
          {@const fresh = l.last && !l.last.mine && Date.now() - l.last.at < 3 * DAY}
          <a class="cover" href="/lists/{l.id}">
            <span class="art" class:one={l.art.length < 4}>
              {#each l.art.slice(0, l.art.length < 4 ? 1 : 4) as a, k (k)}<img src={artW(a, 342)} alt="" loading="lazy" decoding="async" />{/each}
              {#if !l.art.length}<span class="none"><Icon name="film" size={30} /></span>{/if}
              {#if fresh}<span class="badge">Новое</span>{/if}
              <span class="stack">{#each l.members.slice(0, 4) as m (m.id)}<Avatar avatar={m.avatar} photo={m.photo} size={30} />{/each}</span>
            </span>
            <b>{l.name}</b>
            <small>{l.count ? `${l.count} ${plural(l.count, 'фильм', 'фильма', 'фильмов')}` : 'пока пусто'}{l.last && !l.last.mine ? ` · ${l.last.by} добавил(а) «${l.last.title}»` : l.watched ? ` · ${l.watched} посмотрели` : ''}</small>
          </a>
        {/each}
        <button class="cover add" type="button" onclick={() => (creating = true)}>
          <span class="art"><span><i><Icon name="plus" size={26} /></i>Новая подборка</span></span>
        </button>
      </div>
    </section>
  {/if}
</main>
<NewList bind:open={creating} oncreated={(id) => goto(`/lists/${id}`)} />

<style>
  .sel { min-height: 100dvh; padding: calc(var(--header) + 24px) var(--gutter) calc(56px + var(--tabbar)); max-width: 1680px; margin: 0 auto; }
  .top { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; flex-wrap: wrap; margin-bottom: 30px; }
  .top > div { min-width: 0; }
  h1 { margin: 0; font-size: clamp(40px, 5.4vw, 72px); font-weight: 900; letter-spacing: -0.045em; line-height: 0.95; }
  .top p { margin: 10px 0 0; color: var(--muted); }
  .note { color: var(--muted); font-size: 16px; }
  .note a { color: #fff; text-decoration: underline; }
  .skeleton { height: 300px; border-radius: 24px; background: #141414; animation: wait 1.4s ease-in-out infinite; }
  @keyframes wait { 50% { opacity: 0.55; } }

  /* Плитки отметок с веером постеров. */
  .tiles { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
  .tile { position: relative; isolation: isolate; display: flex; flex-direction: column; justify-content: flex-end; height: 210px; padding: 22px; border: 0; border-radius: 24px; overflow: hidden; background: #141416; color: inherit; font: inherit; text-align: left; cursor: pointer; transition: transform 0.3s var(--ease), box-shadow 0.3s; }
  .tile:hover { transform: translateY(-3px); }
  .tile[aria-selected='true'] { box-shadow: 0 0 0 2px #fff; }
  .tile::before { content: ''; position: absolute; inset: 0; z-index: -1; background: linear-gradient(90deg, #141416 30%, #14141600 75%); }
  .fan { position: absolute; z-index: -2; top: 18px; right: -10px; width: 58%; height: 100%; }
  .fan img { position: absolute; top: 0; width: 46%; max-width: none; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 12px; box-shadow: -10px 10px 30px #000c; transition: transform 0.4s var(--ease); }
  .fan img:nth-child(1) { right: 44%; transform: rotate(-10deg) translateY(16px); }
  .fan img:nth-child(2) { right: 22%; transform: rotate(-2deg); z-index: 1; }
  .fan img:nth-child(3) { right: 0; transform: rotate(7deg) translateY(10px); }
  .tile:hover .fan img:nth-child(1) { transform: rotate(-14deg) translate(-8px, 12px); }
  .tile:hover .fan img:nth-child(3) { transform: rotate(11deg) translate(8px, 8px); }
  .n { font-size: 56px; font-weight: 900; letter-spacing: -0.05em; line-height: 0.9; font-variant-numeric: tabular-nums; }
  .tile b { margin-top: 6px; font-size: 19px; font-weight: 800; }
  .tile .sub { margin-top: 2px; max-width: 60%; color: var(--muted); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .marks { margin-top: 22px; }
  .tools { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 18px; }
  .chips { display: flex; gap: 8px; flex-wrap: wrap; }
  .chip { height: 40px; padding: 0 18px; border: 0; border-radius: 99px; background: #ffffff12; color: #d1d1d6; font: inherit; font-size: 14px; font-weight: 700; cursor: pointer; }
  .chip[aria-pressed='true'] { background: #fff; color: #000; }
  .sp { flex: 1; }
  .find { width: 220px; min-height: 40px; height: 40px; border-radius: 99px; }
  /* Постер - `--pw`, растёт с шириной окна, как в рядах сайта. */
  .grid { display: grid; grid-template-columns: repeat(auto-fill, var(--pw)); gap: 24px 16px; }
  .more { display: flex; justify-content: center; margin-top: 24px; }

  .row { display: flex; align-items: baseline; gap: 14px; margin: 52px 0 18px; font-size: 26px; font-weight: 800; letter-spacing: -0.02em; }
  .row small { color: var(--muted); font-size: 15px; font-weight: 600; letter-spacing: 0; }
  .lists { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 24px 18px; }
  .cover { display: block; min-width: 0; padding: 0; border: 0; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; transition: transform 0.3s var(--ease); }
  .cover:hover { transform: translateY(-4px); }
  .art { position: relative; display: grid; grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr; aspect-ratio: 1; border-radius: 22px; overflow: hidden; background: #141416; }
  .art img { width: 100%; max-width: none; height: 100%; object-fit: cover; }
  .art.one img { grid-column: 1 / -1; grid-row: 1 / -1; }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(0deg, #000a, #0000 45%); pointer-events: none; }
  .none { grid-column: 1 / -1; grid-row: 1 / -1; display: grid; place-items: center; color: var(--muted); }
  .badge { position: absolute; z-index: 1; top: 12px; left: 12px; padding: 4px 10px; border-radius: 99px; background: #fff; color: #000; font-size: 11.5px; font-weight: 800; }
  .stack { position: absolute; z-index: 1; left: 12px; bottom: 12px; display: flex; }
  .stack :global(.ava) { margin-right: -8px; box-shadow: 0 0 0 2px #000; }
  .cover b { display: block; margin-top: 12px; font-size: 17px; font-weight: 800; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cover small { display: block; color: var(--muted); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .add .art { place-items: center; grid-template-columns: 1fr; grid-template-rows: 1fr; border: 2px dashed #ffffff2e; background: none; }
  .add .art::after { content: none; }
  .add .art > span { display: grid; justify-items: center; gap: 10px; color: var(--muted); font-weight: 700; }
  .add i { display: grid; place-items: center; width: 56px; height: 56px; border-radius: 50%; background: #ffffff14; color: #fff; }

  @media (max-width: 700px) {
    .tiles { grid-template-columns: minmax(0, 1fr); gap: 10px; }
    .tile { height: 132px; padding: 16px; }
    .n { font-size: 40px; }
    .tile b { font-size: 16px; }
    .fan { top: 12px; width: 50%; }
    .grid { gap: 16px 10px; }
    .lists { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 18px 12px; }
    .find { flex: 1; width: auto; min-width: 0; }
  }
  /* Телефон: шапки нет - заголовок ниже строки состояния. */
  @media (max-width: 640px) {
    .sel { padding-top: calc(env(safe-area-inset-top) + 56px); }
    h1 { font-size: 40px; }
  }
</style>
