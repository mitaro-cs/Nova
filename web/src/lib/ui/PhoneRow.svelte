<script lang="ts">
  import type { Card, ExtCard } from '$lib/types';
  import PhonePoster from './PhonePoster.svelte';

  // Ряд постеров телефонного вида: заголовок 20 px, справа оранжевое «Все», постеры 148 px с отступом 8,
  // край ряда - боковое поле (первый постер у поля, последний уезжает за край).
  let { title, href = '', items }: { title: string; href?: string; items: { card?: Card; ext?: ExtCard }[] } = $props();
</script>

<section class="row">
  <div class="head">
    <h2>{title}</h2>
    {#if href}<a href={href}>Все</a>{/if}
  </div>
  <div class="strip">
    {#each items as it (it.card ? `t${it.card.id}` : `x${it.ext?.id}`)}<PhonePoster card={it.card} ext={it.ext} />{/each}
  </div>
</section>

<style>
  .row {
    margin-bottom: 32px;
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin: 0 var(--gutter) 14px;
  }
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }
  .head a {
    color: var(--m-accent);
    font-size: 16px;
    font-weight: 600;
  }
  .strip {
    display: flex;
    gap: 8px;
    padding: 0 var(--gutter);
    overflow-x: auto;
    scrollbar-width: none;
    overscroll-behavior-x: contain;
  }
  .strip::-webkit-scrollbar {
    display: none;
  }
</style>
