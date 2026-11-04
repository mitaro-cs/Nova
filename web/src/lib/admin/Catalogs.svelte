<script lang="ts">
  import { api, errorText, post, put } from '$lib/api';
  import { session } from '$lib/session.svelte';
  import Icon from '$lib/ui/Icon.svelte';

  // Каталоги: сайты, где много фильмов, серий-франшиз и сериалов. Хост добавляет сайт - Nova сама находит
  // у него поиск (или хост вписывает адрес поиска с {q}) и ищет там фильм, который открыт у зрителя.
  type Catalog = { domain: string; search: string | null };
  let items = $state<Catalog[] | null>(null);
  let domain = $state('');
  let busy = $state(false);
  let msg = $state('');
  let error = $state('');
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
        [...items, { domain: found.domain, search: found.search }],
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
</script>

<section class="card pane" aria-labelledby="catalogs">
  <h3 id="catalogs">Каталоги фильмов и сериалов</h3>
  <p class="muted">
    Сайты, где много фильмов, серий-франшиз и сериалов. Когда зритель открывает фильм, Nova ищет его на этих сайтах
    и предлагает найденные страницы: видео открывается в плеере Nova. Сериал находится страницей целиком, серия за
    серией - только у LordFilm. Сайты выбирает только хост.
  </p>
  {#if items === null}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  {:else}
    {#if items.length}
      <ul class="list">
        {#each items as c (c.domain)}
          <li>
            <div class="head">
              <Icon name="globe" size={15} /><b>{c.domain}</b>
              <span class="state" class:on={!!c.search}>{c.search ? 'поиск настроен' : 'нет адреса поиска'}</span>
              {#if host}<button class="btn sm ghost" aria-label="Убрать {c.domain}" onclick={() => drop(c.domain)}><Icon name="close" size={14} /></button>{/if}
            </div>
            {#if host}
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
