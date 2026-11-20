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
  import { getDetail, peekDetail } from '$lib/discover/details';
  import { localized } from '$lib/localized.svelte';
  import { genreRu } from '$lib/genres';
  import { setMark, setStatus, toggle, toggleMark } from '$lib/marks';
  import type { Card, ExtCard, ExtDetail } from '$lib/types';
  import { artW, heroSet, placeholder } from './art';
  import Icon from './Icon.svelte';

  // Карточка ленты «Что смотреть»: кадр из фильма 16:9, на нём - трейлер без звука, пока карточка занимает
  // экран; кнопка звука - круг 32 справа внизу. Рядом краткая информация: название, оценка, год, жанры,
  // длительность и описание в три строки; «…» (уже смотрел / не интересно) и закладка в планы. Играет одна
  // карточка за раз (`active`). У фильма из ленты кадр и описание подгружаются, когда карточка подходит к экрану.
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

  let root = $state<HTMLElement>();
  let near = $state(false);
  let detail = $state<ExtDetail | null>(null);
  // Описание сбрасываем только при смене фильма, а не при каждом обновлении ленты (отметка «Хочу» перезагружает
  // ленту новыми объектами - карточка на секунду теряла кадр и «пропадала»).
  const extId = $derived(ext?.id ?? null);
  $effect(() => {
    const id = extId;
    detail = id ? (peekDetail(id) ?? (detail && detail.card.id === id ? detail : null)) : null;
  });
  $effect(() => {
    if (!extId || !near || detail) return;
    let live = true;
    getDetail(extId).then(
      (d) => live && (detail = d),
      () => {},
    );
    return () => (live = false);
  });

  const big = $derived(card ? (card.backdrop ?? null) : (detail?.backdrop ?? detail?.gallery?.[0] ?? null));
  const small = $derived(card?.backdropSmall ?? null);
  // Кадра ещё нет (у фильма из ленты - пока не пришли подробности): показываем постер целиком, не обрезая.
  const shot = $derived(big ?? small ?? c.poster ?? null);
  const framed = $derived(!!(big ?? small));
  // Без кадра из фильма карточку не показываем (постер посреди широкой рамки смотрелся плохо): своя - сразу,
  // из ленты - когда описание загрузилось и кадра в нём нет.
  let gone = false;
  $effect(() => {
    if (gone) return;
    const noFrame = card ? !card.backdrop && !card.backdropSmall : !!detail && !detail.refreshing && !framed;
    if (noFrame) {
      gone = true;
      ongone();
    }
  });
  const overview = $derived(localized(card?.overview ?? detail?.overview));
  const minutes = $derived(card?.runtime ?? detail?.runtime ?? null);
  const facts = $derived(
    [c.year, c.genres.slice(0, 2).map(genreRu).join(', '), minutes ? `${Math.floor(minutes / 60) ? `${Math.floor(minutes / 60)} ч ` : ''}${minutes % 60 ? `${minutes % 60} мин` : ''}`.trim() : '']
      .filter(Boolean)
      .join(' · '),
  );

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
    const preload = new IntersectionObserver(([e]) => e.isIntersecting && (near = true), { rootMargin: '100% 0px' });
    preload.observe(root);
    return () => {
      io.disconnect();
      preload.disconnect();
    };
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
      <span class="cover-fallback" style:background={placeholder(c.title)} aria-hidden="true"><i>{c.kind==='series'?'СЕРИАЛ':'ФИЛЬМ'}</i><b>{c.title}</b>{#if c.year}<small>{c.year}</small>{/if}</span>
      {#if shot}<img class:poster={!framed} src={artW(shot, 640)} srcset={big ? heroSet(big, small) : undefined} sizes="(min-width: 1001px) 60vw, 100vw" alt="" loading="lazy" decoding="async" onerror={(e)=>{const img=e.currentTarget as HTMLImageElement;const fallback=c.poster?artW(c.poster,640):undefined;if(fallback&&img.dataset.posterFallback!=='1'&&img.currentSrc!==fallback){img.dataset.posterFallback='1';img.removeAttribute('srcset');img.src=fallback;}else img.style.display='none';}} />{/if}
      {#if active && TB && imdb}{#key imdb}<TB imdbId={imdb} bare bind:available bind:muted bind:this={tb} />{/key}{/if}
    </a>
    {#if active && available}
      <button class="sound" aria-label={muted ? 'Включить звук трейлера' : 'Выключить звук трейлера'} onclick={() => void tb?.toggleSound()}><Icon name={muted ? 'mute' : 'volume'} size={18} /></button>
    {/if}
  </div>
  <div class="info">
    <h2><a {href} onclick={() => ext && remember(ext)}>{c.title}</a></h2>
    <p class="facts">
      {#if c.rating}<span class="rate" class:low={c.rating < 7}>{c.rating.toFixed(1)}</span>{/if}
      {#if facts}<span>{facts}</span>{/if}
    </p>
    <p class="about">{overview}</p>
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
  .cover-fallback { position:absolute; inset:0; z-index:0; display:grid; place-content:center; justify-items:center; gap:10px; padding:24px; text-align:center; background-image:radial-gradient(ellipse at 50% 35%,#ffffff18,transparent 60%),linear-gradient(0deg,#050506bb,transparent 85%); }
  .cover-fallback i { font-size:10px; font-style:normal; font-weight:700; letter-spacing:.22em; color:#ffffffa8; }
  .cover-fallback b { max-width:18ch; font-size:clamp(20px,3vw,42px); line-height:1.1; text-wrap:balance; text-shadow:0 3px 18px #000; }
  .cover-fallback small { font-size:12px; color:#ffffffb5; }
  .cover img {
    position: absolute;
    z-index: 1;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 30%;
  }
  /* Пока кадра нет - постер целиком по центру, а не обрезанная полоска. */
  .cover img.poster {
    object-fit: contain;
    object-position: center;
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
  .facts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    margin: 0 0 10px;
    color: #b5b5ba;
    font-size: 14px;
    line-height: 1.3;
    /* Место под одну строку занято всегда: когда длительность придёт позже, ничего не прыгает. */
    min-height: 22px;
  }
  .rate {
    padding: 1px 6px;
    border-radius: 3px;
    background: #3bb33b;
    color: #fff;
    font-size: 13px;
    font-weight: 700;
    line-height: 18px;
  }
  .rate.low {
    background: #7b7b7e;
  }
  .about {
    display: -webkit-box;
    min-height: 3.9em;
    margin: 0;
    overflow: hidden;
    color: #d8d8dc;
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
    background: #fff;
    color: #0a0a0a;
  }
  /* Широкий экран: одна карточка на всю ширину, кадр слева, краткая информация справа. */
  @media (min-width: 1001px) {
    .fc {
      display: grid;
      grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
      align-items: center;
      gap: 32px;
      max-height: calc(100svh - var(--header) - 48px);
    }
    .shot {
      max-height: calc(100svh - var(--header) - 48px);
    }
    .info {
      padding: 24px 32px 24px 0;
    }
    h2 {
      font-size: clamp(26px, 2.6vw, 44px);
    }
    .facts {
      font-size: clamp(15px, 1.2vw, 20px);
    }
    .about {
      font-size: clamp(16px, 1.3vw, 22px);
      -webkit-line-clamp: 6;
      line-clamp: 6;
    }
    .act {
      justify-content: flex-start;
    }
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
