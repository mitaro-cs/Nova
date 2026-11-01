<script module lang="ts">
  import type { Component } from 'svelte';

  // Трейлер грузится лениво и один раз на всех (бюджет JS страниц, hls.js внутри - ещё лениво).
  let banner: Promise<Component<any>> | null = null;
  const loadBanner = () => (banner ??= import('$lib/title/TrailerBanner.svelte').then((m) => m.default));
</script>

<script lang="ts">
  import { api } from '$lib/api';
  import { trailersOff } from '$lib/trailers';
  import { remember } from '$lib/discover/sheet.svelte';
  import { localized } from '$lib/localized.svelte';
  import { genreRu } from '$lib/genres';
  import { setMark, setStatus, toggle, toggleMark } from '$lib/marks';
  import type { Card, ExtCard } from '$lib/types';
  import { artW } from './art';
  import Icon from './Icon.svelte';

  // Карточка ленты «Что смотреть» (телефон, как у Кинопоиска): кадр 16:9, на нём - трейлер без звука, пока
  // карточка занимает экран; кнопка звука - круг 32 справа внизу. Под кадром название, описание в три
  // строки, «…» (уже смотрел / не интересно) и «+» в планы. Играет одна карточка за раз (`active`).
  let {
    card,
    ext,
    active = false,
    onseen,
    ongone,
  }: { card?: Card; ext?: ExtCard; active?: boolean; onseen: () => void; ongone: () => void } = $props();

  const c = $derived(card ?? ext!);
  const href = $derived(card ? `/title/${card.id}` : `/ext/${ext!.id}`);
  const wanted = $derived(card ? card.status === 'want' : ext?.mark === 'want');
  const shot = $derived(card ? (card.backdropSmall ?? card.backdrop ?? card.poster) : ext!.poster);
  const text = $derived(
    localized(card?.overview) || [c.year, c.genres.slice(0, 2).map(genreRu).join(', ')].filter(Boolean).join(' · '),
  );

  let root = $state<HTMLElement>();
  let TB = $state<Component<any> | null>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let tb = $state<any>();
  let available = $state(false);
  let muted = $state(true);
  let menu = $state(false);
  let imdb = $state<string | null>(null);

  $effect(() => {
    if (!root) return;
    const io = new IntersectionObserver(([e]) => e.isIntersecting && onseen(), { threshold: 0.6 });
    io.observe(root);
    return () => io.disconnect();
  });

  // Нужен id IMDb: у фильма из ленты это его id, у своего - из подробностей (один лёгкий запрос).
  $effect(() => {
    if (!active || trailersOff()) return;
    let live = true;
    void loadBanner().then((b) => live && (TB = b));
    if (ext) imdb = ext.id;
    else if (card)
      api<{ imdbId: string | null }>(`/api/titles/${card.id}`).then(
        (d) => live && (imdb = d.imdbId),
        () => {},
      );
    return () => (live = false);
  });

  function plan() {
    if (card) void toggle(card, 'want');
    else if (ext) void toggleMark(ext, 'want');
  }
  function done(kind: 'watched' | 'dropped') {
    menu = false;
    if (card) void setStatus(card, kind);
    else if (ext) void setMark(ext, kind);
    ongone();
  }
</script>

<article class="fc" bind:this={root}>
  <div class="shot">
    <a class="cover" {href} onclick={() => ext && remember(ext)} aria-label={c.title}>
      {#if shot}<img src={artW(shot, 640)} alt="" loading="lazy" decoding="async" />{/if}
      {#if active && TB && imdb}{#key imdb}<TB imdbId={imdb} bare bind:available bind:muted bind:this={tb} />{/key}{/if}
    </a>
    {#if active && available}
      <button class="sound" aria-label={muted ? 'Включить звук трейлера' : 'Выключить звук трейлера'} onclick={() => void tb?.toggleSound()}><Icon name={muted ? 'mute' : 'volume'} size={18} /></button>
    {/if}
  </div>
  <div class="info">
    <h2><a {href} onclick={() => ext && remember(ext)}>{c.title}</a></h2>
    {#if text}<p>{text}</p>{/if}
    <div class="act">
      <div class="more">
        <button class="dots" aria-label="Ещё" aria-expanded={menu} onclick={() => (menu = !menu)}><Icon name="dots" size={22} /></button>
        {#if menu}
          <div class="pop" role="menu">
            <button role="menuitem" onclick={() => done('watched')}>Уже смотрел</button>
            <button role="menuitem" onclick={() => done('dropped')}>Не интересно</button>
          </div>
        {/if}
      </div>
      <button class="add" class:on={wanted} aria-pressed={wanted} aria-label={wanted ? 'Убрать из планов' : 'Хочу посмотреть'} onclick={plan}><Icon name={wanted ? 'check' : 'bookmark'} size={20} /></button>
    </div>
  </div>
</article>

<style>
  .fc {
    border-radius: 0;
    background: #0f0f0f;
  }
  .shot {
    position: relative;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    background: #1c1c1e;
  }
  .cover {
    position: absolute;
    inset: 0;
    display: block;
  }
  .cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 30%;
  }
  .sound {
    position: absolute;
    right: 13px;
    bottom: 13px;
    z-index: 2;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: #1e1e1ed9;
    color: #fff;
    cursor: pointer;
  }
  .info {
    padding: 16px;
  }
  h2 {
    margin: 0 0 8px;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }
  h2 a {
    color: #fff;
  }
  p {
    display: -webkit-box;
    min-height: 3.9em;
    margin: 0;
    overflow: hidden;
    color: #b5b5ba;
    font-size: 15px;
    line-height: 1.3;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
  }
  .act {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 20px;
    margin-top: 14px;
  }
  .more {
    position: relative;
  }
  .dots,
  .add {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: none;
    color: #fff;
    cursor: pointer;
  }
  .add {
    background: #2a2a2a;
  }
  .add.on {
    color: var(--m-accent);
  }
  .pop {
    position: absolute;
    right: 0;
    bottom: 46px;
    z-index: 3;
    display: grid;
    min-width: 170px;
    padding: 6px;
    border-radius: 12px;
    background: #262628;
    box-shadow: 0 10px 30px #000a;
  }
  .pop button {
    padding: 10px 12px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: #fff;
    font: inherit;
    font-size: 15px;
    text-align: left;
    cursor: pointer;
  }
  .pop button:hover {
    background: #ffffff14;
  }
</style>

