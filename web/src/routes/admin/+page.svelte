<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, errorText } from '$lib/api';
  import Access from '$lib/admin/Access.svelte';
  import { attention } from '$lib/admin/attention';
  import Catalog from '$lib/admin/Catalog.svelte';
  import Folders from '$lib/admin/Folders.svelte';
  import Hall from '$lib/admin/Hall.svelte';
  import HomeMap from '$lib/admin/HomeMap.svelte';
  import HostTitleSource from '$lib/admin/HostTitleSource.svelte';
  import LoginJournal from '$lib/admin/LoginJournal.svelte';
  import ManualSources from '$lib/admin/ManualSources.svelte';
  import { href, resolve, SECTIONS } from '$lib/admin/nav';
  import Overview from '$lib/admin/Overview.svelte';
  import Palette from '$lib/admin/Palette.svelte';
  import People from '$lib/admin/People.svelte';
  import Settings from '$lib/admin/Settings.svelte';
  import SourcesHealth from '$lib/admin/SourcesHealth.svelte';
  import ThemeSettings from '$lib/admin/ThemeSettings.svelte';
  import type { Health, Status } from '$lib/admin/types';
  import Upkeep from '$lib/admin/Upkeep.svelte';
  import Version from '$lib/admin/Version.svelte';
  import Wishes from '$lib/admin/Wishes.svelte';
  import Wizard from '$lib/admin/Wizard.svelte';
  import ChainCheck from '$lib/discover/ChainCheck.svelte';
  import { plural, size } from '$lib/fmt';
  import { session } from '$lib/session.svelte';
  import { badges } from '$lib/social/badges.svelte';
  import { artW } from '$lib/ui/art';
  import { catalog, loadCatalog } from '$lib/catalog.svelte';
  import Icon from '$lib/ui/Icon.svelte';

  // «Управление» A1 «Зал» (выбор владельца 27.11): слева пять разделов, у каждого живая цифра (кто смотрит, места беты,
  // свежие постеры, огоньки источников, нагрузка) и счётчик того, что требует внимания; справа - раздел с вкладками.
  // Ctrl K - поиск по всему управлению. Прежние `?tab=` ведут на новые места (`nav.ts`).
  const at = $derived(resolve(page.url.searchParams.get('tab'), page.url.searchParams.get('sub')));
  const section = $derived(SECTIONS.find((s) => s.id === at.section) ?? null);

  let status = $state<Status | null>(null);
  let health = $state<Health | null>(null);
  let statusError = $state('');
  let palette = $state(false);

  function reload() {
    api<Status>('/api/admin/status').then((s) => { status = s; statusError = ''; }, (e) => { statusError = errorText(e); });
  }
  function loadHealth() {
    api<Health>('/api/admin/health').then((h) => (health = h), () => {});
  }
  onMount(() => {
    reload();
    loadHealth();
    loadCatalog();
    const t = setInterval(loadHealth, 30000);
    const key = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        palette = true;
      }
    };
    addEventListener('keydown', key);
    return () => {
      clearInterval(t);
      removeEventListener('keydown', key);
    };
  });

  $effect(() => {
    if (session.me && !session.me.manage) goto('/', { replace: true });
  });

  // Что требует внимания - по разделам: красный счётчик, если есть поломка, жёлтый - если стоит заглянуть.
  const items = $derived(status ? attention(status, health, badges.wishes) : []);
  const flags = $derived.by(() => {
    const out: Record<string, { n: number; bad: boolean }> = {};
    for (const a of items) {
      if (a.level === 'info') continue;
      const s = a.act === 'backup' ? 'system' : (a.href?.match(/tab=([a-z]+)/)?.[1] ?? 'summary');
      const f = (out[s] ??= { n: 0, bad: false });
      f.n++;
      f.bad ||= a.level === 'bad';
    }
    return out;
  });
  const fresh = $derived((catalog.items ?? []).filter((c) => c.poster).slice().sort((a, b) => b.addedAt - a.addedAt).slice(0, 3));
  const sources = $derived(health?.sources ?? []);
  const sourcesOk = $derived(sources.filter((s) => s.state !== 'down').length);
