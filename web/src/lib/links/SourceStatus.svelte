<script lang="ts">
  import { api } from '$lib/api';

  // Пустой результат поиска видео: что искали и кто из источников отвечает (а не просто «нет»).
  let { title, original = null, year = null }: { title: string; original?: string | null; year?: number | null } = $props();

  type Source = { site: string; label: string; state: 'ok' | 'down' | 'unknown' };
  let sources = $state<Source[]>([]);
  $effect(() => {
    api<Source[]>('/api/sources/status').then((s) => (sources = s), () => {});
  });

  const names = $derived([title, original].filter((n, i, all): n is string => !!n && all.indexOf(n) === i));
  const stateText = (s: Source['state']) => (s === 'ok' ? 'отвечает' : s === 'down' ? 'не отвечает, проверим позже' : 'ещё не проверялся');
  const lordfilmDown = $derived(sources.some((s) => s.site === 'lordfilm' && s.state === 'down'));
</script>

<p class="muted small searched">
  Искали: {names.map((n) => `«${n}»`).join(', ')}{year ? ` (${year})` : ''}.
</p>
{#if sources.length}
  <ul class="sources" aria-label="Источники видео">
    {#each sources as s (s.site)}
      <li class={s.state}><b>{s.label}</b> - {stateText(s.state)}</li>
    {/each}
  </ul>
{/if}

{#if lordfilmDown}
  <p class="muted small">LordFilm сейчас не отвечает - попробуйте повторить чуть позже.</p>
{:else}
  <p class="muted small">Если на LordFilm ничего не нашлось, можно попробовать другой источник или вставьте ссылку вручную.</p>
{/if}

<style>
  .searched {
    margin: 0 0 8px;
  }
  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin: 0 0 14px;
    padding: 0;
    list-style: none;
    font-size: 13px;
    color: var(--muted);
  }
  .sources li.down b {
    color: var(--danger);
  }
  .sources li.ok b {
    color: var(--ok);
  }
</style>
