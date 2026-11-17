<script lang="ts">
  import MailSettings from './MailSettings.svelte';
  import NightMode from './NightMode.svelte';
  import { api, errorText, post, put } from '$lib/api';
  import { loadMe, session } from '$lib/session.svelte';
  import LoginJournal from './LoginJournal.svelte';
  import ThemeSettings from './ThemeSettings.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';
  import type { ExtCard } from '$lib/types';
  import { artW, placeholder } from '$lib/ui/art';
  let {sourcesOnly=false}:{sourcesOnly?:boolean}=$props();

  interface S {
    metadataLanguage: 'ru' | 'source';
    siteName: string;
    registration: 'invite' | 'open' | 'closed';
    downloads: boolean;
    manageRemote: boolean;
    provider: 'imdb' | 'tmdb' | 'kinopoisk' | 'off';
    tmdbKeySet: boolean;
    kpKeySet: boolean;
    vkTokenSet: boolean;
    sites: string[];
    tmdbApi: string;
    tmdbImages: string;
    ffmpegPath: string;
    ffmpeg: string | null;
    ffprobe: string | null;
  }

  let s = $state<S | null>(null);
  let tmdbKey = $state('');
  let kpKey = $state('');
  let vkToken = $state('');
  let site = $state('');
  let sourceQuery = $state(''), sourceUrl = $state('');
  let sourceOptions = $state<ExtCard[]>([]), sourceTitle = $state<ExtCard | null>(null), sourceBusy = $state(false);
  let sourceSearching = $state(false), sourceSearchError = $state('');
  $effect(() => {
    const q = sourceQuery.trim();
    sourceOptions = []; sourceTitle = null; sourceSearchError = '';
    if (!q) { sourceSearching = false; return; }
    const controller = new AbortController();
    sourceSearching = true;
    const timer = setTimeout(() => {
      api<ExtCard[]>('/api/discover/search?q=' + encodeURIComponent(q), {signal: controller.signal})
        .then(items => { if (!controller.signal.aborted) sourceOptions = items; })
        .catch(() => { if (!controller.signal.aborted) sourceSearchError = 'Не удалось найти названия. Попробуйте ещё раз.'; })
        .finally(() => { if (!controller.signal.aborted) sourceSearching = false; });
    }, 300);
    return () => { clearTimeout(timer); controller.abort(); };
  });
  async function importSource(e: SubmitEvent) {
    e.preventDefault(); if (!sourceTitle || !sourceUrl.trim() || sourceBusy) return;
    sourceBusy = true; error = msg = '';
    try { const result = await post<{count:number}>('/api/admin/video-source', {extId: sourceTitle.id, url: sourceUrl.trim()});
      msg = 'Добавлено источников: ' + result.count + '. Они выбраны для всех зрителей.'; sourceUrl = '';
    } catch (e) { error = errorText(e); } finally { sourceBusy = false; }
  }
  let msg = $state('');
  let error = $state('');

  function load() {
    api<S>('/api/admin/settings').then((x) => (s = x), (e) => (error = errorText(e)));
  }
  $effect(load);

  async function save(body: Record<string, unknown>, note = 'Сохранено') {
    error = msg = '';
    try {
      await put('/api/admin/settings', body);
      msg = note;
      load();
      loadMe();
      setTimeout(() => (msg = ''), 2000);
    } catch (x) {
      error = errorText(x);
    }
  }

  // Свои видеосайты: ссылки с них открываются в плеере Nova (видео ищется на странице).
  function addSite(e: SubmitEvent) {
    e.preventDefault();
    const v = site.trim();
    if (!s || !v) return;
    save({ sites: [...s.sites, v] }, 'Сайт добавлен');
    site = '';
  }
  function dropSite(d: string) {
    if (s) save({ sites: s.sites.filter((x) => x !== d) }, 'Сайт убран');
  }

  async function refreshAll() {
    const r = await post<{ count: number }>('/api/admin/meta/refresh').catch(() => null);
    msg = r ? `Обновляем описания: ${r.count}` : '';
  }
