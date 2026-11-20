<script lang="ts">
  import { api, errorText, post } from '$lib/api';
  import type { ExtCard, FoundVideo } from '$lib/types';
  import { artW } from '$lib/ui/art';
  import { isAnime } from '$lib/library';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';
  import { extSheet } from '$lib/discover/sheet.svelte';

  type Catalog = { domain: string; name: string; kind: string; enabled: boolean };
  let catalogs = $state<Catalog[]>([]);
  let query = $state('');
  let results = $state<ExtCard[]>([]);
  let streams = $state<FoundVideo[]>([]);
  let selected = $state<ExtCard | null>(null);
  let address = $state('');
  let sourceDomain = $state('');
  let season = $state('1');
  let episode = $state('1');
  let searching = $state(false);
  let findingStreams = $state(false);
  let busy = $state(false);
  let error = $state('');
  let message = $state('');
  let controller: AbortController | null = null;
  const series = $derived(!!selected && selected.kind !== 'movie');
  const enabled = $derived(catalogs.filter((c) => c.enabled));

  $effect(() => {
    let live = true;
    api<{ items: Catalog[] }>('/api/admin/catalogs').then((r) => {
      if (!live) return;
      catalogs = r.items;
      if (!sourceDomain || !r.items.some((c) => c.domain === sourceDomain && c.enabled)) {
        sourceDomain = r.items.find((c) => c.enabled)?.domain ?? '';
      }
    }, (e) => { if (live) error = errorText(e); });
    return () => { live = false; };
  });

  $effect(() => {
    const q = query.trim();
    controller?.abort();
    results = [];
    if (q.length < 2) { searching = false; return; }
    const abort = new AbortController();
    controller = abort;
    const timer = setTimeout(async () => {
      searching = true;
      error = '';
      try {
        const found = await api<ExtCard[]>(`/api/discover/search?q=${encodeURIComponent(q)}`, { signal: abort.signal });
        if (!abort.signal.aborted) results = found.filter((card) => !card.local || card.id.startsWith('tt')).slice(0, 12);
      } catch (e) {
        if (!abort.signal.aborted) error = errorText(e);
      } finally {
        if (!abort.signal.aborted) searching = false;
      }
    }, 250);
    return () => { clearTimeout(timer); abort.abort(); };
  });

  function choose(card: ExtCard) {
    selected = card;
    results = [];
    streams = [];
    error = message = '';
    if (enabled.length) setTimeout(() => void findStreams(), 0);
  }

  async function findStreams() {
    if (!selected || findingStreams) return;
    const s = series ? Number(season) : null;
    const ep = series ? Number(episode) : null;
    if (series && (!Number.isInteger(s) || s! < 0 || !Number.isInteger(ep) || ep! < 1)) {
      error = 'Укажите сезон и серию, чтобы искать точный эпизод';
      return;
    }
    findingStreams = true;
    error = message = '';
    streams = [];
    try {
      const params = new URLSearchParams({ q: selected.title, extId: selected.id, provider: 'own' });
      if (series) { params.set('season', String(s)); params.set('episode', String(ep)); }
      streams = await api<FoundVideo[]>(`/api/links/search?${params}`);
      if (!streams.length) message = 'Подключённые сайты пока не вернули подходящих вариантов для этого тайтла.';
    } catch (e) {
      error = errorText(e);
    } finally {
      findingStreams = false;
    }
  }

  function chooseStream(stream: FoundVideo) {
    address = stream.url;
    if (stream.site === 'own' && stream.author && enabled.some((c) => c.domain === stream.author)) sourceDomain = stream.author;
    message = `Выбран вариант: ${stream.label}`;
    error = '';
  }

  async function attach(e: SubmitEvent) {
    e.preventDefault();
    if (!selected || !sourceDomain || busy) return;
    const url = address.trim();
    try {
      const parsed = new URL(url);
      if (!['http:', 'https:'].includes(parsed.protocol)) throw new Error('Нужна ссылка с http или https');
    } catch (e) {
      error = e instanceof Error && e.message.includes('Нужна ссылка') ? e.message : 'Введите полную ссылку на страницу или видео';
      return;
    }
    const s = series ? Number(season) : null;
    const ep = series ? Number(episode) : null;
    if (series && (!Number.isInteger(s) || s! < 0 || !Number.isInteger(ep) || ep! < 1)) {
      error = 'Укажите сезон и серию';
      return;
    }
    busy = true;
    error = message = '';
    try {
      await post('/api/links', {
        url,
        titleId: selected.local ?? null,
        extId: selected.local ? null : selected.id,
        sourceDomain,
        label: streams.find((stream) => stream.url === url)?.label ?? selected.title,
        thumb: streams.find((stream) => stream.url === url)?.thumbUrl ?? selected.poster,
        season: s,
        episode: ep,
      });
      message = `Источник добавлен к «${selected.title}»${series ? ` · ${s} сезон, ${ep} серия` : ''}`;
      address = '';
      extSheet.version++;
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="attach card" aria-labelledby="attach-title">
  <header class="heading">
    <span class="icon"><Icon name="link" size={20} /></span>
    <div class="headline">
      <span class="eyebrow">ПРИВЯЗКА ФИЛЬМОВ</span>
      <h4 id="attach-title">Источник для конкретного тайтла</h4>
      <p class="muted">Найдите фильм, сериал или аниме в IMDb, проверьте постер и привяжите ссылку к выбранному сайту.</p>
    </div>
    <ol class="steps" aria-label="Шаги привязки">
      <li class:on={!selected}><b>1</b><span>Найти</span></li>
      <li class:on={!!selected && !address.trim()}><b>2</b><span>Выбрать</span></li>
      <li class:on={!!address.trim()}><b>3</b><span>Привязать</span></li>
    </ol>
  </header>

  <div class="search-block">
    <label class="field search"><span>Поиск по названию</span><span class="search-box"><Icon name="search" size={16} /><input class="input" bind:value={query} placeholder="Например, Дороро или Бойцовский клуб" autocomplete="off" /></span></label>
    {#if searching}<p class="muted small status-line" role="status"><i class="spinner"></i>Ищем в IMDb…</p>{/if}
  </div>

  {#if results.length}
    <div class="results" aria-label="Результаты поиска">
      {#each results as card (card.id)}
        <button type="button" class="result" class:selected={selected?.id === card.id} onclick={() => choose(card)}>
          <span class="poster">{#if card.poster}<img src={artW(card.poster, 220)} alt="" loading="lazy" decoding="async" />{:else}<Icon name="film" size={26} />{/if}</span>
          <span class="result-copy"><b>{card.title}</b><small>{card.year ?? 'Год неизвестен'} · {isAnime(card) ? 'Аниме' : card.kind === 'movie' ? 'Фильм' : 'Сериал'}</small></span>
          {#if selected?.id === card.id}<span class="tick"><Icon name="check" size={14} /></span>{/if}
        </button>
      {/each}
    </div>
  {/if}

  {#if selected}
    <form class="form" onsubmit={attach}>
      <div class="selected">
        <span class="poster large">{#if selected.poster}<img src={artW(selected.poster, 320)} alt="Постер: {selected.title}" />{:else}<Icon name="film" size={30} />{/if}</span>
        <span class="selected-copy"><b>{selected.title}</b><small>{selected.year ?? ''} · {isAnime(selected) ? 'Аниме' : selected.kind === 'movie' ? 'Фильм' : 'Сериал'} · IMDb</small></span>
        <button type="button" class="btn sm" onclick={() => (selected = null)}>Выбрать другой</button>
      </div>

      <div class="finder">
        <button type="button" class="btn" disabled={findingStreams || !enabled.length || (series && (!Number.isInteger(Number(season)) || !Number.isInteger(Number(episode))))} onclick={findStreams}>
          <Icon name="search" size={15} />{findingStreams ? 'Ищем на сайтах…' : 'Найти на подключённых сайтах'}
        </button>
        {#if streams.length}
          <div class="streams" aria-label="Найденные варианты видео">
            {#each streams as stream (stream.url)}
              <button type="button" class="stream" class:selected={address === stream.url} onclick={() => chooseStream(stream)}>
                <span class="still">{#if stream.thumbUrl || selected.poster}<img src={artW(stream.thumbUrl || selected.poster, 320)} alt="" loading="lazy" decoding="async" />{:else}<Icon name="play" size={22} />{/if}</span>
                <span class="stream-copy"><b>{stream.label}</b><small>{stream.author || stream.site}{stream.duration ? ` · ${Math.round(stream.duration / 60)} мин` : ''}</small></span>
                <span class="pick"><Icon name={address === stream.url ? 'check' : 'plus'} size={17} /></span>
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="fields">
        <label class="field"><span>Сайт из каталога</span>
          <Select label="Сайт источника" value={sourceDomain} onchange={(v) => sourceDomain = v} options={enabled.map((c) => ({ value: c.domain, label: `${c.name || c.domain} · ${c.domain}` }))} />
        </label>
        <label class="field wide"><span>Страница тайтла или прямая ссылка на видео</span><input class="input" bind:value={address} placeholder="https://site.example/movie-or-episode" inputmode="url" required /></label>
        {#if series}<div class="numbers"><label class="field"><span>Сезон</span><input class="input" type="number" min="0" max="1000" bind:value={season} required /></label><label class="field"><span>Серия</span><input class="input" type="number" min="1" max="10000" bind:value={episode} required /></label></div>{/if}
      </div>
      <button class="btn primary attach-btn" disabled={busy || !sourceDomain || !address.trim()}><Icon name="plus" size={15} />{busy ? 'Добавляем…' : 'Добавить источник'}</button>
    </form>
  {:else if query.trim().length >= 2 && !searching && !results.length && !error}
    <p class="notice muted small">Ничего не найдено. Уточните название или попробуйте оригинальное.</p>
  {/if}
  {#if !enabled.length}<p class="notice muted small">Сначала подключите сайт-источник выше в каталоге.</p>{/if}
  {#if message}<p class="success" role="status"><Icon name="check" size={15} />{message}</p>{/if}
  {#if error}<p class="error" role="alert"><Icon name="info" size={15} />{error}</p>{/if}
</section>

<style>
  h4, p { margin: 0; }

  .attach {
    display: grid;
    gap: 18px;
    padding: 20px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 22px;
  }

  .heading {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    flex: none;
    color: var(--text);
    background: var(--surface-3);
    border: 1px solid var(--line);
    border-radius: 14px;
  }
  .headline { display: grid; gap: 6px; min-width: 0; }
  .eyebrow {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.14em;
    color: var(--faint);
  }
  .heading h4 { font-size: 17px; letter-spacing: -0.02em; }
  .heading p { max-width: 660px; line-height: 1.5; }

  .steps { display: flex; gap: 8px; margin: 2px 0 0 auto; padding: 0; list-style: none; }
  .steps li {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 11px 6px 6px;
    border: 1px solid var(--line);
    border-radius: 999px;
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
    opacity: 0.55;
    transition: opacity 0.18s, background-color 0.18s;
  }
  .steps li.on { opacity: 1; background: #ffffff12; color: var(--text); }
  .steps b {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--surface-3);
    font-size: 10px;
  }
  .steps li.on b { background: var(--text); color: #0a0a0a; }

  .field { display: grid; gap: 7px; min-width: 0; font-size: 12px; }
  .search-box { position: relative; display: block; }
  .search-box :global(svg) {
    position: absolute;
    top: 50%;
    left: 14px;
    color: var(--faint);
    transform: translateY(-50%);
    pointer-events: none;
  }
  .search-box .input { padding-left: 40px; }
  .status-line { display: inline-flex; align-items: center; gap: 8px; }
  .spinner {
    width: 11px;
    height: 11px;
    border: 2px solid var(--line-strong);
    border-top-color: var(--text);
    border-radius: 50%;
    animation: nova-spin 0.8s linear infinite;
  }
  @keyframes nova-spin { to { transform: rotate(360deg); } }

  .results { display: grid; grid-template-columns: repeat(auto-fill, minmax(196px, 1fr)); gap: 10px; }
  .result {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 9px;
    text-align: left;
    color: var(--text);
    background: var(--surface-3);
    border: 1px solid var(--line);
    border-radius: 14px;
    cursor: pointer;
    transition: transform 0.18s, border-color 0.18s, background-color 0.18s;
  }
  .result:hover { transform: translateY(-2px); border-color: var(--line-strong); }
  .result.selected { border-color: var(--text); background: #ffffff0f; }
  .result-copy, .selected-copy, .stream-copy { display: grid; gap: 4px; min-width: 0; }
  .result b, .selected b, .stream b { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .result small, .selected small, .stream small { font-size: 11px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .poster {
    display: grid;
    place-items: center;
    flex: none;
    width: 46px;
    aspect-ratio: 2 / 3;
    overflow: hidden;
    color: var(--faint);
    background: var(--surface);
    border-radius: 9px;
  }
  .poster img, .still img { width: 100%; height: 100%; object-fit: cover; }
  .poster.large { width: 64px; border-radius: 11px; }
  .tick {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 19px;
    height: 19px;
    color: #0a0a0a;
    background: var(--text);
    border-radius: 50%;
  }

  .form { display: grid; gap: 14px; }
  .selected {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    background: linear-gradient(90deg, #ffffff0d, transparent);
    border: 1px solid var(--line);
    border-radius: 16px;
  }
  .selected-copy { flex: 1; }
  .selected .btn { margin-left: auto; }

  .finder { display: grid; gap: 10px; }
  .finder > .btn { justify-self: start; }
  .streams { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 8px; }
  .stream {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 8px;
    text-align: left;
    color: var(--text);
    background: var(--surface-3);
    border: 1px solid var(--line);
    border-radius: 12px;
    cursor: pointer;
    transition: transform 0.18s, border-color 0.18s;
  }
  .stream:hover { transform: translateY(-2px); border-color: var(--line-strong); }
  .stream.selected { border-color: var(--text); background: #ffffff0f; }
  .pick {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 50%;
  }
  .stream.selected .pick { color: #0a0a0a; background: var(--text); border-color: var(--text); }
  .still {
    display: grid;
    place-items: center;
    width: 56px;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    color: var(--faint);
    background: var(--surface);
    border-radius: 8px;
  }

  .fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .fields .wide { grid-column: 1 / -1; }
  .numbers { display: grid; grid-column: 1 / -1; grid-template-columns: 1fr 1fr; gap: 12px; }
  .attach-btn { justify-self: end; min-width: 240px; }

  .notice, .success, .error {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 9px 13px;
    border-radius: 999px;
    line-height: 1.4;
  }
  .notice { background: var(--surface-3); border: 1px solid var(--line); }
  .success { color: #8cdaaa; background: #8cdaaa14; border: 1px solid #8cdaaa40; }
  .error { color: var(--error, #f99); background: #f9999912; border: 1px solid #f9999940; }

  @media (max-width: 980px) {
    .steps { display: none; }
  }
  @media (max-width: 700px) {
    .attach { gap: 14px; padding: 15px; }
    .heading { gap: 11px; }
    .icon { width: 38px; height: 38px; }
    .results { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .streams { grid-template-columns: 1fr; }
    .fields { grid-template-columns: 1fr; }
    .selected { flex-wrap: wrap; }
    .selected .btn { margin-left: 0; }
    .attach-btn { width: 100%; min-width: 0; justify-self: stretch; }
  }
  @media (max-width: 430px) {
    .results { grid-template-columns: 1fr; }
  }
</style>
