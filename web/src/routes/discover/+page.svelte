<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, errorText, post } from '$lib/api';
  import Poster from '$lib/discover/Poster.svelte';
  import PosterRow from '$lib/discover/PosterRow.svelte';
  import { extSheet } from '$lib/discover/sheet.svelte';
  import LinkFinder from '$lib/links/LinkFinder.svelte';
  import { session } from '$lib/session.svelte';
  import type { ExtCard, NowItem } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import BrowseGenres from '$lib/ui/BrowseGenres.svelte';
  import Loading from '$lib/ui/Loading.svelte';
  import PosterWall from '$lib/social/PosterWall.svelte';

  // Обзор как на IMDb: что смотрят друзья, «потому что вам понравилось», новинки, жанры, российское,
  // аниме, популярное и топ-250 (не только то, что есть у нас), поиск по всему IMDb, видео по ссылке.
  // У каждого фильма - своя страница `/ext/<id>`. Капсулы сверху - только фильмы или сериалы.
  type Row = { key: string; title: string; items: ExtCard[] };

  const q = $derived(page.url.searchParams.get('q') ?? '');
  const chart = $derived(page.url.searchParams.get('chart'));

  let feed = $state<{ rows: Row[]; updatedAt: number | null } | null>(null);
  let friends = $state<NowItem[]>([]);
  const kind = $derived(page.url.searchParams.get('kind'));
  const rows = $derived((feed?.rows ?? []).filter((r) => !kind || r.key.endsWith(kind === 'series' ? '_series' : '_movies')));
  const friendRows = $derived(friends.filter((f) => !kind || (f.card ?? f.ext)?.kind === (kind === 'series' ? 'series' : 'movie')));
  let full = $state<Row | null>(null);
  let found = $state<ExtCard[] | null>(null);
  let failed = $state('');
  let text = $state('');
  let finder = $state(false);
  let searching = $state(false);

  $effect(() => {
    text = q;
  });

  // Пустая лента - сервер только начал загрузку: спрашиваем ещё раз, пока не появится.
  $effect(() => {
    void extSheet.version;
    if (chart) return;
    let timer = 0;
    let tries = 0;
    const load = () =>
      Promise.all([
        api<{ rows: Row[]; updatedAt: number | null }>('/api/discover'),
        api<NowItem[]>('/api/friends/activity').catch(() => [] as NowItem[]),
      ]).then(
        ([r, a]) => {
          friends = a;
          feed = r;
          failed = '';
          if (!r.rows.length && ++tries < 10) timer = window.setTimeout(load, 4000);
        },
        (e) => (failed = errorText(e)),
      );
    load();
    return () => clearTimeout(timer);
  });

  $effect(() => {
    void extSheet.version;
    if (!chart) {
      full = null;
      return;
    }
    api<Row>(`/api/discover/chart/${encodeURIComponent(chart)}`).then(
      (r) => (full = r),
      (e) => (failed = errorText(e)),
    );
  });

  $effect(() => {
    void extSheet.version;
    const query = q.trim();
    if (query.length < 2) {
      found = null;
      return;
    }
    searching = true;
    api<ExtCard[]>(`/api/discover/search?q=${encodeURIComponent(query)}`)
      .then(
        (r) => (found = r),
        (e) => {
          found = [];
          toast(errorText(e));
        },
      )
      .finally(() => (searching = false));
  });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const v = text.trim();
    goto(v ? `/discover?q=${encodeURIComponent(v)}` : '/discover', { reset: false });
  }

  async function refresh() {
    try {
      await post('/api/admin/discover/refresh');
      toast('Лента обновится через минуту');
    } catch (e) {
      toast(errorText(e));
    }
  }
</script>

