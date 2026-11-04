<script lang="ts">
  import type { VisLevel } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';

  // «Кто видит этот раздел»: все, друзья или только я. Показывается только владельцу под вкладкой.
  let { level, label = 'Кто видит этот раздел', pick }: { level: VisLevel; label?: string; pick: (v: VisLevel) => void } = $props();
  const LEVELS: [VisLevel, string][] = [
    ['all', 'Все'],
    ['friends', 'Друзья'],
    ['me', 'Только я'],
  ];
</script>

<div class="pv">
  <span class="l"><Icon name="eye" size={16} /> {label}</span>
  <span class="seg" role="radiogroup" aria-label={label}>
    {#each LEVELS as [v, text] (v)}
      <button type="button" role="radio" aria-checked={level === v} class:on={level === v} onclick={() => pick(v)}>{text}</button>
    {/each}
  </span>
</div>

<style>
  .pv {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
    padding: 14px 0 0;
    color: var(--muted);
    font-size: 13px;
  }
  .l {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .seg {
    display: inline-flex;
    max-width: 100%;
    padding: 2px;
    gap: 2px;
    border: 1px solid var(--line-strong, #ffffff40);
    border-radius: 999px;
  }
  button {
    padding: 4px 13px;
    border: 0;
    background: none;
    border-radius: 999px;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background 0.25s,
      color 0.25s;
  }
  button.on {
    background: #fff;
    color: #0a0a0a;
  }
  button:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }
</style>
