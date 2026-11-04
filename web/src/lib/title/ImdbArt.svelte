<script lang="ts">
  import { api, errorText } from '$lib/api';
  import Icon from '$lib/ui/Icon.svelte';
  import Modal from '$lib/ui/Modal.svelte';
  import { toast } from '$lib/ui/toast.svelte';

  // Хост выбирает обложку и фон фильма среди картинок IMDb (постеры и кадры) и качество: обычное, высокое, максимум.
  // Превью идёт через сервер (страница к IMDb не ходит), выбранное скачивается к нам.
  // base - адрес фильма в админском API: `/api/admin/titles/<id>` (свой) или `/api/admin/ext/<imdb>` (из ленты IMDb).
  let { open = $bindable(false), base, onpicked }: { open: boolean; base: string; onpicked: (which: 'poster' | 'backdrop', url: string) => void } = $props();

  type Art = { url: string; width: number; height: number; kind: 'poster' | 'frame'; caption: string | null };
  let items = $state<Art[] | null>(null);
  let error = $state('');
  let tab = $state<'poster' | 'frame'>('poster');
  let picked = $state<Art | null>(null);
  let width = $state(1000);
  let busy = $state('');

  const QUALITY = [
    { w: 500, label: 'Обычное', note: 'быстро, до 500 px' },
    { w: 1000, label: 'Высокое', note: 'до 1000 px' },
    { w: 2000, label: 'Максимум', note: 'до 2000 px' },
  ];
  const shown = $derived((items ?? []).filter((a) => a.kind === tab));
  const real = $derived(picked ? Math.min(width, picked.width) : 0);

  $effect(() => {
    if (!open || items) return;
    let live = true;
    error = '';
    api<{ items: Art[] }>(`${base}/imdb-art`).then(
      (r) => {
        if (!live) return;
        items = r.items;
        if (!r.items.some((a) => a.kind === 'poster') && r.items.length) tab = 'frame';
      },
      (e) => live && (error = errorText(e)),
    );
    return () => (live = false);
  });
  $effect(() => {
    void tab;
    picked = null;
  });

  async function apply(which: 'poster' | 'backdrop') {
    if (!picked || busy) return;
    busy = which;
    try {
      const r = await api<{ url: string }>(`${base}/image/${which}/imdb`, { method: 'POST', json: { url: picked.url, width } });
      onpicked(which, r.url);
      toast(which === 'poster' ? 'Обложка обновлена' : 'Фон обновлён');
      open = false;
    } catch (x) {
      toast(errorText(x));
    } finally {
      busy = '';
    }
  }
  const thumb = (a: Art) => `/api/admin/imdb-art/preview?u=${encodeURIComponent(a.url)}&w=${a.kind === 'poster' ? 342 : 640}`;
</script>

<Modal bind:open title="Картинки с IMDb" wide>
  <div class="art">
    {#if error}
      <p class="error" role="alert">{error}</p>
    {:else if items === null}
      <p class="muted">Загружаем картинки с IMDb…</p>
    {:else if !items.length}
      <p class="muted">У этого фильма на IMDb нет картинок.</p>
    {:else}
      <div class="bar">
        <div class="seg" role="tablist" aria-label="Что выбрать">
          <button role="tab" aria-selected={tab === 'poster'} class:on={tab === 'poster'} onclick={() => (tab = 'poster')}>Постеры</button>
          <button role="tab" aria-selected={tab === 'frame'} class:on={tab === 'frame'} onclick={() => (tab = 'frame')}>Кадры</button>
        </div>
        <div class="seg" role="radiogroup" aria-label="Качество">
          {#each QUALITY as q (q.w)}
            <button role="radio" aria-checked={width === q.w} class:on={width === q.w} title={q.note} onclick={() => (width = q.w)}>{q.label}</button>
          {/each}
        </div>
      </div>
      <div class="grid" class:frames={tab === 'frame'}>
        {#each shown as a (a.url)}
          <button class="pic" style:--pad="{((a.height / a.width) * 100).toFixed(3)}%" class:on={picked?.url === a.url} aria-pressed={picked?.url === a.url} onclick={() => (picked = a)} title={a.caption ?? ''}>
            <img src={thumb(a)} alt={a.caption ?? ''} loading="lazy" decoding="async" />
            <small>{a.width} × {a.height}</small>
          </button>
        {:else}
          <p class="muted">Здесь пусто - загляните во вкладку рядом.</p>
        {/each}
      </div>
      <div class="foot">
        <span class="muted">{picked ? `Скачаем ${real} px по ширине` : 'Выберите картинку'}</span>
        <button class="btn" disabled={!picked || !!busy} onclick={() => apply('poster')}><Icon name="image" size={16} /> {busy === 'poster' ? 'Загружаем…' : 'Обложкой'}</button>
        <button class="btn primary" disabled={!picked || !!busy} onclick={() => apply('backdrop')}><Icon name="image" size={16} /> {busy === 'backdrop' ? 'Загружаем…' : 'Фоном'}</button>
      </div>
    {/if}
  </div>
</Modal>

<style>
  .art {
    display: grid;
    gap: 16px;
    width: 100%;
    min-width: 0;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 18px;
    justify-content: space-between;
    min-width: 0;
  }
  .seg {
    display: inline-flex;
    gap: 4px;
    padding: 4px;
    border-radius: 999px;
    background: #ffffff12;
  }
  .seg button {
    padding: 7px 16px;
    border: 0;
    border-radius: 999px;
    background: none;
    color: var(--muted);
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.2s, color 0.2s;
  }
  .seg button.on {
    background: #fff;
    color: #0a0a0a;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(112px, 1fr));
    align-items: start;
    gap: 10px;
    max-height: 56vh;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px;
    margin: -4px;
  }
  .grid.frames {
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
  }
  .pic {
    position: relative;
    display: block;
    width: 100%;
    /* Высота - отступом в процентах от ширины: строка сетки считает её всегда, а у aspect-ratio плитки в окне
       схлопывались и ложились друг на друга. */
    height: 0;
    padding: var(--pad, 150%) 0 0;
    border: 0;
    border-radius: 14px;
    overflow: hidden;
    background: #ffffff0d;
    cursor: pointer;
    transition: transform 0.35s var(--ease), box-shadow 0.2s;
  }
  /* Картинка - в своих пропорциях (без обрезки), чтобы было видно, что именно выбираешь. */
  .pic img {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    background: #ffffff0d;
  }
  .pic small {
    position: absolute;
    left: 8px;
    bottom: 8px;
    padding: 2px 8px;
    border-radius: 99px;
    background: #000000b3;
    font-size: 11px;
    color: #fff;
  }
  .pic:hover {
    transform: translateY(-3px);
  }
  .pic.on {
    box-shadow: 0 0 0 3px #fff;
    transform: translateY(-3px);
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    justify-content: flex-end;
  }
  .foot .muted {
    margin-right: auto;
  }
</style>
