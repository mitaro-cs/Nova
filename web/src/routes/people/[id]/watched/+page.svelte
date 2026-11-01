<script lang="ts">
  import { page } from '$app/state';
  import { api, errorText } from '$lib/api';
  import { count } from '$lib/fmt';
  import Poster from '$lib/discover/Poster.svelte';
  import type { Card, ExtCard } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';
  import Sheet from '$lib/ui/Sheet.svelte';

  // Что человек смотрел: отдельно фильмы и сериалы (с сервиса и из Обзора), свежее первым.
  const id = $derived(page.params.id);
  const kind = $derived(page.url.searchParams.get('kind') === 'series' ? 'series' : 'movie');
  let data = $state<{ local: Card[]; ext: ExtCard[] } | null>(null);
  let error = $state('');

  $effect(() => {
    const who = id;
    const k = kind;
    const controller = new AbortController();
    data = null;
    error = '';
    api<{ local: Card[]; ext: ExtCard[] }>(`/api/people/${who}/watched?kind=${k}`, { signal: controller.signal }).then(
      (r) => !controller.signal.aborted && (data = r),
      (e) => { if (!controller.signal.aborted) error = errorText(e); },
    );
    return () => controller.abort();
  });
  const total = $derived(data ? data.local.length + data.ext.length : 0);
</script>

<Sheet title={kind === 'series' ? 'Какие сериалы смотрел' : 'Какие фильмы смотрел'} subtitle={data ? count(total, 'отметка', 'отметки', 'отметок') : ''}>
  {#snippet above()}
    <a class="back" href="/people/{id}"><Icon name="back" size={16} /> К профилю</a>
  {/snippet}
  {#snippet actions()}
    <div class="chips" role="tablist" aria-label="Что показывать">
      <a class="chip" class:on={kind === 'movie'} role="tab" aria-selected={kind === 'movie'} href="/people/{id}/watched?kind=movie">Фильмы</a>
      <a class="chip" class:on={kind === 'series'} role="tab" aria-selected={kind === 'series'} href="/people/{id}/watched?kind=series">Сериалы</a>
    </div>
  {/snippet}

  <div class="rw-block">
    {#if error}
      <div class="empty"><Icon name="info" size={36} /> {error}</div>
    {:else if data === null}
      <div class="grid">{#each Array(14) as _, i (i)}<span class="sk skeleton"></span>{/each}</div>
    {:else if total === 0}
      <div class="empty">{kind === 'series' ? 'Сериалов пока нет' : 'Фильмов пока нет'} - {count(0, 'отметка', 'отметки', 'отметок')}.</div>
    {:else}
      <div class="grid">
        {#each data.local as c, i (`t${c.id}`)}<Poster local={c} {i} />{/each}
        {#each data.ext as e, i (`x${e.id}`)}<Poster card={e} i={i + data.local.length} />{/each}
      </div>
    {/if}
  </div>
</Sheet>

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 12px;
    color: var(--muted);
    font-size: 14px;
  }
  .back:hover {
    color: var(--text);
  }
  .chips {
    display: flex;
    gap: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 150px), 1fr));
    gap: 22px 16px;
  }
  .sk {
    aspect-ratio: 2 / 3;
    border-radius: 12px;
  }
  @media (max-width: 640px) {
    .grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 16px 10px;
    }
  }
</style>
