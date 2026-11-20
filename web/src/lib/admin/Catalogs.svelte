<script lang="ts">
  import { api, errorText, post, put } from '$lib/api';
  import { session } from '$lib/session.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';
  import SiteSources from './SiteSources.svelte';
  import HostTitleSource from './HostTitleSource.svelte';
  import { extSheet } from '$lib/discover/sheet.svelte';

  // Каталоги: сайты, где много фильмов, серий-франшиз и сериалов. Хост добавляет сайт - Nova сама находит
  // у него поиск (или хост вписывает адрес поиска с {q}) и ищет там фильм, который открыт у зрителя.
  type Catalog = { domain: string; name: string; kind: string; manifest: string | null; search: string | null; enabled: boolean };
  const kinds = [{value:'all',label:'Всё'}, {value:'movie',label:'Фильмы'}, {value:'series',label:'Сериалы'}, {value:'anime',label:'Аниме'}];
  let items = $state<Catalog[] | null>(null);
  let domain = $state('');
  let busy = $state(false);
  let msg = $state('');
  let error = $state('');
  let checking = $state('');
  let checks = $state<Record<string, string>>({});
  const host = $derived(session.me?.user.role === 'host');

  function load() {
    api<{ items: Catalog[] }>('/api/admin/catalogs').then((r) => (items = r.items), (e) => (error = errorText(e)));
  }
  $effect(load);

  async function store(next: Catalog[], note: string) {
    error = msg = '';
    try {
      const r = await put<{ items: Catalog[] }>('/api/admin/catalogs', { items: next });
      items = r.items;
      msg = note;
      setTimeout(() => (msg = ''), 4000);
    } catch (x) {
      error = errorText(x);
    }
  }

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const v = domain.trim();
    if (!items || !v || busy) return;
    busy = true;
    error = msg = '';
    try {
      const found = await post<{ domain: string; search: string | null }>('/api/admin/catalogs/probe', { domain: v });
      await store(
        [...items, { domain: found.domain, name: '', kind: 'all', manifest: null, search: found.search, enabled: true }],
        found.search ? 'Каталог добавлен, поиск найден' : 'Каталог добавлен, но поиск найти не удалось - впишите адрес поиска у этого сайта',
      );
      domain = '';
    } catch (x) {
      error = errorText(x);
    } finally {
      busy = false;
    }
  }

  const drop = (d: string) => items && store(items.filter((c) => c.domain !== d), 'Каталог убран');
  const setSearch = (d: string, search: string) =>
    items && store(items.map((c) => (c.domain === d ? { ...c, search: search.trim() || null } : c)), 'Сохранено');
  function move(index: number, delta: number) {
    if (!items) return;
    const to = index + delta;
    if (to < 0 || to >= items.length) return;
    const next = [...items];
    [next[index], next[to]] = [next[to], next[index]];
    void store(next, 'Порядок сохранён');
  }
  async function check(c: Catalog) {
    if (checking) return;
    checking = c.domain;
    try {
      if(c.manifest) {
        const result = await post<{titles:number;variants:number}>('/api/admin/catalogs/check', {domain:c.domain});
        checks[c.domain] = `JSON доступен: ${result.titles} тайтлов, ${result.variants} вариантов. Воспроизведение ещё не проверено`;
        return;
      }
      const result = await post<{ search: string | null }>('/api/admin/catalogs/probe', { domain: c.domain });
      checks[c.domain] = result.search ? 'Сайт ответил, найден шаблон поиска' : 'Поиск не найден или сайт недоступен';
    } catch (e) {
      checks[c.domain] = errorText(e);
    } finally {
      checking = '';
    }
  }

  async function importCatalog(c: Catalog, file?: File) {
    if (busy) return;
    busy = true; error = msg = '';
    try {
      if (file && file.size > 8 * 1024 * 1024) throw new Error('JSON-каталог должен быть меньше 8 МБ');
      const manifest = file ? JSON.parse(await file.text()) : undefined;
      const result = await post<{titles:number;added:number;updated:number}>('/api/admin/catalogs/import', {domain:c.domain, manifest});
      msg = `Импортировано тайтлов: ${result.titles}. Новых вариантов: ${result.added}, обновлено: ${result.updated}`;
      extSheet.version++;
    } catch (e) { error = errorText(e); }
    finally { busy = false; }
  }

  function details(e: SubmitEvent, c: Catalog) {
    e.preventDefault();
    const data = new FormData(e.currentTarget as HTMLFormElement);
    if (items) void store(items.map(item => item.domain === c.domain ? {...item,
      name: String(data.get('name') ?? '').trim(), manifest: String(data.get('manifest') ?? '').trim() || null} : item), 'Настройки источника сохранены');
  }
</script>

