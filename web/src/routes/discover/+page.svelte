<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, errorText, post } from '$lib/api';
  import Poster from '$lib/discover/Poster.svelte';
  import PosterRow from '$lib/discover/PosterRow.svelte';
  import CatalogHero from '$lib/discover/CatalogHero.svelte';
  import GenreTiles from '$lib/discover/GenreTiles.svelte';
  import TopTen from '$lib/ui/TopTen.svelte';
  import { extSheet } from '$lib/discover/sheet.svelte';
  import LinkFinder from '$lib/links/LinkFinder.svelte';
  import { session } from '$lib/session.svelte';
  import type { ExtCard, NowItem } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import CatalogStrip from '$lib/ui/CatalogStrip.svelte';
  import Loading from '$lib/ui/Loading.svelte';

  // Каталог «Афиша мира» (выбор владельца 25.11; бывший Обзор): сверху самое популярное на IMDb кадром на всю ширину
  // (`CatalogHero`, поиск IMDb и «Смотреть по ссылке» - капсулами на кадре), Топ-10 фильмов и сериалов мира, жанры
  // плитками (`GenreTiles`), дальше прежние ряды: друзья, «потому что вам понравилось», новинки, скоро, лучшие, российское.
  // У каждого фильма - своя страница `/ext/<id>`. Капсулы «Всё / Только фильмы / Только сериалы» - тип. Проверка
  // «Как работает видео» переехала в «Управление → Каталог».
  type Row = { key: string; title: string; items: ExtCard[] };

  const q = $derived(page.url.searchParams.get('q') ?? '');
  const chart = $derived(page.url.searchParams.get('chart'));

  let feed = $state<{ rows: Row[]; updatedAt: number | null } | null>(null);
  let friends = $state<NowItem[]>([]);
  const kind = $derived(page.url.searchParams.get('kind'));
  const rows = $derived((feed?.rows ?? []).filter((r) => !kind || r.key.endsWith(kind === 'series' ? '_series' : '_movies')));
  const friendRows = $derived(friends.filter((f) => !kind || (f.card ?? f.ext)?.kind === (kind === 'series' ? 'series' : 'movie')));
  const byKey = (k: string) => feed?.rows.find((r) => r.key === k);
  const GENRE = /^(comedy|action|thriller|horror|scifi|drama|romance|animation|family|documentary|crime)_(movies|series)$/;
  const popMovies = $derived(kind === 'series' ? undefined : byKey('popular_movies'));
  const popSeries = $derived(kind === 'movies' ? undefined : byKey('popular_series'));
  const heroItems = $derived(((popMovies ?? popSeries)?.items ?? rows[0]?.items ?? []).filter((c) => c.poster));
  const showHero = $derived(!chart && q.trim().length < 2 && heroItems.length > 0);
  const genreRows = $derived(rows.filter((r) => GENRE.test(r.key) && r.items.length));
  const restRows = $derived(rows.filter((r) => !r.key.startsWith('popular_') && !GENRE.test(r.key)));
  const top = (r?: Row) => (r?.items ?? []).slice(0, 10).map((ext) => ({ ext }));
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
    if (chart || q.trim().length >= 2) return;
    let timer = 0;
    let tries = 0;
    const scope = kind === 'series' ? 'series' : kind === 'movies' ? 'movies' : '';
    const feedUrl = scope ? `/api/discover?scope=${scope}` : '/api/discover';
    const load = () =>
      Promise.all([
        api<{ rows: Row[]; updatedAt: number | null }>(feedUrl),
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

<CatalogStrip on="all" />
<div class="rw-page" class:with-hero={showHero}>
  {#snippet search(glass: boolean)}
    <form class="find" class:glass role="search" onsubmit={submit}>
      <Icon name="search" size={glass ? 18 : 22} />
      <input type="search" bind:value={text} placeholder="Поиск по всему IMDb" aria-label="Поиск по IMDb" />
      {#if searching}<span class="spin" aria-hidden="true"></span>{/if}
    </form>
  {/snippet}
  {#snippet kinds()}
    <nav class="kinds" aria-label="Что показывать">
      <a class="chip" class:on={!kind} href="/discover">Всё</a>
      <a class="chip" class:on={kind === 'movies'} href="/discover?kind=movies">Только фильмы</a>
      <a class="chip" class:on={kind === 'series'} href="/discover?kind=series">Только сериалы</a>
    </nav>
  {/snippet}
  {#if showHero}
    <CatalogHero items={heroItems} label="Самое популярное на IMDb">
      {#snippet tools()}
        {@render search(true)}
        <button class="btn link-btn" onclick={() => (finder = true)}><Icon name="link" size={17} /> Смотреть по ссылке</button>
      {/snippet}
    </CatalogHero>
    <div class="bar under">{@render kinds()}</div>
  {:else}
    <section class="stage">
      <h1>Каталог</h1>
      <p class="lead">Что смотрит мир кино - и чего у вас пока нет.</p>
      {@render search(false)}
      <div class="bar">
        {@render kinds()}
        <button class="btn" onclick={() => (finder = true)}><Icon name="link" size={18} /> Смотреть по ссылке</button>
      </div>
    </section>
  {/if}

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
        <a class="muted" href="/discover"><Icon name="back" size={14} /> Каталог</a>
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
    {#if popMovies && popMovies.items.length >= 3}<TopTen items={top(popMovies)} title="Топ-10 фильмов мира сейчас" id="top-m" />{/if}
    {#if popSeries && popSeries.items.length >= 3}<TopTen items={top(popSeries)} title="Топ-10 сериалов мира сейчас" id="top-s" />{/if}
    {#if friendRows.length}<PosterRow title="Друзья смотрят" items={friendRows} />{/if}
    <GenreTiles rows={genreRows} />
    {#each restRows as row, i (row.key)}
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
  @media (min-width: 1001px) {
    :global(.rw-page) {
      --pw: 176px;
    }
  }
  /* С кадром: он сразу под полосой каталога, без верхнего поля страницы. */
  .rw-page.with-hero {
    padding-top: var(--header);
  }
  /* Без кадра (поиск, подборка, загрузка): заголовок, поиск и тип - просто на чёрном, без «карточки в карточке». */
  .stage {
    margin-bottom: 26px;
  }
  .stage h1 {
    margin: 0;
    font-size: clamp(34px, 4.4vw, 60px);
    font-weight: 800;
    letter-spacing: -0.04em;
    line-height: 1.02;
  }
  .lead {
    margin: 10px 0 20px;
    max-width: 46ch;
    color: #a1a1aa;
    font-size: clamp(15px, 1.4vw, 18px);
    line-height: 1.4;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: 760px;
    height: 56px;
    padding: 0 22px;
    border-radius: 999px;
    border: 1px solid #ffffff2e;
    background: #141416;
    color: var(--muted);
    transition: border-color 0.2s, background 0.2s;
  }
  .find:focus-within {
    border-color: #ffffffa6;
    background: #1b1b1e;
  }
  /* На кадре - тонкая капсула справа сверху. */
  .find.glass {
    flex: 0 1 430px;
    height: 46px;
    padding: 0 18px;
    gap: 10px;
    background: #000000a6;
    border-color: #ffffff2e;
  }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    font-size: 16px;
    color: var(--text);
  }
  .find.glass input {
    font-size: 15px;
  }
  .link-btn {
    height: 46px;
    background: #000000a6;
    border-color: #ffffff2e;
    color: #fff;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 16px;
    margin-top: 18px;
  }
  .bar.under {
    margin: 0 0 22px;
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
  @media (max-width: 640px) {
    .find.glass {
      flex: 1 1 auto;
    }
    .link-btn {
      width: 46px;
      padding: 0;
      font-size: 0;
      gap: 0;
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
