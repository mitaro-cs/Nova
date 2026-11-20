<script lang="ts">
  import { fadeIn } from '$lib/ui/fade';
  import type { Card, ExtCard } from '$lib/types';
  import { artW, placeholder } from '$lib/ui/art';
  import QuickMarks from '$lib/ui/QuickMarks.svelte';
  import { upcomingYear } from '$lib/title/upcoming';
  import { remember } from './sheet.svelte';

  // Постер 2:3: оценка плашкой в углу (от 7 - зелёная, ниже - серая), название и год, поверх - быстрые
  // отметки. Из ленты IMDb - ссылка на его
  // страницу `/ext/<id>` (карточка запоминается - страница показывается сразу), с сервиса
  // (`local`) - на `/title/<id>`.
  let {
    card,
    local = null,
    rank = null,
    i = null,
    marks = true,
  }: { card?: ExtCard; local?: Card | null; rank?: number | null; i?: number | null; marks?: boolean } = $props();

  const c = $derived(
    local
      ? { title: local.title, year: local.year, rating: local.rating, poster: local.poster, here: false, kind: local.kind, genre: local.genres?.[0] ?? '' }
      : { title: card!.title, year: card!.year, rating: card!.rating, poster: card!.poster, here: !!card!.local, kind: card!.kind, genre: card!.genres?.[0] ?? '' },
  );
  // Не вышел: плашка «Скоро» на постере и «Скоро · год» вместо одного года.
  const soon = $derived(!local && (!!card?.soon || !!upcomingYear(card?.release)));
  let replacement = $state<string | null>(null);
  let loaded = $state(false);
  // Запасной адрес тоже не дал картинку: остаётся заглушка с названием, без битого <img>.
  let failed = $state(false);
  const source = $derived(failed ? null : (replacement ?? c.poster));
  $effect(() => { void c.poster; void card?.id; replacement = null; loaded = false; failed = false; });
  // Картинка из кэша браузера бывает готова раньше, чем навешан onload: тогда заглушка с названием висела поверх постера.
  function ready(img: HTMLImageElement) {
    if (img.complete && img.naturalWidth > 0) loaded = true;
  }
  function retryImage() {
    const fallback = card ? `/api/discover/poster/${card.id}` : null;
    if (fallback && source !== fallback) replacement = fallback;
    else { loaded = false; failed = true; }
  }
</script>

{#snippet body()}
  <span class="art" class:ranked={rank !== null} style:background={c.poster ? undefined : placeholder(c.title)}>
    {#if !loaded}<span class="ph"><i>{c.kind==='series'?'СЕРИАЛ':'ФИЛЬМ'}</i><b>{c.title}</b>{#if c.year}<small>{c.year}</small>{/if}</span>{/if}
    {#if source}<img use:fadeIn use:ready src={artW(source, 342)} alt="" loading="lazy" decoding="async" width="342" height="513" onload={() => loaded = true} onerror={retryImage} />{/if}
    {#if c.rating}<span class="rate" class:low={c.rating < 7}>{c.rating.toFixed(1)}</span>{/if}
    {#if rank}<span class="rank">{rank}</span>{/if}
    {#if c.here}<span class="here">Есть у нас</span>{/if}
    {#if soon}<span class="soon">Скоро</span>{/if}
  </span>
  <span class="name">{c.title}</span>
  {#if c.year}<span class="sub">{soon ? `Скоро · ${c.year}` : c.genre ? `${c.year} · ${c.genre}` : c.year}</span>{/if}
{/snippet}

<div class="tile" class:rise={i !== null} style:--i={i}>
  {#if local}
    <a class="poster" href="/title/{local.id}" aria-label="{c.title}{c.year ? `, ${c.year}` : ''}">{@render body()}</a>
  {:else}
    <a class="poster" href="/ext/{card!.id}" onclick={() => remember(card!)} aria-label="{c.title}{c.year ? `, ${c.year}` : ''}">{@render body()}</a>
  {/if}
  {#if marks}<QuickMarks card={local} ext={local ? null : card} kind={c.kind} title={c.title} />{/if}
</div>

<style>
  .tile {
    position: relative;
    min-width: 0;
  }
  .poster {
    display: grid;
    gap: 2px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .art {
    position: relative;
    display: block;
    aspect-ratio: 2 / 3;
    margin-bottom: 10px;
    border-radius: 18px;
    overflow: hidden;
    background: var(--surface);
    box-shadow: 0 14px 30px -16px #000;
    transition: transform 0.45s var(--ease);
  }
  /* Под мышью постер приподнимается, по краю загорается тонкое кольцо (прозрачность - её считает видеокарта). */
  .art::after {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 2;
    border-radius: inherit;
    box-shadow: inset 0 0 0 1.5px #ffffff66;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s;
  }
  .tile:hover .art,
  .poster:focus-visible .art {
    transform: translateY(-6px) scale(1.025);
  }
  .tile:hover .art::after,
  .poster:focus-visible .art::after {
    opacity: 1;
  }
  .poster:active .art {
    transform: scale(0.97);
    transition-duration: 0.12s;
  }
  .poster:focus-visible {
    outline: none;
  }
  .poster:focus-visible .art {
    outline: 3px solid #fff;
    outline-offset: 2px;
  }
  .art img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .ph {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 20px 14px;
    text-align: center;
    overflow: hidden;
    background: radial-gradient(ellipse at 50% 38%, #ffffff12, transparent 58%), linear-gradient(0deg,#050506a8,transparent 74%);
  }
  .ph::before { content: ''; position: absolute; inset: 14px; border: 1px solid #ffffff24; border-radius: 12px; pointer-events: none; }
  .ph i { font-size: 10px; font-style: normal; font-weight: 700; letter-spacing: .22em; color: #ffffffa0; }
  .ph b { max-width: 100%; font-size: clamp(15px, 1.4vw, 21px); line-height: 1.15; text-wrap: balance; overflow-wrap: anywhere; text-shadow: 0 2px 16px #000; }
  .ph small { font-size: 12px; color: #ffffffa0; }
  .ph > * { position: relative; z-index: 1; }
  .ranked .ph { bottom: 60px; }
  .rank {
    position: absolute;
    left: 8px;
    bottom: 6px;
    font-size: 52px;
    letter-spacing: -0.04em;
    font-weight: 800;
    line-height: 1;
    color: #fff;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.8);
  }
  .here {
    position: absolute;
    top: 8px;
    left: 8px;
    padding: 2px 8px;
    border-radius: 6px;
    background: #fff;
    color: #000;
    font-size: 11px;
    font-weight: 700;
  }
  .name,
  .sub {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name {
    font-size: 14px;
    font-weight: 700;
  }
  .sub {
    display: flex;
    gap: 8px;
    font-size: 13px;
    color: var(--muted);
  }
  .rate {
    position: absolute;
    top: 6px;
    left: 6px;
    padding: 2px 8px;
    border-radius: 8px;
    background: #3bb33b;
    color: #fff;
    font-size: 12px;
    font-weight: 700;
    line-height: 16px;
  }
  .rate.low {
    background: #7b7b7e;
  }
  .here {
    top: 32px;
  }
  .soon {
    position: absolute;
    right: 8px;
    bottom: 8px;
    padding: 3px 10px;
    border-radius: 99px;
    background: #fff;
    color: #0a0a0a;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.02em;
  }
</style>