</script>

{#if s}
  {#if !sourcesOnly}<NightMode />{/if}
  {#if msg}<p class="ok">{msg}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}

  {#if !sourcesOnly}<section class="card pane">
    <h3>Сайт</h3>
    <p class="muted small">Язык интерфейса и описаний — русский. Оригинальные имена сохраняются в метаданных.</p>
    <form class="line" onsubmit={(e) => (e.preventDefault(), save({ siteName: s!.siteName }))}>
      <label class="field grow"><span>Название</span><input class="input" bind:value={s.siteName} maxlength="60" /></label>
      <button class="btn">Сохранить</button>
    </form>
    <label class="field">
      <span>Регистрация</span>
      <Select
        label="Регистрация"
        value={s.registration}
        onchange={(v) => save({ registration: v })}
        options={[
          { value: 'invite', label: 'Только по приглашению' },
          { value: 'open', label: 'Открытая - любой, у кого есть адрес' },
          { value: 'closed', label: 'Закрыта - только те, кого добавили вы' },
        ]}
      />
    </label>
    <label class="check">
      <input type="checkbox" checked={s.downloads} onchange={(e) => save({ downloads: (e.target as HTMLInputElement).checked })} />
      Кнопка «Скачать» у фильмов
    </label>
    <p class="muted hint">Выключено - кнопки нет. Совсем запретить сохранить фильм нельзя: браузер получает видео целиком, пока его смотрят.</p>
  </section>

  <section class="card pane">
    <h3>Описания, постеры и актёры</h3>
    <div class="providers">
      {#each [['imdb', 'IMDb', 'Без ключа. Рейтинг IMDb, актёры, кадры; сюжет - на английском'], ['tmdb', 'TMDB', 'Нужен бесплатный ключ. Описания на русском, фоны, логотипы'], ['kinopoisk', 'Кинопоиск', 'Ключ api.kinopoisk.dev. Всё на русском, 200 запросов в день'], ['off', 'Выключено', 'Только названия из имён файлов']] as [id, name, note] (id)}
        <button class="prov" class:on={s.provider === id} onclick={() => save({ provider: id })}>
          <b>{name}</b><span class="muted">{note}</span>
        </button>
      {/each}
    </div>
    {#if s.provider !== 'tmdb'}
      <p class="muted hint">Русские постеры в Обзоре вместо рекламных плакатов IMDb - с бесплатным ключом TMDB (описания остаются из выбранного источника).</p>
    {/if}
    {#if s.provider !== 'off'}
      <form class="line" onsubmit={(e) => (e.preventDefault(), save({ tmdbKey }, 'Ключ сохранён'), (tmdbKey = ''))}>
        <label class="field grow">
          <span>Ключ TMDB (API key или Read Access Token) {s.tmdbKeySet ? '· задан' : ''}</span>
          <input class="input" bind:value={tmdbKey} placeholder={s.tmdbKeySet ? '••••••••' : 'themoviedb.org → Настройки → API'} autocomplete="off" />
        </label>
        <button class="btn">Сохранить</button>
      </form>
      <details>
        <summary class="muted">Зеркала TMDB (если сайт не открывается)</summary>
        <form class="line" onsubmit={(e) => (e.preventDefault(), save({ tmdbApi: s!.tmdbApi, tmdbImages: s!.tmdbImages }))}>
          <input class="input grow" bind:value={s.tmdbApi} placeholder="https://api.themoviedb.org/3" />
          <input class="input grow" bind:value={s.tmdbImages} placeholder="https://image.tmdb.org/t/p" />
          <button class="btn">Сохранить</button>
        </form>
      </details>
    {/if}
    {#if s.provider === 'kinopoisk'}
      <form class="line" onsubmit={(e) => (e.preventDefault(), save({ kpKey }, 'Ключ сохранён'), (kpKey = ''))}>
        <label class="field grow">
          <span>Ключ api.kinopoisk.dev {s.kpKeySet ? '· задан' : ''}</span>
          <input class="input" bind:value={kpKey} placeholder={s.kpKeySet ? '••••••••' : 'Ключ из бота @kinopoiskdev_bot'} autocomplete="off" />
        </label>
        <button class="btn">Сохранить</button>
      </form>
    {/if}
    <button class="btn sm" onclick={refreshAll}><Icon name="refresh" size={14} /> Обновить все описания</button>
  </section>

  {/if}
  {#if sourcesOnly}<section class="card pane">
    <h3>Видео по ссылке</h3>
    <p class="muted">
      Поиск на Rutube работает без ключа. Для поиска в VK Видео нужен сервисный ключ приложения VK
      (vk.com/apps?act=manage → своё приложение → «Сервисный ключ доступа»).
    </p>
    <form class="line" onsubmit={(e) => (e.preventDefault(), save({ vkToken }, 'Ключ сохранён'), (vkToken = ''))}>
      <label class="field grow">
        <span>Ключ VK {s.vkTokenSet ? '· задан' : ''}</span>
        <input class="input" bind:value={vkToken} placeholder={s.vkTokenSet ? '••••••••' : 'Сервисный ключ доступа'} autocomplete="off" />
      </label>
      <button class="btn">Сохранить</button>
    </form>
  </section>

  <section class="card pane" aria-labelledby="video-import">
    <h3 id="video-import">Источник для фильма или сериала</h3>
    <p class="muted">Найдите фильм, сериал или аниме, затем вставьте ссылку на видео или страницу. Для поддерживаемого сериала Nova загрузит доступные серии и выберет их для всех зрителей.</p>
    <label class="field"><span>Название</span><input class="input" bind:value={sourceQuery} placeholder="Название фильма или сериала" autocomplete="off" /></label>
    {#if sourceSearching}<p class="muted small" role="status">Ищем названия…</p>{:else if sourceSearchError}<p class="error" role="alert">{sourceSearchError}</p>{:else if sourceQuery.trim() && !sourceOptions.length}<p class="muted small">Ничего не найдено</p>{/if}
    {#if sourceOptions.length}<div class="source-results" role="listbox" aria-label="Названия из каталога">{#each sourceOptions as item (item.id)}<button type="button" class="source-result" role="option" aria-selected={sourceTitle?.id === item.id} onclick={() => sourceTitle = item}><span class="source-poster" style:background={placeholder(item.title)}>{#if item.poster}<img src={artW(item.poster, 160)} alt="" loading="lazy" onerror={(e) => (e.currentTarget as HTMLImageElement).style.visibility = 'hidden'} />{/if}</span><span><b>{item.title}</b><small>{item.year ?? 'Год неизвестен'} · {item.kind === 'series' ? 'сериал' : 'фильм'}</small></span></button>{/each}</div>{/if}
    <form class="line" onsubmit={importSource}>
      <label class="field grow"><span>Ссылка на видео или страницу сериала</span><input class="input" type="url" bind:value={sourceUrl} placeholder="https://…" required /></label>
      <button class="btn" disabled={!sourceTitle || !sourceUrl.trim() || sourceBusy}>{sourceBusy ? 'Загружаем…' : 'Добавить и выбрать'}</button>
    </form>
    <p class="muted">Автоматический импорт серий поддерживает страницы LordFilm с доступным списком эпизодов. Для остальных сайтов поддержка зависит от формата.</p>
  </section>

  <section class="card pane" aria-labelledby="own-sites">
    <h3 id="own-sites">Свои сайты</h3>
    <p class="muted">
      Сайты с видео, которым вы доверяете: ссылку со страницы такого сайта можно вставить у фильма - Nova найдёт
      на странице видео и откроет его в своём плеере (через этот компьютер). Не нашлось - ссылка откроется на сайте.
    </p>
    {#if s.sites.length}
      <ul class="sites">
        {#each s.sites as d (d)}
          <li><Icon name="globe" size={15} /><span>{d}</span>{#if session.me?.user.role === 'host'}<button class="btn sm ghost" aria-label="Убрать {d}" onclick={() => dropSite(d)}><Icon name="close" size={14} /></button>{/if}</li>
        {/each}
      </ul>
    {/if}
    {#if session.me?.user.role === 'host'}
      <form class="line" onsubmit={addSite}>
        <label class="field grow"><span>Адрес сайта</span><input class="input" bind:value={site} placeholder="kino.example или https://kino.example/film/1" autocomplete="off" /></label>
        <button class="btn" disabled={!site.trim()}><Icon name="plus" size={14} /> Добавить</button>
      </form>
    {:else}
      <p class="muted small">Сайты выбирает только хост.</p>
    {/if}
  </section>

  {/if}
  {#if !sourcesOnly}<a class="btn" href="/admin?tab=catalog#sources">Источники фильмов, сериалов и аниме →</a>

  <section class="card pane">
    <h3>ffmpeg</h3>
    <p class="muted">
      {#if s.ffmpeg}Найден: <code>{s.ffmpeg}</code>{:else}Не найден. Без него браузер играет только MP4 и WebM.{/if}
    </p>
    <form class="line" onsubmit={(e) => (e.preventDefault(), save({ ffmpegPath: s!.ffmpegPath }))}>
      <input class="input grow" bind:value={s.ffmpegPath} placeholder="Путь к ffmpeg (если не нашёлся сам)" />
      <button class="btn">Сохранить</button>
    </form>
  </section>

  <ThemeSettings />

  <section class="card pane">
    <h3>Безопасность</h3>
    <label class="check">
      <input type="checkbox" checked={s.manageRemote} disabled={!session.me?.window} onchange={(e) => save({ manageRemote: (e.target as HTMLInputElement).checked })} />
      «Управление» не только в окне приложения
    </label>
    <p class="muted hint">
      Выключено - управлять можно только здесь, на этом компьютере. Включено - и с телефона, но только тем, у кого в профиле включён вход в два шага.
      {#if !session.me?.window}Менять можно только в окне приложения.{/if}
    </p>
    <LoginJournal />
  </section>
  {/if}
{/if}

<MailSettings />

<style>
  .source-results{display:grid;gap:4px;max-height:350px;overflow:auto;border:1px solid var(--line);border-radius:14px;padding:6px}.source-result{display:flex;align-items:center;gap:12px;width:100%;padding:7px;border:0;border-radius:9px;background:transparent;color:var(--text);text-align:left;font:inherit;cursor:pointer}.source-result:hover,.source-result[aria-selected=true]{background:var(--surface-3)}.source-poster{width:40px;height:58px;flex:none;border-radius:5px;overflow:hidden}.source-poster img{width:100%;height:100%;object-fit:cover}.source-result small{display:block;color:var(--muted);margin-top:4px}
  .hint {
    margin: -4px 0 0;
    font-size: 12.5px;
  }
  .pane {
    display: grid;
    gap: 14px;
    padding: 18px 20px;
    margin-bottom: 14px;
  }
  h3 {
    font-size: 16px;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 10px;
  }
  .grow {
    flex: 1 1 220px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 14px;
  }
  .providers {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
  }
  .prov {
    display: grid;
    gap: 4px;
    padding: 14px;
    text-align: left;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: rgba(0, 0, 0, 0.2);
    cursor: pointer;
  }
  .prov span {
    font-size: 12px;
  }
  .prov.on {
    border-color: var(--accent-2);
    background: rgba(255, 255, 255, 0.14);
  }
  .pane > .btn {
    justify-self: start;
  }
  p {
    margin: 0;
  }
  code {
    /* Путь Windows длинный и без пробелов - переносится где угодно, иначе распирал страницу. */
    overflow-wrap: anywhere;
    padding: 1px 6px;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.35);
    font-size: 13px;
  }
  .ok {
    color: var(--ok);
  }
  details summary {
    cursor: pointer;
    font-size: 13px;
    margin-bottom: 10px;
  }
  .sites {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .sites li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 6px 6px 12px;
    border-radius: 10px;
    background: var(--surface-2);
    min-width: 0;
  }
  .sites span {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
