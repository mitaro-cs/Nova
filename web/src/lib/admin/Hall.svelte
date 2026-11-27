<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { catalog, loadCatalog } from '$lib/catalog.svelte';
  import { plural, runtime } from '$lib/fmt';
  import { badges } from '$lib/social/badges.svelte';
  import { artSet, artW, placeholder } from '$lib/ui/art';
  import Avatar from '$lib/ui/Avatar.svelte';
  import { fadeIn } from '$lib/ui/fade';
  import AttentionList from './AttentionList.svelte';
  import { href } from './nav';
  import { seatMap } from './seats';
  import type { Health, Metrics, Status } from './types';

  // Сводка «Зал» (выбор владельца 27.11, макет A1): сверху экран кинотеатра - на нём то, что смотрят прямо сейчас;
  // под ним места беты креслами (занято, давно не заходил, свободно; кто смотрит сейчас - с красным ободком).
  // Справа - сколько мест занято и кто смотрит (нажатие показывает его фильм на экране). Ниже - «Требует внимания»,
  // неделя и свежее в библиотеке.
  let { status, health, reload }: { status: Status; health: Health | null; reload: () => void } = $props();

  interface U { id: number; name: string; role: string; disabled: boolean; seenAt: number | null }
  let users = $state<U[] | null>(null);
  let metrics = $state<Metrics | null>(null);
  let pick = $state(0);

  function load() {
    api<U[]>('/api/admin/users').then((u) => (users = u), () => {});
    api<Metrics>(`/api/admin/metrics?tz=${-new Date().getTimezoneOffset()}`).then((m) => (metrics = m), () => {});
  }
  onMount(() => {
    load();
    loadCatalog();
    const fast = setInterval(reload, 5000);
    const slow = setInterval(load, 30000);
    return () => {
      clearInterval(fast);
      clearInterval(slow);
    };
  });

  const watching = $derived(status.watching);
  const live = $derived(watching[Math.min(pick, watching.length - 1)] ?? null);
  const fresh = $derived((catalog.items ?? []).filter((c) => c.poster).slice().sort((a, b) => b.addedAt - a.addedAt).slice(0, 3));
  const quiet = $derived(fresh.find((c) => c.backdrop) ?? fresh[0] ?? null);
  const shot = $derived(live ? (live.backdrop ?? live.poster ?? null) : (quiet?.backdrop ?? null));
  const pct = (w: Status['watching'][number]) => (w.duration > 0 ? Math.min(100, Math.round((w.position / w.duration) * 100)) : 0);
  const mins = (s: number) => runtime(Math.max(1, Math.round(s / 60)));
  const quality = (q: string | undefined) => (!q ? '' : q === 'orig' ? 'оригинал' : q === 'auto' ? 'авто' : q);

  const hall = $derived(users ? seatMap(users, status.seats ?? null, watching) : null);
  const seats = $derived(status.seats ?? null);
  const viewers = $derived(users?.filter((u) => u.role !== 'host').length ?? 0);

  const DOW = ['Вс', 'Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб'];
  const DOW_FULL = ['в воскресенье', 'в понедельник', 'во вторник', 'в среду', 'в четверг', 'в пятницу', 'в субботу'];
  const week = $derived.by(() => {
    const days = metrics?.activity.days.slice(-7) ?? [];
    const max = Math.max(1, ...days.map((d) => d.views));
    const top = days.reduce((a, d) => (d.views > (a?.views ?? 0) ? d : a), null as (typeof days)[number] | null);
    return { days: days.map((d) => ({ ...d, h: Math.max(6, Math.round((d.views / max) * 100)), dow: DOW[new Date(d.day).getDay()], top: d === top })), views: days.reduce((n, d) => n + d.views, 0), top };
  });
</script>