<div class="rw-page">
  <section class="stage">
    <div class="wall" aria-hidden="true"><PosterWall posters={[]} prominent /></div>
    <div class="inner">
      <h1>Обзор</h1>
      <p class="lead">Что смотрит мир кино - и чего у вас пока нет.</p>
      <form class="find" role="search" onsubmit={submit}>
        <Icon name="search" size={22} />
        <input type="search" bind:value={text} placeholder="Поиск по всему IMDb" aria-label="Поиск по IMDb" />
        {#if searching}<span class="spin" aria-hidden="true"></span>{/if}
      </form>
      <div class="bar">
        <nav class="kinds" aria-label="Что показывать">
          <a class="chip" class:on={!kind} href="/discover">Всё</a>
          <a class="chip" class:on={kind === 'movies'} href="/discover?kind=movies">Фильмы</a>
          <a class="chip" class:on={kind === 'series'} href="/discover?kind=series">Сериалы</a>
        </nav>
        <div class="acts">
          <button class="btn" onclick={() => (finder = true)}><Icon name="link" size={18} /> Смотреть по ссылке</button>
          {#if session.me?.user.role === 'host'}
            <a class="btn" href="/admin?tab=catalog#sources">Источники каталога <Icon name="right" size={16} /></a>
          {/if}
        </div>
      </div>
    </div>
  </section>

  {#if q.trim().length >= 2}
    <section aria-label="Найдено">
      <div class="head">
        <h2>Найдено на IMDb</h2>
        <a class="muted" href="/discover">Сбросить</a>
      </div>
      {#if found === null}
        <div class="grid">{#each Array(12) as _, i (i)}<span class="ph skeleton"></span>{/each}</div>
      {:else if found.length}
        <div class="grid">{#each found as card, i (card.id)}<Poster {card} {i} />{/each}</div>
      {:else}
        <div class="empty"><Icon name="search" size={40} /><p>По запросу «{q}» ничего нет</p></div>
      {/if}
    </section>
  {:else if chart}
    <section aria-label={full?.title ?? 'Подборка'}>
      <div class="head">
        <h2>{full?.title ?? ''}</h2>
        <a class="muted" href="/discover"><Icon name="back" size={14} /> Обзор</a>
      </div>
      {#if full}
        <div class="grid">{#each full.items as card, i (card.id)}<Poster {card} rank={chart.startsWith('top') ? i + 1 : null} {i} />{/each}</div>
      {:else}
        <div class="grid">{#each Array(18) as _, i (i)}<span class="ph skeleton"></span>{/each}</div>
      {/if}
    </section>
  {:else if failed && !feed}
    <div class="empty tall"><Icon name="offline" size={40} /><p>{failed}</p></div>
  {:else if feed === null}
    <Loading label="Собираем подборки" />
    {#each Array(2) as _, r (r)}
      <div class="ph-row">
        <span class="ph-title skeleton"></span>
        <div class="ph-cards">{#each Array(8) as _, i (i)}<span class="ph skeleton"></span>{/each}</div>
      </div>
    {/each}
  {:else if feed.rows.length}
    <BrowseGenres />
    {#if friendRows.length}<PosterRow title="Друзья смотрят" items={friendRows} />{/if}
    {#each rows as row, i (row.key)}
      <PosterRow title={row.title} items={row.items} href={row.key.startsWith('because') ? null : `/discover?chart=${row.key}`} ranked={row.key.startsWith('top')} {i} />
    {/each}
    {#if session.me?.manage}
      <p class="foot muted">
        Данные IMDb{#if feed.updatedAt}, обновлено {new Date(feed.updatedAt).toLocaleString('ru-RU', { day: 'numeric', month: 'long', hour: '2-digit', minute: '2-digit' })}{/if}.
        <button class="linkish" onclick={refresh}>Обновить</button>
      </p>
    {/if}
  {:else}
    <div class="empty tall">
      <Icon name="globe" size={40} />
      <p>Загружаем подборки IMDb - это займёт минуту.</p>
      <p class="muted">Пока можно поискать фильм выше.</p>
    </div>
  {/if}
</div>

<LinkFinder bind:open={finder} />

<style>
  /* Сцена: стена постеров подборок за заголовком и поиском. Только transform у стены, без blur. */
  .stage {
    position: relative;
    isolation: isolate;
    overflow: hidden;
    margin-bottom: 8px;
    border: 1px solid var(--line);
    border-radius: clamp(22px, 3vw, 36px);
    background: #050506;
  }
  .wall {
    position: absolute;
    inset: 0;
    z-index: -1;
    opacity: 0.5;
  }
  .stage::after {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background:
      linear-gradient(90deg, #050506f2 0%, #050506b8 45%, #0505068c 100%),
      linear-gradient(180deg, transparent 55%, #050506 100%);
  }
  .inner {
    padding: clamp(28px, 5vw, 72px) clamp(20px, 4vw, 64px) clamp(22px, 3vw, 40px);
    max-width: 980px;
  }
  .inner h1 {
    font-size: clamp(38px, 5.4vw, 76px);
    font-weight: 700;
    letter-spacing: -0.035em;
    line-height: 1;
  }
  .lead {
    margin: 12px 0 clamp(20px, 3vw, 32px);
    max-width: 46ch;
    color: #d4d4d8;
    font-size: clamp(15px, 1.5vw, 19px);
    line-height: 1.4;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 12px;
    height: clamp(52px, 5vw, 64px);
    padding: 0 22px;
    border-radius: 999px;
    border: 1px solid #ffffff2e;
    background: #141416d9;
    color: var(--muted);
    transition: border-color 0.2s, background 0.2s;
  }
  .find:focus-within {
    border-color: #ffffffa6;
    background: #1b1b1e;
  }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    font-size: clamp(16px, 1.5vw, 19px);
    color: var(--text);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 16px;
    margin-top: 18px;
  }
  .kinds {
    display: inline-flex;
    gap: 4px;
    padding: 4px;
    max-width: 100%;
    overflow-x: auto;
    border: 1px solid #ffffff1f;
    border-radius: 999px;
    background: #0c0c0ecc;
  }
  .kinds .chip {
    border-color: transparent;
    background: none;
  }
  .kinds .chip.on {
    background: #fff;
    color: #0a0a0b;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  @media (max-width: 640px) {
    .acts,
    .acts .btn {
      width: 100%;
    }
    .acts .btn {
      justify-content: center;
    }
  }
  .spin {
    width: 16px;
    height: 16px;
    border: 2px solid var(--line);
    border-top-color: var(--text);
    border-radius: 50%;
    animation: turn 0.8s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(1turn);
    }
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 16px;
  }
  .head a {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 14px;
  }
  h2 {
    font-size: max(17px, calc(20 * var(--rw)));
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, calc(var(--pw) - 8px)), 1fr));
    gap: 22px 16px;
  }
  @media (max-width: 640px) {
    .grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 16px 10px;
    }
  }
  .ph {
    aspect-ratio: 2 / 3;
    border-radius: 12px;
  }
  .ph-row {
    margin-bottom: 30px;
  }
  .ph-title {
    display: block;
    width: 180px;
    height: 24px;
    border-radius: 6px;
    margin-bottom: 14px;
  }
  .ph-cards {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: var(--pw);
    gap: 16px;
    overflow: hidden;
  }
  @media (max-width: 640px) {
    .ph-cards {
      gap: 10px;
    }
  }
  .tall {
    min-height: 50vh;
  }
  .foot {
    font-size: 13px;
  }
  .linkish {
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }
</style>
