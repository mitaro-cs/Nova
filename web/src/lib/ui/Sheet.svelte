<script lang="ts">
  import type { Snippet } from 'svelte';

  // Страница в языке Rewind: заголовок и подзаголовок на чёрном, ниже - серая плита с разделами
  // (`.rw-block` и `.rw-h2` из rewind.css). `actions` - кнопки справа от заголовка, `above` - то, что
  // лежит между заголовком и плитой (вкладки, обложка профиля).
  let {
    title,
    subtitle = '',
    actions,
    above,
    children,
    label,
  }: {
    title: string;
    subtitle?: string;
    actions?: Snippet;
    above?: Snippet;
    children: Snippet;
    /** Название области для скринридера, если не совпадает с заголовком. */
    label?: string;
  } = $props();
</script>

<div class="rw-page">
  <header class="rw-head">
    <div>
      <h1>{title}</h1>
      {#if subtitle}<p>{subtitle}</p>{/if}
    </div>
    {#if actions}<div class="rw-actions">{@render actions()}</div>{/if}
  </header>
  {#if above}{@render above()}{/if}
  <section class="rw-plate" aria-label={label ?? title}>
    {@render children()}
  </section>
</div>

<style>
  .rw-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
