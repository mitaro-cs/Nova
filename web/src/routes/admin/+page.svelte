<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, errorText } from '$lib/api';
  import Access from '$lib/admin/Access.svelte';
  import { attention } from '$lib/admin/attention';
  import Catalog from '$lib/admin/Catalog.svelte';
  import Folders from '$lib/admin/Folders.svelte';
  import Live from '$lib/admin/Live.svelte';
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
  import { session } from '$lib/session.svelte';
  import { badges } from '$lib/social/badges.svelte';
  import { loadCatalog } from '$lib/catalog.svelte';
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

  <!-- Разделы текстом с белой чертой, как в шапке сайта (выбор владельца 28.11, макет «как сайт Nova»). -->
  <nav class="secs" aria-label="Разделы управления">
    {#each SECTIONS as s (s.id)}
      {@const f = flags[s.id]}
      <a href={href(s.id)} aria-current={at.section === s.id ? 'page' : undefined} data-sveltekit-replacestate data-sveltekit-noscroll>{s.label}{#if f}<em class:bad={f.bad} aria-label="требует внимания: {f.n}">{f.n}</em>{/if}</a>
    {/each}
    <a class="setup" href="?tab=setup" aria-current={at.section === 'setup' ? 'page' : undefined} data-sveltekit-replacestate data-sveltekit-noscroll><Icon name="sparkle" size={15} /> Мастер</a>
  </nav>

  <section class="panel" aria-label={section?.label ?? 'Мастер настройки'}>
    <header class="sec-h">
      <h2 class="vh">{section?.label ?? 'Мастер настройки'}</h2>
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
      {:else if k === 'summary/now'}<Live {status} {health} {reload} />
      {:else if k === 'summary/stats'}<Overview {status} {reload} />
      {:else if k === 'summary/map'}<HomeMap {status} {reload} window={!!session.me?.window} />
      {:else if k === 'viewers/people'}<People find={page.url.searchParams.get('find') ?? ''} seats={status.seats ?? null} />
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
  /* «Управление» как сайт Nova (28.11): без серых плит и боковой панели - разделы текстом с чертой, вкладки капсулами,
     содержимое прямо на чёрном. Карточки старых вкладок (`.pane`) становятся разделами с тонкой линией. */
  .admin { --line: #ffffff1a; max-width: 1680px; margin: auto; }
  .admin :global(.rw-head h1) { font-size: clamp(40px, 5.4vw, 72px); font-weight: 900; letter-spacing: -0.045em; line-height: 0.95; }
  .admin :global(.panel .pane) { margin: 0; padding: 26px 0; border: 0; border-top: 1px solid var(--line); border-radius: 0; background: none; box-shadow: none; }
  .admin :global(.panel .pane:first-child) { border-top: 0; padding-top: 4px; }
  .admin :global(.panel .pane h3) { font-size: 22px; font-weight: 800; letter-spacing: -0.02em; }
  .admin :global(.panel .input) { min-height: 44px; border-radius: 14px; }
  .rw-head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; flex-wrap: wrap; }

  .find {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    max-width: 100%;
    padding: 11px 14px 11px 18px;
    border: 0;
    border-radius: 999px;
    background: #ffffff12;
    color: var(--muted);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
    transition: background 0.2s, color 0.2s;
  }
  .find:hover { background: #ffffff1f; color: var(--text); }
  .find span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .find kbd { margin-left: 6px; padding: 3px 7px; border-radius: 6px; background: #ffffff1f; color: var(--text); font: 700 11px var(--font, inherit); }

  .secs { display: flex; gap: 30px; margin: 22px 0 0; border-bottom: 1px solid var(--line); overflow-x: auto; scrollbar-width: none; }
  .secs a { position: relative; flex: none; padding: 0 0 14px; color: #6e6e73; font-size: 18px; font-weight: 700; white-space: nowrap; transition: color 0.2s; }
  .secs a:hover { color: #d1d1d6; }
  .secs a[aria-current='page'] { color: var(--text); }
  .secs a[aria-current='page']::after { content: ''; position: absolute; left: 0; right: 0; bottom: -1px; height: 3px; border-radius: 3px; background: #fff; }
  .secs em { margin-left: 8px; padding: 1px 7px; border-radius: 99px; background: #ffd60a; color: #000; font-style: normal; font-size: 11px; font-weight: 800; vertical-align: 3px; }
  .secs em.bad { background: var(--danger); color: #fff; }
  .secs .setup { display: inline-flex; align-items: center; gap: 6px; margin-left: auto; font-size: 15px; }

  .panel { min-width: 0; padding: 22px 0 0; }
  .sec-h { margin-bottom: 24px; }
  .vh { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
  .tabs { display: flex; gap: 8px; flex-wrap: wrap; }
  .tabs a { height: 40px; display: inline-flex; align-items: center; padding: 0 18px; border-radius: 999px; background: #ffffff12; color: #d1d1d6; font-weight: 700; font-size: 14px; white-space: nowrap; transition: background 0.2s, color 0.2s; }
  .tabs a:hover { background: #ffffff1f; }
  .tabs a[aria-current='page'] { background: #fff; color: #000; }

  @media (max-width: 640px) {
    .find { width: 100%; }
    .find kbd { display: none; }
    .secs { gap: 22px; margin-inline: calc(var(--gutter) * -1); padding-inline: var(--gutter); }
    .secs a { font-size: 16px; }
    .tabs { flex-wrap: nowrap; overflow-x: auto; margin-inline: calc(var(--gutter) * -1); padding-inline: var(--gutter); scrollbar-width: none; }
  }
</style>
