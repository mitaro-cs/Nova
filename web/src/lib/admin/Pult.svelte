<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { catalog, loadCatalog } from '$lib/catalog.svelte';
  import { plural, runtime, size } from '$lib/fmt';
  import { badges } from '$lib/social/badges.svelte';
  import { artSet, artW, placeholder } from '$lib/ui/art';
  import Avatar from '$lib/ui/Avatar.svelte';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import type { Health, Metrics, Status } from './types';

  // «Пульт · Прямой эфир» (выбор владельца 26.11): сверху - то, что смотрят прямо сейчас: кадр фильма, кто смотрит,
  // качество и где остановился, с красной точкой «в эфире» (красный здесь - знак эфира, как у трансляций). Справа -
  // все зрители (нажатие показывает его фильм в кадре), действия и огоньки состояния. Ниже четыре живые плитки:
  // диск кольцом, неделя столбиками, источники видео огоньками, свежее в библиотеке постерами. Каждая плитка ведёт в
  // свой раздел. Никто не смотрит - в кадре последнее добавленное, подпись «Сейчас тихо».
  let { status, reload }: { status: Status; reload: () => void } = $props();

  let health = $state<Health | null>(null);
  let metrics = $state<Metrics | null>(null);
  let pick = $state(0);

  function load() {
    api<Health>('/api/admin/health').then((h) => (health = h), () => {});
    api<Metrics>(`/api/admin/metrics?tz=${-new Date().getTimezoneOffset()}`).then((m) => (metrics = m), () => {});
  }
  onMount(() => {
    load();
    loadCatalog();
    const fast = setInterval(reload, 5000);
    const slow = setInterval(load, 20000);
    return () => {
      clearInterval(fast);
      clearInterval(slow);
    };
  });

  const watching = $derived(status.watching);
  const live = $derived(watching[Math.min(pick, watching.length - 1)] ?? null);
  const fresh = $derived(
    (catalog.items ?? [])
      .filter((c) => c.poster)
      .slice().sort((a, b) => b.addedAt - a.addedAt)
      .slice(0, 5),
  );
  const quiet = $derived(fresh.find((c) => c.backdrop) ?? fresh[0] ?? null);
  const shot = $derived(live ? (live.backdrop ?? live.poster ?? null) : (quiet?.backdrop ?? null));
  const pct = (w: Status['watching'][number]) => (w.duration > 0 ? Math.min(100, Math.round((w.position / w.duration) * 100)) : 0);
  const mins = (s: number) => runtime(Math.max(1, Math.round(s / 60)));
  const quality = (q: string | undefined) => (!q ? '' : q === 'orig' ? 'оригинал' : q === 'auto' ? 'авто' : q);

  // Огоньки: зелёный - в порядке, жёлтый - стоит заглянуть, красный - не работает.
  const sourcesDown = $derived(health?.sources.filter((s) => s.state === 'down').length ?? 0);
  const lowDisk = $derived(status.system?.diskFree != null && status.system.diskFree < 10 * 1024 ** 3);
  const lights = $derived([
    { label: 'Сервер', state: 'ok', href: '?tab=overview' },
    { label: status.publicUrl ? 'Интернет' : 'Только дом', state: status.publicUrl ? 'ok' : 'wait', href: '?tab=access' },
    { label: 'ffmpeg', state: status.ffmpeg && status.ffprobe ? 'ok' : 'bad', href: '?tab=settings' },
    { label: 'Диск', state: lowDisk ? 'bad' : 'ok', href: '?tab=overview' },
    ...(health ? [{ label: sourcesDown ? `Источники · ${health.sources.length - sourcesDown} из ${health.sources.length}` : 'Источники', state: sourcesDown ? 'wait' : 'ok', href: '?tab=settings' }] : []),
  ]);

  // Диск: занятая доля кольцом.
  const disk = $derived.by(() => {
    const s = status.system;
    if (!s?.diskTotal || s.diskFree == null) return null;
    return { free: s.diskFree, used: Math.max(0, Math.min(1, 1 - s.diskFree / s.diskTotal)) };
  });
  const cache = $derived(metrics ? metrics.disk.hls + metrics.disk.copy + metrics.disk.cacheOther : null);

  // Неделя: последние семь дней из двух недель показателей.
  const DOW = ['Вс', 'Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб'];
  const DOW_FULL = ['в воскресенье', 'в понедельник', 'во вторник', 'в среду', 'в четверг', 'в пятницу', 'в субботу'];
  const week = $derived.by(() => {
    const days = metrics?.activity.days.slice(-7) ?? [];
    const max = Math.max(1, ...days.map((d) => d.views));
    const top = days.reduce((a, d) => (d.views > (a?.views ?? 0) ? d : a), null as (typeof days)[number] | null);
    return { days: days.map((d) => ({ ...d, h: Math.max(6, Math.round((d.views / max) * 100)), dow: DOW[new Date(d.day).getDay()], top: d === top })), views: days.reduce((n, d) => n + d.views, 0), top };
  });
