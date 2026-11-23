<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '$lib/api';
  import { plural, size } from '$lib/fmt';
  import { badges } from '$lib/social/badges.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';
  import Access from './Access.svelte';
  import Folders from './Folders.svelte';
  import People from './People.svelte';
  import Settings from './Settings.svelte';
  import Version from './Version.svelte';
  import type { Health, Metrics, Status } from './types';

  // «Карта дома» (выбор владельца, 23.11): вся система одной схемой - откуда берутся фильмы (папки, описания и
  // источники видео), сама Nova на этом компьютере, выход в интернет и зрители на своих устройствах. Линии к тем, кто
  // смотрит прямо сейчас, «бегут». Лампочка у каждой карточки - её состояние; нажатие открывает справа её настройки
  // (те же разделы, что во вкладках). На узком экране карточки идут столбиком в том же порядке, линии - вертикальные.
  let { status, reload, window: inWindow }: { status: Status; reload: () => void; window: boolean } = $props();

  type Lamp = 'ok' | 'wait' | 'bad' | 'off';
  type NodeId = 'folders' | 'sources' | 'server' | 'internet' | 'viewers';
  interface AccessView { mode: 'lan' | 'cloudpub' | 'fxtunnel' | 'custom'; state: string; message: string | null; publicUrl: string | null; lanUrl: string | null }
  interface Lib { id: number; path: string; files: number }

  let libs = $state<Lib[] | null>(null);
  let access = $state<AccessView | null>(null);
  let health = $state<Health | null>(null);
  let metrics = $state<Metrics | null>(null);
  let open = $state<NodeId | null>(null);

  function load() {
    api<Lib[]>('/api/admin/libraries').then((l) => (libs = l), () => {});
    api<AccessView>('/api/admin/access').then((a) => (access = a), () => {});
    api<Health>('/api/admin/health').then((h) => (health = h), () => {});
    api<Metrics>(`/api/admin/metrics?tz=${-new Date().getTimezoneOffset()}`).then((m) => (metrics = m), () => {});
  }
  // Схема живая: состояние раз в 5 с, остальное раз в 20 с (пока открыта вкладка).
  onMount(() => {
    load();
    const fast = setInterval(reload, 5000);
    const slow = setInterval(load, 20000);
    return () => {
      clearInterval(fast);
      clearInterval(slow);
    };
  });

  const MODE: Record<AccessView['mode'], string> = { lan: 'Только дома', cloudpub: 'По ссылке CloudPub', fxtunnel: 'По ссылке fxTunnel', custom: 'Свой адрес' };
  const sourcesUp = $derived(health?.sources.filter((s) => s.state !== 'down').length ?? 0);
  const sourcesDown = $derived(health?.sources.filter((s) => s.state === 'down') ?? []);
  const lowDisk = $derived(status.system?.diskFree != null && status.system.diskFree < 10 * 1024 ** 3);
  const cache = $derived(metrics ? metrics.disk.hls + metrics.disk.copy + metrics.disk.cacheOther : null);
  const pct = (n: number) => `${Math.round(n)}%`;

  interface Card { id: NodeId; icon: IconName; title: string; sub: string; lamp: Lamp; lampText: string; facts: { v: string; l: string }[] }
  const cards = $derived.by<Record<Exclude<NodeId, 'viewers'>, Card>>(() => {
    const scan = status.scan;
    const folders: Card = {
      id: 'folders', icon: 'folder', title: 'Папки с фильмами',
      sub: libs ? (libs.length ? `${libs.length} ${plural(libs.length, 'папка', 'папки', 'папок')} на этом компьютере` : 'Папок пока нет') : 'Проверяю папки…',
      lamp: scan.error ? 'bad' : scan.running ? 'wait' : libs && !libs.length ? 'bad' : 'ok',
      lampText: scan.error ? 'Ошибка обхода' : scan.running ? `Обхожу${scan.progress != null ? ` ${pct(scan.progress * 100)}` : ''}` : libs && !libs.length ? 'Добавьте папку' : 'Всё на месте',
      facts: [
        { v: String(status.titles), l: plural(status.titles, 'фильм и сериал', 'фильма и сериала', 'фильмов и сериалов') },
        { v: String(status.files), l: plural(status.files, 'файл', 'файла', 'файлов') },
        { v: size(status.size), l: 'на диске' },
      ],
    };
    const sources: Card = {
      id: 'sources', icon: 'film', title: 'Описания и видео',
      sub: health ? health.sources.map((s) => s.label).slice(0, 4).join(', ') : 'IMDb и сайты с видео',
      lamp: sourcesDown.length || status.unmatched ? 'wait' : status.meta.running ? 'wait' : 'ok',
      lampText: sourcesDown.length ? `Не отвечает: ${sourcesDown.map((s) => s.label).join(', ')}` : status.unmatched ? 'Есть фильмы без описания' : status.meta.running ? 'Скачиваю описания' : 'Всё нашлось',
      facts: [
        { v: String(status.unmatched), l: 'без описания' },
        { v: health ? `${sourcesUp} из ${health.sources.length}` : '…', l: 'источников работают' },
      ],
    };
    const update = status.update?.state === 'available';
    const server: Card = {
      id: 'server', icon: 'pulse', title: 'Nova на этом компьютере',
      sub: `версия ${status.version}${update ? ` · есть ${status.update?.latest}` : ''}`,
      lamp: !status.ffmpeg || !status.ffprobe || lowDisk ? 'bad' : update ? 'wait' : 'ok',
      lampText: !status.ffmpeg || !status.ffprobe ? 'Нет ffmpeg: MKV и AVI не играют' : lowDisk ? 'Мало места на диске' : update ? 'Доступно обновление' : 'Работает',
      facts: [
        ...(status.system?.cpu != null ? [{ v: pct(status.system.cpu), l: 'процессор' }] : []),
        ...(cache != null ? [{ v: size(cache), l: 'кэш видео' }] : []),
        ...(status.system?.diskFree != null ? [{ v: size(status.system.diskFree), l: 'свободно' }] : []),
      ],
    };
    const lanOnly = !access || access.mode === 'lan';
    const internet: Card = {
      id: 'internet', icon: 'globe', title: 'Интернет',
      sub: access ? (access.publicUrl ?? access.lanUrl ?? '') : 'Проверяю доступ…',
      lamp: !access ? 'off' : access.state === 'error' ? 'bad' : lanOnly ? 'off' : access.publicUrl ? 'ok' : 'wait',
      lampText: !access ? '' : access.state === 'error' ? (access.message ?? 'Не подключается') : lanOnly ? 'Только домашняя сеть' : access.publicUrl ? MODE[access.mode] : 'Подключаюсь…',
      facts: [],
    };
    return { folders, sources, server, internet };
  });
  const watching = $derived(status.watching);

  // Что требует внимания - одной строкой каждое, с переходом к нужной карточке.
  const todo = $derived.by<{ text: string; node: NodeId }[]>(() => {
    const t: { text: string; node: NodeId }[] = [];
    if (libs && !libs.length) t.push({ text: 'Добавьте папку с фильмами - пока зрителям нечего смотреть', node: 'folders' });
    if (!status.ffmpeg || !status.ffprobe) t.push({ text: 'Нет ffmpeg: фильмы в MKV и AVI не будут играть', node: 'server' });
    if (status.unmatched) t.push({ text: `${status.unmatched} ${plural(status.unmatched, 'фильм', 'фильма', 'фильмов')} без описания - Nova не нашла ${status.unmatched === 1 ? 'его' : 'их'} на IMDb`, node: 'sources' });
    for (const s of sourcesDown) t.push({ text: `${s.label} не отвечает - Nova пробует снова через 10 минут`, node: 'sources' });
    if (access?.state === 'error') t.push({ text: `Доступ из интернета: ${access.message ?? 'не подключается'}`, node: 'internet' });
    if (status.update?.state === 'available') t.push({ text: `Доступна Nova ${status.update.latest}`, node: 'server' });
    if (lowDisk) t.push({ text: 'Свободно меньше 10 ГБ - очистите кэш видео или диск', node: 'server' });
    if (badges.wishes) t.push({ text: `${badges.wishes} ${plural(badges.wishes, 'просьба', 'просьбы', 'просьб')} зрителей добавить фильм`, node: 'folders' });
    return t;
  });

  // ---------- линии между карточками ----------
  let map = $state<HTMLDivElement>();
  let paths = $state<{ d: string; live: boolean; key: string }[]>([]);
  function measure() {
    if (!map) return;
    const box = map.getBoundingClientRect();
    if (box.width < 860) return void (paths = []);
    const at = (id: string) => map?.querySelector<HTMLElement>(`[data-node="${id}"]`)?.getBoundingClientRect();
    const rel = (r: DOMRect) => ({ l: r.left - box.left, r: r.right - box.left, t: r.top - box.top, b: r.bottom - box.top, cx: r.left - box.left + r.width / 2, cy: r.top - box.top + r.height / 2 });
    const curve = (x1: number, y1: number, x2: number, y2: number) => {
      const k = Math.max(30, Math.abs(x2 - x1) / 2);
      return `M ${x1} ${y1} C ${x1 + k} ${y1}, ${x2 - k} ${y2}, ${x2} ${y2}`;
    };
    const server = at('server');
    if (!server) return;
    const s = rel(server);
    const out: { d: string; live: boolean; key: string }[] = [];
    for (const id of ['folders', 'sources']) {
      const r = at(id);
      if (r) {
        const a = rel(r);
        out.push({ key: id, d: curve(a.r, a.cy, s.l, s.cy), live: id === 'folders' && watching.length > 0 });
      }
    }
    const net = at('internet');
    if (net) {
      const n = rel(net);
      out.push({ key: 'internet', d: `M ${s.cx} ${s.t} L ${n.cx} ${n.b}`, live: !!access?.publicUrl && watching.length > 0 });
    }
    map.querySelectorAll<HTMLElement>('[data-node^="viewer"]').forEach((el, i) => {
      const v = rel(el.getBoundingClientRect());
      out.push({ key: `v${i}`, d: curve(s.r, s.cy, v.l, v.cy), live: /^viewer\d+$/.test(el.dataset.node ?? '') });
    });
    paths = out;
  }
  $effect(() => {
    // Пересчитываем при смене данных (меняются высоты карточек и число зрителей) и при изменении размера.
    void [watching.length, libs, access, health, metrics, todo.length];
    tick().then(measure);
  });
  onMount(() => {
    const ro = new ResizeObserver(() => measure());
    if (map) ro.observe(map);
    return () => ro.disconnect();
  });

  // ---------- панель настроек справа ----------
  let dialog = $state<HTMLDialogElement>();
  const PANEL: Record<NodeId, string> = { folders: 'Папки с фильмами', sources: 'Описания и видео', server: 'Nova на этом компьютере', internet: 'Вход из интернета', viewers: 'Зрители' };
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });
  function cancel(e: Event) {
    e.preventDefault();
    open = null;
  }
  function outside(e: MouseEvent) {
    if (e.target !== dialog || !dialog) return;
    const r = dialog.getBoundingClientRect();
    if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) open = null;
  }
  const quality = (q: string) => (q === 'orig' ? 'оригинал' : q === 'auto' ? 'авто' : q);
