<script lang="ts">
  import { api } from '$lib/api';
  import { catalog, loadCatalog } from '$lib/catalog.svelte';
  import { extSheet } from '$lib/discover/sheet.svelte';
  import type { Card, ExtCard } from '$lib/types';
  import FeedCard from '$lib/ui/FeedCard.svelte';

  // «Что смотреть» (телефон, как у Кинопоиска): вертикальная лента карточек, на экране играет трейлер
  // одной. Свои фильмы и подборки IMDb вперемешку (два из ленты, один свой); отмеченное «Просмотрено» и
  // «Не интересно» не показываем. Подгружается по шесть, когда лента подходит к концу.
  type Item = { key: string; card?: Card; ext?: ExtCard };
  type Feed = { rows: { key: string; items: ExtCard[] }[] };
  let feed = $state<Feed | null>(null);
  let failed = $state(false);
  let hidden = $state<Set<string>>(new Set());
  let current = $state('');
  let limit = $state(6);
  let more = $state<HTMLElement>();

  $effect(() => {
    loadCatalog();
  });
  $effect(() => {
    void extSheet.version;
    let live = true;
    api<Feed>('/api/discover').then(
      (f) => live && ((feed = f), (failed = false)),
      () => live && ((feed = { rows: [] }), (failed = true)),
    );
    return () => (live = false);
  });

  const items = $derived.by<Item[]>(() => {
    if (!feed || !catalog.items) return [];
    const own = catalog.items
      .filter((c) => c.status !== 'dropped' && c.status !== 'watched' && (c.backdrop || c.poster))
      .sort((a, b) => b.addedAt - a.addedAt)
      .map<Item>((card) => ({ key: `t${card.id}`, card }));
    const seen = new Set<string>();
    const out: Item[] = [];
    for (const e of feed.rows.flatMap((r) => r.items)) {
      if (e.local || seen.has(e.id) || e.mark === 'dropped' || e.mark === 'watched' || !e.poster) continue;
      seen.add(e.id);
      out.push({ key: `x${e.id}`, ext: e });
    }
    const mixed: Item[] = [];
    for (let i = 0, o = 0; i < own.length || o < out.length; i++) {
      for (const e of out.slice(o, o + 2)) mixed.push(e);
      o += 2;
      if (own[i]) mixed.push(own[i]);
    }
    return mixed.filter((m) => !hidden.has(m.key));
  });
  const shown = $derived(items.slice(0, limit));

  $effect(() => {
    if (!more) return;
    const io = new IntersectionObserver((e) => e[0].isIntersecting && (limit += 6), { rootMargin: '100% 0px' });
    io.observe(more);
    return () => io.disconnect();
  });
</script>

<svelte:head><title>Что смотреть - Nova</title></svelte:head>

<main class="feed">
  <h1>Что смотреть</h1>
  {#if feed === null || !catalog.items}
    <div class="skeleton" aria-busy="true"></div>
  {:else if !items.length}
    <p class="note">{failed ? 'Подборки сейчас недоступны. Проверьте связь и зайдите позже.' : 'Пока нечего предложить - загляните позже.'}</p>
  {:else}
    <div class="list">
      {#each shown as it (it.key)}
        <FeedCard
          card={it.card}
          ext={it.ext}
          active={current === it.key}
          onseen={() => (current = it.key)}
          ongone={() => (hidden = new Set([...hidden, it.key]))}
        />
      {/each}
    </div>
    {#if items.length > limit}<div bind:this={more} class="more" aria-hidden="true"></div>{/if}
  {/if}
</main>

<style>
  .feed {
    min-height: 100dvh;
    padding: calc(var(--header) + 24px) 0 calc(48px + var(--tabbar));
  }
  h1 {
    margin: 0 var(--gutter) 24px;
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }
  .list {
    display: grid;
    gap: 16px;
    padding: 0 var(--gutter);
    max-width: 720px;
    margin-inline: auto;
  }
  .note {
    margin: 0 var(--gutter);
    color: var(--muted);
    font-size: 16px;
  }
  .skeleton {
    aspect-ratio: 16 / 11;
    margin: 0 var(--gutter);
    background: #141414;
  }
  .more {
    height: 1px;
  }
  @media (max-width: 640px) {
    .feed {
      padding-top: calc(env(safe-area-inset-top) + 56px);
    }
  }
</style>