<section class="card pane" aria-labelledby="catalogs">
  <h3 id="catalogs">Каталоги фильмов и сериалов</h3>
  <p class="muted">
    Добавьте сайт для поиска или импортируйте JSON-каталог с фильмами, сериями и вариантами видео.
    Источники выше в списке открываются первыми. Выбор администратора имеет приоритет.
  </p>
  {#if host}
    <SiteSources onchanged={load} />
    <HostTitleSource />
  {/if}
  {#if items === null}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  {:else}
    {#if items.length}
      <ul class="list">
        {#each items as c, i (c.domain)}
          <li>
            <div class="head">
              <Icon name="globe" size={15} /><b>{c.name || c.domain}</b>
              <span class="state" class:on={!!c.search}>{c.search ? 'поиск настроен' : 'нет адреса поиска'}</span>
              {#if host}
                <button class="btn sm ghost" aria-label="{c.enabled ? 'Выключить' : 'Включить'} {c.domain}" onclick={() => store(items!.map((x) => x.domain === c.domain ? { ...x, enabled: !x.enabled } : x), 'Сохранено')}>{c.enabled ? 'Вкл.' : 'Выкл.'}</button>
                <button class="btn sm ghost" disabled={i === 0} aria-label="Поднять {c.domain}" onclick={() => move(i, -1)}>↑</button>
                <button class="btn sm ghost" disabled={i === items.length - 1} aria-label="Опустить {c.domain}" onclick={() => move(i, 1)}>↓</button>
              {/if}
              {#if host}<button class="btn sm ghost" aria-label="Убрать {c.domain}" onclick={() => drop(c.domain)}><Icon name="close" size={14} /></button>{/if}
            </div>
            {#if host}
              <form class="line" onsubmit={(e) => details(e, c)}>
                <label class="field grow"><span>Название источника</span><input class="input" name="name" value={c.name} placeholder={c.domain} /></label>
                <label class="field grow"><span>JSON-каталог</span><input class="input" name="manifest" value={c.manifest ?? ''} placeholder="https://example.org/catalog.json" /></label>
                <button class="btn sm" disabled={busy}>Применить</button>
              </form>
              <div class="line">
                <Select value={c.kind} options={kinds} label="Тип каталога {c.domain}" disabled={busy}
                  onchange={(kind) => items && store(items.map(item => item.domain === c.domain ? {...item, kind} : item), 'Тип сохранён')} />
                <button class="btn sm" disabled={busy || !c.enabled || !c.manifest} onclick={() => importCatalog(c)}>Обновить из JSON</button>
                <label class="field grow file-import"><span>Импорт JSON-файла {c.domain}</span><span class="btn sm">Загрузить JSON-файл</span><input class="file-input" type="file" accept="application/json,.json" disabled={busy || !c.enabled}
                  onchange={(e) => { const file = e.currentTarget.files?.[0]; e.currentTarget.value = ''; if(file) void importCatalog(c,file); }} /></label>
              </div>
              <div><button class="btn sm ghost" disabled={!!checking} onclick={() => check(c)}>{checking === c.domain ? 'Проверяем…' : 'Проверить'}</button> {#if checks[c.domain]}<span class="muted small" role="status">{checks[c.domain]}</span>{/if}</div>
              <form class="line" onsubmit={(e) => (e.preventDefault(), setSearch(c.domain, new FormData(e.currentTarget as HTMLFormElement).get('search')?.toString() ?? ''))}>
                <input class="input grow" name="search" value={c.search ?? ''} placeholder="https://{c.domain}/?s={'{q}'}" aria-label="Адрес поиска {c.domain}" autocomplete="off" />
                <button class="btn sm">Сохранить</button>
              </form>
            {:else if c.search}
              <code class="selectable">{c.search}</code>
            {/if}
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted">Каталогов пока нет.</p>
    {/if}
    {#if host}
      <form class="line" onsubmit={add}>
        <label class="field grow"><span>Адрес сайта</span><input class="input" bind:value={domain} placeholder="kino.example" autocomplete="off" /></label>
        <button class="btn" disabled={!domain.trim() || busy}><Icon name="plus" size={14} /> {busy ? 'Ищем поиск…' : 'Добавить'}</button>
      </form>
      <p class="muted small">
        Адрес поиска - это адрес страницы результатов на самом сайте, где вместо названия стоит <code>{'{q}'}</code>. Узнать
        его просто: найдите на сайте любой фильм и скопируйте адрес страницы результатов, заменив название на <code>{'{q}'}</code>.
      </p>
      <details><summary>Формат каталога и доступные варианты</summary>
        <p class="muted small">JSON версии 1: items содержит тайтлы с imdbId, title, kind (movie, series, anime) и streams.
          У каждого варианта свой id, url (MP4, WebM или HLS), label, quality и voice. Для серий укажите season и episode.
          HLS-мастер передаётся целиком. Повторный импорт обновляет варианты по id и сохраняет прогресс. Отсутствующие записи не удаляются.
          Подключайте собственные, открытые или лицензированные источники. Локальные папки добавляются в «Управление - Папки».</p>
      </details>
    {:else}
      <p class="muted small">Каталоги добавляет только хост.</p>
    {/if}
    {#if msg}<p class="ok" role="status">{msg}</p>{/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  {/if}
</section>

<style>
  .pane {
    display: grid;
    gap: 14px;
    padding: 18px 20px;
    margin-bottom: 14px;
  }
  h3 {
    font-size: 16px;
  }
  p {
    margin: 0;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .list li {
    display: grid;
    gap: 8px;
    padding: 10px 10px 10px 14px;
    border-radius: 12px;
    background: var(--surface-2);
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    flex-wrap: wrap;
  }
  .head b {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .state {
    flex: none;
    padding: 3px 10px;
    border-radius: 99px;
    background: var(--surface);
    font-size: 12px;
    font-weight: 700;
  }
  .state.on {
    background: #1d3b2a;
    color: #9ce5b8;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 10px;
  }
  .grow {
    flex: 1 1 220px;
    min-width: 0;
  }
  input[type='file'] {
    width: 100%;
    min-width: 0;
    font-size: 13px;
  }
  .file-import { position: relative; }
  .file-input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  .file-import:focus-within { outline: 2px solid white; outline-offset: 3px; }
  .file-import:has(input:disabled) { opacity: .5; }
  @media (max-width: 640px) {
    .head b { flex-basis: calc(100% - 40px); }
  }
  code {
    overflow-wrap: anywhere;
    padding: 1px 6px;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.35);
    font-size: 13px;
  }
  .ok {
    color: var(--ok);
  }
</style>
