<script lang="ts">
  import { api } from '$lib/api';
  import { trailersOff } from '$lib/trailers';
  import type { Card, ExtCard } from '$lib/types';
  import type { Component } from 'svelte';
  import Icon from './Icon.svelte';

  // Трейлер героя главной на телефоне: официальный HD с IMDb без звука поверх кадра, кнопка звука - круг 44
  // слева вверху. Вынесено из `Showcase`, чтобы главная не тащила это на компьютере (бюджет JS 80 КБ).
  let { card, ext }: { card?: Card; ext?: ExtCard } = $props();

  let TB = $state<Component<any> | null>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let tb = $state<any>();
  let available = $state(false);
  let muted = $state(true);
  // У карточки с сервиса id IMDb в списке нет - берём из подробностей (один лёгкий запрос на кадр).
  let imdbOf = $state<Record<number, string | null>>({});

  $effect(() => {
    if (!trailersOff()) void import('$lib/title/TrailerBanner.svelte').then((m) => (TB = m.default));
  });
  $effect(() => {
    const c = card;
    if (!c || c.id in imdbOf) return;
    api<{ imdbId: string | null }>(`/api/titles/${c.id}`).then(
      (d) => (imdbOf[c.id] = d.imdbId),
      () => (imdbOf[c.id] = null),
    );
  });
  const id = $derived(card ? (imdbOf[card.id] ?? null) : (ext?.id ?? null));
</script>

{#if id && TB}
  <div class="layer" aria-hidden="true">{#key id}<TB imdbId={id} bare bind:available bind:muted bind:this={tb} />{/key}</div>
  {#if available}
    <button class="sound" aria-label={muted ? 'Включить звук трейлера' : 'Выключить звук трейлера'} onclick={(e) => { e.stopPropagation(); void tb?.toggleSound(); }}><Icon name={muted ? 'mute' : 'volume'} size={22} /></button>
  {/if}
{/if}

<style>
  .layer { position: absolute; inset: 0; z-index: -1; }
  .sound { position: absolute; top: 12px; left: 12px; z-index: 2; display: grid; place-items: center; width: 44px; height: 44px; padding: 0; border: 0; border-radius: 50%; background: #1e1e1ed9; color: #fff; cursor: pointer; }
</style>