</script>

{#snippet node(c: Card)}
  <button type="button" class="node" class:sel={open === c.id} data-node={c.id} onclick={() => (open = c.id)} aria-haspopup="dialog">
    <span class="nt"><i class="lamp {c.lamp}" aria-hidden="true"></i><Icon name={c.icon} size={16} /><b>{c.title}</b></span>
    {#if c.sub}<small class="sub selectable">{c.sub}</small>{/if}
    {#if c.facts.length}<span class="facts">{#each c.facts as f (f.l)}<span><b>{f.v}</b>{f.l}</span>{/each}</span>{/if}
    {#if c.lampText}<span class="state {c.lamp}">{c.lampText}</span>{/if}
  </button>
{/snippet}

<div class="hm">
  <div class="map" bind:this={map}>
    <svg class="links" aria-hidden="true">
      {#each paths as p (p.key)}<path d={p.d} class:live={p.live} />{/each}
    </svg>
    <div class="col left">
      {@render node(cards.folders)}
      {@render node(cards.sources)}
    </div>
    <div class="col mid">
      {@render node(cards.internet)}
      {@render node(cards.server)}
    </div>
    <div class="col right">
      {#each watching as w, i (i)}
        <button type="button" class="node viewer" data-node="viewer{i}" onclick={() => (open = 'viewers')} aria-haspopup="dialog">
          <span class="nt"><i class="lamp {w.net && w.net.stalls > 1 ? 'wait' : 'ok'}" aria-hidden="true"></i><Icon name="user" size={16} /><b>{w.user}</b></span>
          <small class="sub">{w.title}{w.episode ? ` · ${w.episode}` : ''}</small>
          {#if w.net}<span class="state {w.net.stalls > 1 ? 'wait' : 'ok'}">{quality(w.net.quality)}{w.net.mbit != null ? ` · ${w.net.mbit.toFixed(1).replace('.', ',')} Мбит/с` : ''}{w.net.stalls ? ` · ${w.net.stalls} ${plural(w.net.stalls, 'остановка', 'остановки', 'остановок')}` : ' · без остановок'}</span>{/if}
          <span class="prog" aria-hidden="true"><i style:transform="scaleX({w.duration ? Math.min(1, w.position / w.duration) : 0})"></i></span>
        </button>
      {/each}
      <button type="button" class="node" data-node={watching.length ? 'viewers-all' : 'viewers'} class:quiet={watching.length > 0} onclick={() => (open = 'viewers')} aria-haspopup="dialog">
        <span class="nt"><i class="lamp {watching.length ? 'ok' : 'off'}" aria-hidden="true"></i><Icon name="users" size={16} /><b>Зрители</b></span>
        <small class="sub">{watching.length ? `${watching.length} ${plural(watching.length, 'смотрит', 'смотрят', 'смотрят')} сейчас` : 'Сейчас никто не смотрит'}</small>
        {#if metrics}<span class="facts"><span><b>{metrics.people.users}</b>{plural(metrics.people.users, 'аккаунт', 'аккаунта', 'аккаунтов')}</span><span><b>{metrics.people.activeWeek}</b>смотрели за неделю</span></span>{/if}
      </button>
    </div>
  </div>

  <section class="todo" aria-label="Требует внимания">
    <h3>{todo.length ? 'Требует внимания' : 'Всё в порядке'}</h3>
    {#if todo.length}
      <ul>
        {#each todo as t (t.text)}
          <li><span>{t.text}</span><button class="btn sm" type="button" onclick={() => (open = t.node)}>Открыть</button></li>
        {/each}
      </ul>
    {:else}
      <p>Фильмы на месте, описания нашлись, Nova работает{access?.publicUrl ? ', вход из интернета открыт' : ''}.</p>
    {/if}
  </section>
</div>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog class="panel" bind:this={dialog} oncancel={cancel} onclose={() => (open = null)} onclick={outside} aria-label={open ? PANEL[open] : 'Настройки'}>
  {#if open}
    <header>
      <h2>{PANEL[open]}</h2>
      <button class="btn ghost sm icon" type="button" aria-label="Закрыть" onclick={() => (open = null)}><Icon name="close" size={18} /></button>
    </header>
    <div class="body">
      {#if open === 'folders'}<Folders {status} window={inWindow} {reload} />
      {:else if open === 'sources'}
        {#if status.unmatched}<p class="hint">{status.unmatched} {plural(status.unmatched, 'фильм', 'фильма', 'фильмов')} без описания - поправьте в <a href="?tab=catalog" data-sveltekit-replacestate>Каталоге</a>.</p>{/if}
        <Settings sourcesOnly />
      {:else if open === 'server'}<Version {status} {reload} />
      {:else if open === 'internet'}<Access />
      {:else if open === 'viewers'}
        {#if watching.length}
          <ul class="now">
            {#each watching as w, i (i)}
              <li><b>{w.user}</b><span>{w.title}{w.episode ? ` · ${w.episode}` : ''}</span><span class="prog"><i style:transform="scaleX({w.duration ? Math.min(1, w.position / w.duration) : 0})"></i></span></li>
            {/each}
          </ul>
        {/if}
        <People />
      {/if}
    </div>
  {/if}
</dialog>

<style>
  .hm { display: grid; gap: 22px; }
  .map {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr);
    gap: 72px;
    padding: 28px;
    border-radius: 24px;
    background: radial-gradient(60% 70% at 50% 45%, #12171b, #08080a 75%);
    border: 1px solid #ffffff14;
    min-height: 420px;
  }
  .col { position: relative; z-index: 1; display: grid; gap: 18px; align-content: center; }
  .mid { grid-template-rows: auto 1fr; align-content: stretch; gap: 44px; }
  .mid > :last-child { align-self: center; }
  .links { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; overflow: visible; }
  .links path { fill: none; stroke: #ffffff2e; stroke-width: 1.5; }
  .links path.live { stroke: #ffffffc0; stroke-width: 2.4; stroke-linecap: round; stroke-dasharray: 2 9; animation: flow 1.1s linear infinite; }
  @keyframes flow { to { stroke-dashoffset: -11; } }
  :global(.no-motion) .links path.live { animation: none; }

  .node {
    display: grid;
    gap: 8px;
    width: 100%;
    padding: 16px;
    border: 1px solid #ffffff1f;
    border-radius: 18px;
    background: #121215;
    color: #fff;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.2s, transform 0.25s var(--ease, ease);
  }
  .node:hover { border-color: #ffffff55; transform: translateY(-2px); }
  .node.sel { border-color: #fff; }
  .node:focus-visible { outline: 2px solid #fff; outline-offset: 3px; }
  .node.quiet { background: #0d0d10; }
  .nt { display: flex; align-items: center; gap: 8px; }
  .nt b { font-size: 15px; font-weight: 700; }
  .lamp { width: 9px; height: 9px; flex: none; border-radius: 50%; background: #6c6c75; }
  .lamp.ok { background: #46c992; box-shadow: 0 0 0 4px #46c99226; }
  .lamp.wait { background: #e5b452; box-shadow: 0 0 0 4px #e5b45226; }
  .lamp.bad { background: #ef6a6a; box-shadow: 0 0 0 4px #ef6a6a26; }
  .sub { color: #a1a1aa; font-size: 12.5px; overflow-wrap: anywhere; }
  .facts { display: flex; flex-wrap: wrap; gap: 6px 16px; color: #a1a1aa; font-size: 12px; }
  .facts b { display: block; color: #fff; font-size: 19px; font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .state { font-size: 12px; font-weight: 600; color: #c9c9cf; }
  .state.wait { color: #e5b452; } .state.bad { color: #ef6a6a; } .state.ok { color: #9fe3c3; }
  .prog { display: block; height: 3px; border-radius: 3px; background: #ffffff1f; overflow: hidden; }
  .prog i { display: block; height: 100%; background: #fff; transform-origin: left; }

  .todo { display: grid; gap: 10px; }
  .todo h3 { margin: 0; font-size: 16px; }
  .todo p { margin: 0; color: #a1a1aa; }
  .todo ul { margin: 0; padding: 0; list-style: none; display: grid; gap: 8px; }
  .todo li { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 14px; border-radius: 14px; background: #ffffff0a; }
  .todo li span { min-width: 0; }

  dialog.panel {
    position: fixed;
    inset: 0 0 0 auto;
    margin: 0;
    width: min(560px, 100vw);
    max-width: none;
    height: 100dvh;
    max-height: none;
    padding: 0;
    border: 0;
    border-left: 1px solid #ffffff1f;
    border-radius: 0;
    background: #0d0d0f;
    color: #fff;
    overflow: auto;
  }
  dialog.panel[open] { animation: slide 0.28s cubic-bezier(0.2, 0.9, 0.3, 1); }
  dialog.panel::backdrop { background: #000000a0; }
  @keyframes slide { from { transform: translateX(40px); opacity: 0; } }
  dialog.panel header { position: sticky; top: 0; z-index: 2; display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 18px 20px; padding-top: max(18px, env(safe-area-inset-top)); background: #0d0d0f; border-bottom: 1px solid #ffffff14; }
  dialog.panel h2 { margin: 0; font-size: 20px; letter-spacing: -0.02em; }
  dialog.panel .body { padding: 18px 20px calc(28px + env(safe-area-inset-bottom)); display: grid; gap: 16px; }
  .hint { margin: 0; color: #c9c9cf; }
  .now { margin: 0; padding: 0; list-style: none; display: grid; gap: 8px; }
  .now li { display: grid; gap: 4px; padding: 12px; border-radius: 14px; background: #ffffff0a; }
  .now span { color: #a1a1aa; font-size: 13px; }

  /* Узко: столбиком, связи - короткие вертикальные черты между карточками. */
  @media (max-width: 900px) {
    .map { grid-template-columns: minmax(0, 1fr); gap: 0; padding: 16px; }
    .col, .mid { gap: 0; align-content: start; grid-template-rows: none; }
    .mid > :first-child { order: 2; }
    .col .node { position: relative; margin-top: 22px; }
    .left .node:first-child { margin-top: 0; }
    .col .node::before { content: ''; position: absolute; left: 50%; top: -23px; height: 22px; border-left: 1px dashed #ffffff40; }
    .left .node:first-child::before { display: none; }
  }
</style>