</script>

<div class="pult">
  <section class="live" aria-label="Прямой эфир">
    <div class="shot" style:background={placeholder(live?.title ?? quiet?.title ?? 'Nova')}>
      {#key shot}
        {#if shot}<img use:fadeIn src={artW(shot, 1280)} srcset={artSet(shot)} sizes="(max-width: 900px) 100vw, 60vw" alt="" decoding="async" />{/if}
      {/key}
      <div class="over">
        {#if live}
          <span class="onair"><i class="dot-live" aria-hidden="true"></i>Сейчас в эфире · {watching.length} {plural(watching.length, 'зритель', 'зрителя', 'зрителей')}</span>
          <h3>{live.title}</h3>
          <p class="sub">
            {[live.user, live.episode, quality(live.net?.quality), live.duration ? `${mins(live.position)} из ${mins(live.duration)}` : ''].filter(Boolean).join(' · ')}
          </p>
          <div class="bar" role="progressbar" aria-label="Где остановился" aria-valuenow={pct(live)} aria-valuemin="0" aria-valuemax="100"><i style:transform="scaleX({pct(live) / 100})"></i></div>
        {:else}
          <span class="onair quiet"><i class="dot-live" aria-hidden="true"></i>Сейчас тихо</span>
          <h3>{quiet ? quiet.title : 'Никто не смотрит'}</h3>
          <p class="sub">{quiet ? 'Последнее в библиотеке' : 'Когда кто-то включит фильм, он появится здесь'}</p>
        {/if}
      </div>
    </div>
    <div class="side">
      <span class="lab">Смотрят</span>
      {#if watching.length}
        <ul class="viewers">
          {#each watching as w, i (i)}
            <li>
              <button type="button" class:on={live === w} aria-pressed={live === w} onclick={() => (pick = i)}>
                <Avatar avatar={w.avatar ?? 0} photo={w.photo ?? null} size={40} />
                <span class="who"><b>{w.user}</b><small>{w.episode ? `${w.title}, ${w.episode}` : w.title}</small></span>
                <span class="chip">{pct(w)}%</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="none">Сейчас никто не смотрит</p>
      {/if}
      <span class="lab">Действия</span>
      <div class="acts">
        <a class="btn primary sm" href="?tab=folders" data-sveltekit-replacestate data-sveltekit-noscroll><Icon name="plus" size={16} /> Добавить фильмы</a>
        <a class="btn sm" href="?tab=people" data-sveltekit-replacestate data-sveltekit-noscroll><Icon name="users" size={16} /> Позвать зрителя</a>
      </div>
      <ul class="lights" aria-label="Состояние">
        {#each lights as l (l.label)}
          <li><a class={l.state} href={l.href} data-sveltekit-replacestate data-sveltekit-noscroll><i aria-hidden="true"></i>{l.label}</a></li>
        {/each}
      </ul>
    </div>
  </section>

  <div class="tiles">
    <a class="tile" href="?tab=overview" data-sveltekit-replacestate data-sveltekit-noscroll>
      <h4>Диск</h4><Icon name="right" size={18} />
      {#if disk}
        <div class="ring" style:--p="{Math.round(disk.used * 360)}deg"><span><b>{size(disk.free)}</b>свободно</span></div>
      {:else}
        <div class="ring"><span><b>-</b>нет данных</span></div>
      {/if}
      <small>фильмы {size(status.size)}{cache != null ? ` · кэш ${size(cache)}` : ''}</small>
    </a>
    <a class="tile" href="?tab=overview" data-sveltekit-replacestate data-sveltekit-noscroll>
      <h4>Неделя</h4><Icon name="right" size={18} />
      <div class="spark" aria-hidden="true">
        {#each week.days as d (d.day)}<i class:hi={d.top} style:transform="scaleY({d.h / 100})"></i>{/each}
      </div>
      <div class="days" aria-hidden="true">{#each week.days as d (d.day)}<span>{d.dow}</span>{/each}</div>
      <small>
        {#if metrics}
          {metrics.people.activeWeek} {plural(metrics.people.activeWeek, 'зритель', 'зрителя', 'зрителей')} · {week.views} {plural(week.views, 'просмотр', 'просмотра', 'просмотров')}{week.top ? ` · пик ${DOW_FULL[new Date(week.top.day).getDay()]}` : ''}
        {:else}Считаем…{/if}
      </small>
    </a>
    <a class="tile" href="?tab=settings" data-sveltekit-replacestate data-sveltekit-noscroll>
      <h4>Источники видео</h4><Icon name="right" size={18} />
      {#if health?.sources.length}
        <ul class="srcs">
          {#each health.sources.slice(0, 8) as s (s.site)}<li class={s.state}><i aria-hidden="true"></i>{s.label}</li>{/each}
        </ul>
      {:else}
        <small>Проверяем…</small>
      {/if}
    </a>
    <a class="tile" href="?tab=catalog" data-sveltekit-replacestate data-sveltekit-noscroll>
      <h4>Свежее в библиотеке</h4><Icon name="right" size={18} />
      <div class="stripe" aria-hidden="true">
        {#each fresh as c (c.id)}<img src={artW(c.poster, 185)} alt="" loading="lazy" decoding="async" />{/each}
      </div>
      <small>
        {status.files} {plural(status.files, 'файл', 'файла', 'файлов')} · {status.unmatched ? `без описания ${status.unmatched}` : 'все описаны'}{badges.wishes ? ` · ${badges.wishes} ${plural(badges.wishes, 'просьба ждёт', 'просьбы ждут', 'просьб ждут')}` : ''}
      </small>
    </a>
  </div>
</div>

<style>
  .pult { display: grid; gap: 16px; }
  .lab { font-size: 12px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; color: var(--faint, #71717a); }

  .live {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(0, 1fr);
    border-radius: 28px;
    overflow: hidden;
    background: #0c0c0e;
    border: 1px solid var(--line);
    min-height: 340px;
  }
  .shot { position: relative; isolation: isolate; min-height: 340px; overflow: hidden; }
  .shot img { position: absolute; inset: 0; width: 100%; max-width: none; height: 100%; object-fit: cover; z-index: -1; }
  .shot::after {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background: linear-gradient(90deg, #0000 45%, #0c0c0e 100%), linear-gradient(0deg, #000000d9, #0000 60%);
  }
  .over { position: absolute; left: 28px; right: 28px; bottom: 24px; display: grid; gap: 6px; justify-items: start; }
  .onair { display: inline-flex; align-items: center; gap: 10px; font-size: 13px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; }
  .dot-live { width: 8px; height: 8px; border-radius: 50%; background: #f43f5e; box-shadow: 0 0 0 4px #f43f5e33; animation: beat 1.6s ease-in-out infinite; }
  .quiet .dot-live { background: #71717a; box-shadow: 0 0 0 4px #71717a33; animation: none; }
  @keyframes beat { 50% { opacity: 0.35; } }
  :global(.no-motion) .dot-live { animation: none; }
  h3 { margin: 4px 0 0; max-width: 100%; font-size: clamp(28px, 3.2vw, 44px); font-weight: 900; letter-spacing: -0.045em; line-height: 1.04; text-wrap: balance; }
  .sub { margin: 0; color: #ffffffb8; font-size: 14px; }
  .bar { width: min(520px, 100%); height: 4px; margin-top: 10px; border-radius: 4px; background: #ffffff30; overflow: hidden; }
  .bar i { display: block; height: 100%; background: #fff; transform-origin: left; transition: transform 0.6s var(--ease, ease); }

  .side { padding: 24px; display: grid; gap: 12px; align-content: start; }
  .viewers { display: grid; gap: 4px; margin: 0; padding: 0; list-style: none; }
  .viewers button {
    width: 100%;
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 8px 10px;
    border: 0;
    border-radius: 16px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 0.2s;
  }
  .viewers button:hover, .viewers button.on { background: #ffffff12; }
  .who { min-width: 0; display: grid; }
  .who b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who small { color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .chip { padding: 4px 10px; border-radius: 999px; background: #ffffff14; font-size: 12.5px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .none { margin: 0; color: var(--muted); font-size: 14px; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; }
  .lights { display: flex; flex-wrap: wrap; gap: 8px; margin: 6px 0 0; padding: 0; list-style: none; }
  .lights a { display: inline-flex; align-items: center; gap: 8px; height: 32px; padding: 0 12px; border-radius: 999px; background: #ffffff0f; color: var(--text); font-size: 13px; font-weight: 600; text-decoration: none; }
  .lights a:hover { background: #ffffff1a; }
  .lights i, .srcs i { width: 8px; height: 8px; border-radius: 50%; background: #22c55e; flex: none; }
  .lights .wait i { background: #eab308; }
  .srcs .unknown i { background: #71717a; }
  .lights .bad i, .srcs .down i { background: #ef4444; }

  .tiles { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 16px; }
  .tile {
    position: relative;
    display: grid;
    gap: 14px;
    align-content: start;
    min-height: 230px;
    padding: 20px;
    border-radius: 24px;
    border: 1px solid var(--line);
    background: #111113;
    color: var(--text);
    text-decoration: none;
    transition: background 0.2s, transform 0.3s var(--ease, ease);
  }
  .tile:hover { background: #17171a; transform: translateY(-2px); }
  .tile > :global(svg) { position: absolute; right: 18px; top: 20px; color: var(--faint, #71717a); }
  h4 { margin: 0; padding-right: 26px; font-size: 17px; letter-spacing: -0.02em; }
  .tile small { color: var(--muted); font-size: 13px; line-height: 1.4; }
  .ring { --p: 0deg; justify-self: center; width: 120px; height: 120px; border-radius: 50%; background: conic-gradient(#fff 0 var(--p), #ffffff1f var(--p) 360deg); display: grid; place-items: center; }
  .ring span { width: 92px; height: 92px; border-radius: 50%; background: #111113; display: grid; place-content: center; text-align: center; font-size: 12px; color: var(--muted); }
  .tile:hover .ring span { background: #17171a; }
  .ring b { display: block; color: #fff; font-size: 18px; font-variant-numeric: tabular-nums; }
  .tile:has(.ring) small { text-align: center; }
  .spark { display: flex; align-items: flex-end; gap: 6px; height: 90px; }
  .spark i { flex: 1; height: 100%; border-radius: 6px 6px 2px 2px; background: #ffffff2e; transform-origin: bottom; }
  .spark i.hi { background: #fff; }
  .days { display: flex; justify-content: space-between; color: var(--faint, #71717a); font-size: 11px; }
  .days span { flex: 1; text-align: center; }
  .srcs { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; margin: 0; padding: 0; list-style: none; }
  .srcs li { display: flex; align-items: center; gap: 8px; min-width: 0; padding: 8px 10px; border-radius: 12px; background: #ffffff0a; font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .stripe { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 6px; max-width: 280px; }
  .stripe img { width: 100%; max-width: none; aspect-ratio: 2 / 3; border-radius: 8px; object-fit: cover; }

  @media (max-width: 1280px) { .tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @media (max-width: 900px) {
    .live { grid-template-columns: minmax(0, 1fr); }
    .shot { min-height: 260px; }
    .shot::after { background: linear-gradient(0deg, #000000e0, #0000 70%); }
    .over { left: 18px; right: 18px; bottom: 18px; }
    .side { padding: 18px; }
  }
  @media (max-width: 560px) { .tiles { grid-template-columns: minmax(0, 1fr); } .tile { min-height: 0; } }
</style>