</script>

<div class="rw-page admin">
  <header class="rw-head">
    <div>
      <h1>Управление</h1>
      {#if status}<p>Nova {status.version}</p>{/if}
    </div>
    <button class="find" type="button" onclick={() => (palette = true)} aria-keyshortcuts="Control+K">
      <Icon name="search" size={16} /><span>Найти зрителя, настройку, действие</span><kbd>Ctrl K</kbd>
    </button>
  </header>

  <nav class="rail" aria-label="Разделы управления">
    {#each SECTIONS as s (s.id)}
      {@const f = flags[s.id]}
      <a href={href(s.id)} class="nav" aria-current={at.section === s.id ? 'page' : undefined} data-sveltekit-replacestate data-sveltekit-noscroll>
        <span class="ic"><Icon name={s.icon} size={18} /></span>
        <b>{s.label}{#if f}<em class:bad={f.bad} aria-label="требует внимания: {f.n}">{f.n}</em>{/if}</b>
        <span class="live">
          {#if !status}<span class="wait">…</span>
          {:else if s.id === 'summary'}
            {#if status.watching.length}<i class="rec" aria-hidden="true"></i>{status.watching.length} {plural(status.watching.length, 'смотрит', 'смотрят', 'смотрят')} сейчас{:else}Сейчас тихо{/if}
          {:else if s.id === 'viewers'}
            {#if status.seats}<span class="num">{status.seats.used}/{status.seats.total}</span><span class="bar" aria-hidden="true"><i style:transform="scaleX({Math.min(1, status.seats.used / Math.max(1, status.seats.total))})"></i></span>{:else}Без лимита мест{/if}
          {:else if s.id === 'library'}
            {#if fresh.length}<span class="ps" aria-hidden="true">{#each fresh as c (c.id)}<img src={artW(c.poster, 92)} alt="" loading="lazy" />{/each}</span>{/if}{status.titles}{badges.wishes ? ` · ${badges.wishes} ${plural(badges.wishes, 'просьба', 'просьбы', 'просьб')}` : ''}
          {:else if s.id === 'sources'}
            {#if sources.length}<span class="ds" aria-hidden="true">{#each sources.slice(0, 6) as x (x.site)}<i class={x.state}></i>{/each}</span>{sourcesOk} из {sources.length} работают{:else}Проверяем…{/if}
          {:else if s.id === 'system'}
            {#if status.update?.state === 'available'}Есть обновление{:else}{[status.system?.cpu != null ? `ЦП ${Math.round(status.system.cpu)}%` : '', status.system?.diskFree != null ? size(status.system.diskFree) : ''].filter(Boolean).join(' · ') || 'Сервер работает'}{/if}
          {/if}
        </span>
      </a>
    {/each}
    <a class="setup" href="?tab=setup" aria-current={at.section === 'setup' ? 'page' : undefined} data-sveltekit-replacestate data-sveltekit-noscroll><Icon name="sparkle" size={15} /> Мастер первой настройки</a>
  </nav>

  <section class="panel rw-plate" aria-label={section?.label ?? 'Мастер настройки'}>
    <header class="sec-h">
      <div>
        <h2 class="rw-h2">{section?.label ?? 'Мастер настройки'}</h2>
        <p>{section?.hint ?? 'Пять шагов до готовности: папка, описания, доступ, люди'}</p>
      </div>
      {#if section && section.subs.length > 1}
        <nav class="tabs" aria-label="Вкладки раздела «{section.label}»">
          {#each section.subs as x (x.id)}
            <a href={href(section.id, x.id)} aria-current={at.sub === x.id ? 'page' : undefined} data-sveltekit-replacestate data-sveltekit-noscroll>{x.label}</a>
          {/each}
        </nav>
      {/if}
    </header>
    {#if statusError}<p class="error" role="alert">Не удалось обновить показатели: {statusError} <button class="btn sm" onclick={reload}>Повторить</button></p>{/if}
    {#if status}
      {@const k = `${at.section}/${at.sub}`}
      {#if at.section === 'setup'}<Wizard {status} {reload} window={!!session.me?.window} />
      {:else if k === 'summary/now'}<Hall {status} {health} {reload} />
      {:else if k === 'summary/stats'}<Overview {status} {reload} />
      {:else if k === 'summary/map'}<HomeMap {status} {reload} window={!!session.me?.window} />
      {:else if k === 'viewers/people'}<People find={page.url.searchParams.get('find') ?? ''} />
      {:else if k === 'viewers/signup'}<Settings parts={['site']} />
      {:else if k === 'viewers/logins'}<section class="card pane"><LoginJournal /></section>
      {:else if k === 'library/catalog'}<Catalog sources={false} />
      {:else if k === 'library/folders'}<Folders {status} window={!!session.me?.window} {reload} />
      {:else if k === 'library/wishes'}<Wishes />
      {:else if k === 'library/showcase'}<ThemeSettings />
      {:else if k === 'sources/video'}
        <SourcesHealth {health} />
        <Settings parts={['links']} />
        <ChainCheck />
        <div id="sources"><ManualSources /></div>
        {#if session.me?.user.role === 'host'}<HostTitleSource />{/if}
      {:else if k === 'sources/meta'}<Settings parts={['meta']} />
      {:else if k === 'system/server'}<Upkeep {status} {reload} /><Settings parts={['ffmpeg', 'night']} />
      {:else if k === 'system/net'}<Access />
      {:else if k === 'system/security'}<Settings parts={['security', 'mail']} />
      {:else if k === 'system/update'}<Version {status} {reload} />
      {/if}
    {/if}
  </section>
</div>

<Palette bind:open={palette} {reload} />

<style>
  /* Плиты #101012 с тонкой линией - как раньше; токены переопределены для всех вкладок сразу. */
  .admin {
    --sheet: #101012;
    --sheet-2: #17171a;
    --sheet-3: #1f1f23;
    --sheet-line: #ffffff1a;
    --surface: #101012;
    --surface-2: #17171a;
    --surface-3: #1f1f23;
    --line: #ffffff1a;
  }
  .admin :global(.rw-head h1) { font-size: clamp(34px, 4.2vw, 56px); letter-spacing: -0.04em; line-height: 1.02; }
  .admin :global(.panel .pane) { border: 1px solid var(--line); border-radius: 24px; padding: 22px; margin-block: 16px; background: #141416; }
  .admin :global(.panel .pane h3) { font-size: 17px; letter-spacing: -0.02em; }
  .admin :global(.panel .input) { min-height: 42px; border-radius: 12px; }
  .rw-head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; flex-wrap: wrap; }

  .find {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    max-width: 100%;
    padding: 10px 12px 10px 16px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: #ffffff0a;
    color: var(--muted);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
    transition: background 0.2s, color 0.2s;
  }
  .find:hover { background: #ffffff14; color: var(--text); }
  .find span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .find kbd { margin-left: 6px; padding: 2px 7px; border-radius: 6px; background: #ffffff1a; color: var(--text); font: 700 11px var(--font, inherit); }

  /* Панель разделов: у каждого живая цифра; выбранный - белый. */
  .rail { display: flex; flex-direction: column; gap: 4px; }
  .nav {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr);
    gap: 3px 12px;
    align-items: center;
    padding: 11px 12px;
    border-radius: 18px;
    color: var(--text);
    transition: background 0.25s var(--ease), transform 0.2s var(--ease);
  }
  .nav:hover { background: #ffffff0d; }
  .nav:active { transform: scale(0.98); }
  .ic { grid-row: span 2; display: grid; place-items: center; width: 36px; height: 36px; border-radius: 11px; background: #ffffff14; transition: background 0.25s, color 0.25s; }
  .nav b { display: flex; justify-content: space-between; align-items: center; gap: 8px; font-weight: 600; }
  .nav em { min-width: 20px; padding: 1px 7px; border-radius: 99px; background: #ffd60a; color: #000; font-style: normal; font-size: 11px; font-weight: 800; text-align: center; }
  .nav em.bad { background: var(--danger); color: #fff; }
  .live { display: flex; align-items: center; gap: 6px; min-width: 0; overflow: hidden; color: var(--muted); font-size: 12px; white-space: nowrap; }
  .live .num { font-variant-numeric: tabular-nums; }
  .live .bar { flex: 1; max-width: 80px; height: 4px; border-radius: 99px; background: #ffffff1f; overflow: hidden; }
  .live .bar i { display: block; height: 100%; background: #fff; transform-origin: 0 50%; }
  .rec { width: 7px; height: 7px; border-radius: 50%; background: #ff3b30; flex: none; }
  .ps { display: flex; padding-right: 8px; flex: none; }
  .ps img { width: 16px; height: 24px; margin-right: -8px; border: 1px solid #000; border-radius: 3px; object-fit: cover; }
  .ds { display: flex; gap: 3px; flex: none; }
  .ds i { width: 6px; height: 6px; border-radius: 50%; background: var(--ok); }
  .ds i.down { background: var(--danger); }
  .ds i.unknown { background: #636366; }
  .nav[aria-current='page'] { background: #fff; color: #000; }
  .nav[aria-current='page'] .ic { background: #000; color: #fff; }
  .nav[aria-current='page'] .live { color: #555; }
  .nav[aria-current='page'] .live .bar { background: #0000001f; }
  .nav[aria-current='page'] .live .bar i { background: #000; }
  .setup { display: inline-flex; align-items: center; gap: 8px; margin-top: 10px; padding: 8px 12px; border-radius: 12px; color: var(--muted); font-size: 13px; }
  .setup:hover, .setup[aria-current='page'] { color: var(--text); background: #ffffff0d; }

  .panel.rw-plate { min-width: 0; margin-inline: 0; padding: clamp(18px, 2.4vw, 30px); border: 1px solid #ffffff1a; border-radius: 30px; background: #101012; }
  .sec-h { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap; margin-bottom: 18px; }
  .sec-h :global(.rw-h2) { margin: 0; font-size: 30px; letter-spacing: -0.03em; }
  .sec-h p { margin: 4px 0 0; color: var(--muted); }
  .tabs { display: inline-flex; gap: 2px; max-width: 100%; padding: 4px; overflow-x: auto; border: 1px solid var(--line); border-radius: 999px; background: #000; }
  .tabs a { padding: 8px 16px; border-radius: 999px; color: var(--muted); font-weight: 600; font-size: 14px; white-space: nowrap; transition: background 0.2s, color 0.2s; }
  .tabs a:hover { color: var(--text); }
  .tabs a[aria-current='page'] { background: #fff; color: #000; }

  @media (min-width: 1100px) {
    .admin { display: grid; grid-template-columns: 272px minmax(0, 1fr); gap: 0 28px; max-width: 1680px; margin: auto; }
    .admin > .rw-head { grid-column: 1 / -1; padding-block: 12px 20px; }
    .rail { grid-column: 1; position: sticky; top: calc(var(--header) + 18px); align-self: start; }
    .panel { grid-column: 2; }
  }
  @media (max-width: 1099px) {
    .rail { flex-direction: row; margin: 0 calc(var(--gutter) * -1) 16px; padding: 0 var(--gutter) 4px; overflow-x: auto; }
    .nav { flex: none; grid-template-columns: 30px auto; padding: 6px 14px 6px 6px; border-radius: 99px; background: #ffffff0d; }
    .ic { grid-row: auto; width: 30px; height: 30px; border-radius: 50%; }
    .live { display: none; }
    .setup { flex: none; margin: 0; white-space: nowrap; }
  }
  @media (max-width: 640px) {
    .find { width: 100%; }
    .find kbd { display: none; }
    .panel.rw-plate { margin-inline: calc(var(--gutter) * -1); border-radius: 26px 26px 0 0; }
  }
</style>
