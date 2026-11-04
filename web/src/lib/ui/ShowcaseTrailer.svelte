<script lang="ts">
  import { api } from '$lib/api';
  import { trailersOff } from '$lib/trailers';
  import type { Card, ExtCard } from '$lib/types';
  import type { Component } from 'svelte';
  import Icon from './Icon.svelte';

  // Трейлер героя: официальный HD с IMDb без звука поверх кадра, кнопка звука - круг 44 слева вверху.
  // Вынесено из `Showcase` и грузится лениво: бюджет JS главной 80 КБ.
  // active - наружу: трейлер готов и играет (карусель тогда убирает лишний текст в угол).
  let { card, ext, active = $bindable(false) }: { card?: Card; ext?: ExtCard; active?: boolean } = $props();

  let TB = $state<Component<any> | null>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let tb = $state<any>();
  let available = $state(false);
  let muted = $state(true);
  // Идёт ли картинка трейлера на самом деле (не просто готов к показу): текст уходит в угол только в этот миг.
  let playing = $state(false);
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
  $effect(() => {
    active = available && playing;
    return () => {
      active = false;
    };
  });
</script>

{#if id && TB}
  <div class="layer" aria-hidden="true">{#key id}<TB imdbId={id} bare bind:available bind:muted bind:playing bind:this={tb} />{/key}</div>
  {#if available}
    <button class="sound" aria-label={muted ? 'Включить звук трейлера' : 'Выключить звук трейлера'} onclick={(e) => { e.stopPropagation(); void tb?.toggleSound(); }}><Icon name={muted ? 'mute' : 'volume'} size={22} /></button>
  {/if}
{/if}

<style>
  .layer { position: absolute; inset: 0; z-index: -1; }
  .sound { position: absolute; top: 16px; left: 16px; z-index: 2; display: grid; place-items: center; width: 44px; height: 44px; padding: 0; border: 0; border-radius: 50%; background: #1e1e1ed9; color: #fff; cursor: pointer; outline: none; transition: transform .25s cubic-bezier(.22, 1, .36, 1), background .2s; }
  .sound:hover { background: #2a2a2cf0; transform: scale(1.08); }
  .sound:active { transform: scale(.92); }
</style>
