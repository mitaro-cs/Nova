<script lang="ts">
  import { remember } from '$lib/discover/sheet.svelte';
  import type { Card, ExtCard } from '$lib/types';
  import { artW } from './art';

  // Постер телефонного вида (как «Буду смотреть» у Кинопоиска): 148 × 222, оценка зелёной плашкой в левом
  // углу (от 7 - зелёная, ниже - серая), название под постером в две строки. Фильм с сервиса - на
  // `/title/<id>`, из ленты IMDb - на `/ext/<id>` (карточка запоминается - страница видна сразу).
  let { card, ext }: { card?: Card; ext?: ExtCard } = $props();
  const c = $derived(card ?? ext!);
  const href = $derived(card ? `/title/${card.id}` : `/ext/${ext!.id}`);
  let failed = $state(false);
</script>

<a class="p" {href} onclick={() => ext && remember(ext)} aria-label="{c.title}{c.year ? `, ${c.year}` : ''}">
  <span class="art">
    {#if c.poster && !failed}<img src={artW(c.poster, 342)} alt="" loading="lazy" decoding="async" width="148" height="222" onerror={() => (failed = true)} />{:else}<span class="ph">{c.title}</span>{/if}
    {#if c.rating}<span class="rate" class:low={c.rating < 7}>{c.rating.toFixed(1)}</span>{/if}
  </span>
  <span class="name">{c.title}</span>
</a>

<style>
  .p {
    flex: none;
    display: block;
    width: 148px;
    color: #fff;
  }
  .art {
    position: relative;
    display: block;
    aspect-ratio: 2 / 3;
    overflow: hidden;
    border-radius: 4px;
    background: #1c1c1e;
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
    inset: 12px;
    display: grid;
    place-items: center;
    text-align: center;
    font-weight: 700;
    overflow: hidden;
  }
  .rate {
    position: absolute;
    top: 6px;
    left: 6px;
    padding: 2px 6px;
    border-radius: 2px;
    background: #3bb33b;
    color: #fff;
    font-size: 12px;
    font-weight: 700;
    line-height: 16px;
  }
  .rate.low {
    background: #7b7b7e;
  }
  .name {
    display: -webkit-box;
    margin-top: 10px;
    overflow: hidden;
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
</style>