<div class="zal">
  <section class="hall" aria-label="Зал">
    <div class="stage">
      <div class="screen-w">
        <div class="screen" style:background={placeholder(live?.title ?? quiet?.title ?? 'Nova')}>
          {#key shot}
            {#if shot}<img use:fadeIn src={artW(shot, 1280)} srcset={artSet(shot)} sizes="(max-width: 900px) 100vw, 50vw" alt="" decoding="async" />{/if}
          {/key}
          <div class="cap">
            {#if live}
              <span class="onair"><i aria-hidden="true"></i>В эфире</span>
              <b>{live.title}</b>
              <small>{[live.user, live.episode, quality(live.net?.quality), live.duration ? `${mins(live.position)} из ${mins(live.duration)}` : ''].filter(Boolean).join(' · ')}</small>
            {:else}
              <span class="onair quiet"><i aria-hidden="true"></i>Сейчас тихо</span>
              <b>{quiet ? quiet.title : 'Никто не смотрит'}</b>
              <small>{quiet ? 'Последнее в библиотеке' : 'Когда кто-то включит фильм, он появится здесь'}</small>
            {/if}
          </div>
          {#if live}<div class="prog" aria-hidden="true"><i style:transform="scaleX({pct(live) / 100})"></i></div>{/if}
        </div>
      </div>
      <div class="glow" aria-hidden="true"></div>
      {#if hall}
        <div class="seats" class:wide={hall.cols > 10} role="img" aria-label="Места беты: занято {hall.used} из {hall.total}{hall.now ? `, смотрят сейчас ${hall.now}` : ''}">
          {#each hall.rows as row, r (r)}
            <div class="srow">
              <span class="rn" aria-hidden="true">{r + 1}</span>
              {#each row as s, c (c)}
                {#if c === hall.cols / 2}<span class="aisle" aria-hidden="true"></span>{/if}
                <span class="seat {s.state}" data-n={s.label} aria-hidden="true"></span>
              {/each}
              <span class="rn" aria-hidden="true">{r + 1}</span>
            </div>
          {/each}
        </div>
        {#if hall.hidden}<p class="more">И ещё {hall.hidden} {plural(hall.hidden, 'место', 'места', 'мест')} - зал показывает первые {hall.shown}</p>{/if}
        <ul class="legend" aria-hidden="true">
          <li><i class="seat now"></i>смотрит сейчас</li>
          <li><i class="seat taken"></i>занято</li>
          <li><i class="seat idle"></i>давно не заходил</li>
          <li><i class="seat free"></i>свободно</li>
        </ul>
      {:else}
        <div class="seats-wait" aria-hidden="true"></div>
      {/if}
    </div>

    <aside class="side">
      <div>
        <span class="lab">{seats ? 'Места беты' : 'Зрители'}</span>
        <div class="big"><b>{seats ? seats.used : viewers}</b>{#if seats}<small>/{seats.total}</small>{/if}</div>
        <p class="note">
          {#if !seats}Без лимита мест - регистрация {users ? 'по правилам раздела «Регистрация»' : ''}
          {:else if seats.left > 0}Осталось {seats.left} {plural(seats.left, 'место', 'места', 'мест')}. Когда займут все, регистрация закроется сама.
          {:else}Все места заняты - регистрация закрыта. Освободить место можно, удалив аккаунт.{/if}
        </p>
        <a class="btn sm" href={href('viewers', 'signup')} data-sveltekit-replacestate data-sveltekit-noscroll>Правила входа</a>
      </div>
      <div>
        <span class="lab">Смотрят сейчас</span>
        {#if watching.length}
          <ul class="watch">
            {#each watching as w, i (i)}
              <li>
                <button type="button" aria-pressed={live === w} onclick={() => (pick = i)}>
                  <Avatar avatar={w.avatar ?? 0} photo={w.photo ?? null} size={32} />
                  <span class="who"><b>{w.user}</b><small>{w.episode ? `${w.title}, ${w.episode}` : w.title}</small></span>
                  <span class="pct">{pct(w)}%</span>
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="note">Сейчас никто не смотрит</p>
        {/if}
      </div>
    </aside>
  </section>

  <div class="low">
    <AttentionList {status} {health} wishes={badges.wishes} {reload} />
    <div class="pair">
      <a class="tile" href={href('summary', 'stats')} data-sveltekit-replacestate data-sveltekit-noscroll>
        <h4>Неделя</h4>
        <div class="spark" aria-hidden="true">{#each week.days as d (d.day)}<i class:hi={d.top} style:transform="scaleY({d.h / 100})"></i>{/each}</div>
        <div class="days" aria-hidden="true">{#each week.days as d (d.day)}<span>{d.dow}</span>{/each}</div>
        <small>{#if metrics}{week.views} {plural(week.views, 'просмотр', 'просмотра', 'просмотров')}{week.top ? ` · пик ${DOW_FULL[new Date(week.top.day).getDay()]}` : ''}{:else}Считаем…{/if}</small>
      </a>
      <a class="tile" href={href('library', 'catalog')} data-sveltekit-replacestate data-sveltekit-noscroll>
        <h4>Свежее</h4>
        <div class="posters" aria-hidden="true">{#each fresh as c (c.id)}<img src={artW(c.poster, 185)} alt="" loading="lazy" decoding="async" />{/each}</div>
        <small>{status.titles} {plural(status.titles, 'фильм и сериал', 'фильма и сериала', 'фильмов и сериалов')}</small>
      </a>
    </div>
  </div>
</div>

<style>
  .zal { display: grid; gap: 16px; }
  .lab { display: block; font-size: 11.5px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; color: var(--faint, #71717a); }

  .hall {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px;
    gap: 24px;
    padding: 24px;
    border: 1px solid var(--line);
    border-radius: 28px;
    background: radial-gradient(120% 70% at 50% 0%, #1b1b1f 0%, #111113 60%);
  }
  .stage { min-width: 0; }
  /* Экран кинотеатра: кадр чуть наклонён от зрителя, под ним мягкий свет на зал. */
  .screen-w { perspective: 900px; margin: 0 6% 6px; }
  .screen {
    position: relative;
    isolation: isolate;
    aspect-ratio: 2.39 / 1;
    max-width: 100%;
    border-radius: 8px;
    overflow: hidden;
    transform: rotateX(12deg);
    transform-origin: 50% 0;
    box-shadow: 0 30px 60px -24px rgb(255 255 255 / 0.14);
  }
  .screen img { position: absolute; inset: 0; width: 100%; max-width: none; height: 100%; object-fit: cover; z-index: -1; }
  .screen::after { content: ''; position: absolute; inset: 0; z-index: -1; background: linear-gradient(0deg, #000000c7, #0000 60%); }
  .cap { position: absolute; left: 18px; right: 18px; bottom: 16px; display: grid; gap: 4px; justify-items: start; min-width: 0; }
  .cap b { font-size: clamp(18px, 2.2vw, 28px); font-weight: 800; letter-spacing: -0.02em; line-height: 1.1; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cap small { color: #d1d1d6; font-size: 12.5px; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .onair { display: inline-flex; align-items: center; gap: 7px; padding: 4px 10px; border-radius: 99px; background: #0009; font-size: 11px; font-weight: 800; letter-spacing: 0.09em; text-transform: uppercase; }
  .onair i { width: 7px; height: 7px; border-radius: 50%; background: #ff3b30; box-shadow: 0 0 0 3px #ff3b304d; animation: pulse 1.6s ease-in-out infinite; }
  .onair.quiet i { background: #8e8e93; box-shadow: none; animation: none; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  .prog { position: absolute; left: 0; right: 0; bottom: 0; height: 3px; background: #ffffff26; }
  .prog i { display: block; height: 100%; background: #fff; transform-origin: 0 50%; transition: transform 0.6s var(--ease); }
  .glow { height: 22px; margin: -2px 10% 10px; background: radial-gradient(50% 100% at 50% 0%, #ffffff1a, #0000 70%); }

  /* Кресла: ряды по 10 (больше 100 мест - по 20) с проходом посередине. */
  .seats { display: grid; gap: 7px; justify-content: center; }
  .srow { display: flex; gap: 6px; align-items: center; }
  .rn { width: 16px; font-size: 10px; color: #5a5a5f; text-align: center; font-variant-numeric: tabular-nums; }
  .aisle { width: 18px; }
  .seat { position: relative; width: 26px; height: 22px; border-radius: 7px 7px 4px 4px; background: #26262a; transition: transform 0.2s var(--ease); }
  .seats.wide .seat { width: 15px; height: 13px; border-radius: 4px 4px 2px 2px; }
  .seats.wide .srow { gap: 4px; }
  .seat.taken { background: #d9d6ce; }
  .seat.idle { background: #6d6b66; }
  .seat.off { background: #3a3a3c; }
  .seat.now { background: #fff; box-shadow: 0 0 0 3px #ff3b30e6; }
  .seat.free { background: transparent; box-shadow: inset 0 0 0 1.5px #ffffff2e; }
  .seat:hover { transform: translateY(-2px); }
  .seat:hover::before {
    content: attr(data-n);
    position: absolute;
    bottom: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
    padding: 4px 8px;
    border-radius: 8px;
    background: #fff;
    color: #000;
    font-size: 11px;
    font-weight: 700;
    white-space: nowrap;
    z-index: 2;
    pointer-events: none;
  }
  .seats-wait { height: 300px; border-radius: 16px; background: #ffffff08; animation: wait 1.4s ease-in-out infinite; }
  @keyframes wait { 50% { opacity: 0.5; } }
  .more { margin: 10px 0 0; text-align: center; color: var(--muted); font-size: 12.5px; }
  .legend { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px 16px; margin: 16px 0 0; padding: 0; list-style: none; font-size: 12px; color: var(--muted); }
  .legend li { display: inline-flex; align-items: center; gap: 6px; }
  .legend .seat { width: 14px; height: 12px; border-radius: 4px 4px 2px 2px; }
  .legend .seat:hover { transform: none; }
  .legend .seat:hover::before { content: none; }

  .side { display: flex; flex-direction: column; gap: 22px; padding-left: 22px; border-left: 1px solid var(--line); min-width: 0; }
  .big { display: flex; align-items: baseline; margin: 8px 0 6px; font-variant-numeric: tabular-nums; }
  .big b { font-size: 64px; font-weight: 900; letter-spacing: -0.05em; line-height: 0.95; }
  .big small { font-size: 20px; font-weight: 700; color: var(--muted); }
  .note { margin: 0 0 12px; color: var(--muted); font-size: 13px; line-height: 1.45; }
  .watch { display: grid; gap: 4px; margin: 8px 0 0; padding: 0; list-style: none; }
  .watch button {
    display: grid;
    grid-template-columns: 32px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    width: 100%;
    padding: 6px;
    border: 0;
    border-radius: 14px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 0.2s;
  }
  .watch button:hover { background: #ffffff0d; }
  .watch button[aria-pressed='true'] { background: #ffffff17; }
  .who { display: grid; min-width: 0; }
  .who b, .who small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .who small { color: var(--muted); font-size: 12px; }
  .pct { padding: 3px 7px; border-radius: 99px; background: #000; font-size: 11px; font-weight: 800; font-variant-numeric: tabular-nums; }

  .low { display: grid; grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr); gap: 16px; align-items: start; }
  .pair { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 16px; }
  .tile { display: grid; gap: 10px; align-content: start; min-width: 0; padding: 18px; border: 1px solid var(--line); border-radius: 24px; background: #141416; color: inherit; transition: transform 0.3s var(--ease), border-color 0.3s; }
  .tile:hover { transform: translateY(-2px); border-color: #ffffff29; }
  .tile h4 { margin: 0; font-size: 15px; }
  .tile small { color: var(--muted); font-size: 12.5px; }
  .spark { display: flex; align-items: flex-end; gap: 5px; height: 72px; }
  .spark i { flex: 1; height: 100%; border-radius: 6px; background: #ffffff1f; transform-origin: 50% 100%; }
  .spark i.hi { background: #fff; }
  .days { display: flex; gap: 5px; margin-top: -4px; }
  .days span { flex: 1; text-align: center; font-size: 10.5px; color: var(--faint, #71717a); }
  .posters { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
  .posters img { width: 100%; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 10px; }

  @media (max-width: 1180px) {
    .hall { grid-template-columns: minmax(0, 1fr); }
    .side { padding: 18px 0 0; border-left: 0; border-top: 1px solid var(--line); display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }
    .low { grid-template-columns: minmax(0, 1fr); }
  }
  @media (max-width: 640px) {
    .hall { padding: 16px; border-radius: 22px; }
    .screen-w { margin: 0 0 6px; }
    .cap small { display: none; }
    .side { grid-template-columns: minmax(0, 1fr); }
    .seats { gap: 5px; }
    .srow { gap: 3px; }
    .seat { width: 15px; height: 13px; border-radius: 4px 4px 2px 2px; }
    .seats.wide .seat { width: 7px; height: 7px; border-radius: 2px; }
    .seats.wide .srow { gap: 2px; }
    .aisle { width: 8px; }
    .rn { display: none; }
  }
  @media (max-width: 420px) {
    .pair { grid-template-columns: minmax(0, 1fr); }
  }
</style>
