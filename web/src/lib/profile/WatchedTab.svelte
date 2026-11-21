<script lang="ts">
  import type { Card, ExtCard } from '$lib/types';
  import { placeholder } from '$lib/ui/art';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';

  // Вкладка «Просмотрено» (макет в Figma 21.11): поиск, «Все / Фильмы / Сериалы», порядок, сетка карточек с оценкой и
  // датой, «Показать ещё». Фильмы с сервиса и из ленты IMDb - вместе.
  let {
    local,
    ext,
    loading = false,
    failed = false,
    me = false,
    personId,
  }: { local: Card[]; ext: ExtCard[]; loading?: boolean; failed?: boolean; me?: boolean; personId: number } = $props();

  type Row = { key: string; kind: string; title: string; year: number | null; genre: string | null; poster: string | null; rating: number | null; at: number | null; href: string };
  const rows = $derived<Row[]>([
    ...local.map((c) => ({ key: `t${c.id}`, kind: c.kind, title: c.title, year: c.year, genre: c.genres[0] ?? null, poster: c.posterLarge ?? c.poster, rating: c.rating, at: c.watchedAt, href: `/title/${c.id}` })),
    ...ext.filter((e) => !e.local || !local.some((c) => c.id === e.local)).map((e) => ({ key: `e${e.id}`, kind: e.kind, title: e.title, year: e.year, genre: e.genres?.[0] ?? null, poster: e.poster, rating: e.rating, at: null, href: `/ext/${e.id}` })),
  ]);

  let q = $state('');
  let kind = $state<'all' | 'movie' | 'series'>('all');
  let order = $state<string>('recent');
  let limit = $state(12);
  const n = $derived({ all: rows.length, movie: rows.filter((r) => r.kind === 'movie').length, series: rows.filter((r) => r.kind === 'series').length });
  const list = $derived.by(() => {
    const needle = q.trim().toLowerCase().replaceAll('ё', 'е');
    const out = rows.filter((r) => (kind === 'all' || r.kind === kind) && (!needle || r.title.toLowerCase().replaceAll('ё', 'е').includes(needle)));
    const by: Record<'recent' | 'rating' | 'title' | 'year', (a: Row, b: Row) => number> = {
      recent: (a, b) => (b.at ?? 0) - (a.at ?? 0),
      rating: (a, b) => (b.rating ?? 0) - (a.rating ?? 0),
      title: (a, b) => a.title.localeCompare(b.title, 'ru'),
      year: (a, b) => (b.year ?? 0) - (a.year ?? 0),
    };
    return out.sort(by[order as keyof typeof by]);
  });
  $effect(() => {
    void q;
    void kind;
    limit = 12;
  });
  const fmt1 = (x: number) => (x % 1 ? x.toFixed(1).replace('.', ',') : String(x));
  const short = (ms: number | null) => (ms ? new Date(ms).toLocaleDateString('ru-RU', { day: 'numeric', month: 'short' }).replace('.', '') : '');
  const plural = (x: number, a: string, b: string, c: string) => (x % 10 === 1 && x % 100 !== 11 ? a : x % 10 >= 2 && x % 10 <= 4 && (x % 100 < 10 || x % 100 >= 20) ? b : c);
</script>

<div class="wh">
  <div>
    <h2>{me ? 'Твоя коллекция просмотров' : 'Коллекция просмотров'}</h2>
    <p class="muted">{n.all} {plural(n.all, 'просмотр', 'просмотра', 'просмотров')} · {n.movie} {plural(n.movie, 'фильм', 'фильма', 'фильмов')} и {n.series} {plural(n.series, 'сериал', 'сериала', 'сериалов')}</p>
  </div>
</div>

<div class="tools">
  <label class="find"><Icon name="search" size={16} /><input type="search" placeholder="Найти фильм или сериал" bind:value={q} aria-label="Найти в просмотренном" /></label>
  <div class="seg" role="radiogroup" aria-label="Что показать">
    {#each [{ id: 'all', l: 'Все', c: n.all }, { id: 'movie', l: 'Фильмы', c: n.movie }, { id: 'series', l: 'Сериалы', c: n.series }] as k (k.id)}
      <button type="button" role="radio" aria-checked={kind === k.id} class:on={kind === k.id} onclick={() => (kind = k.id as typeof kind)}>{k.id === 'all' ? k.l : `${k.l} · ${k.c}`}</button>
    {/each}
  </div>
  <div class="sort"><Select label="Порядок" variant="sm" fit bind:value={order} options={[{ value: 'recent', label: 'Сначала недавние' }, { value: 'rating', label: 'По оценке IMDb' }, { value: 'year', label: 'Сначала новые фильмы' }, { value: 'title', label: 'По названию' }]} /></div>
</div>

{#if list.length}
  <ul class="grid" aria-label="Просмотренное">
    {#each list.slice(0, limit) as r, i (r.key)}
      <li class="rise" style:--i={i % 12}>
        <a href={r.href}>
          <span class="art" style:background={placeholder(r.title)}>
            {#if r.poster}<img use:fadeIn src={r.poster} alt="" loading="lazy" decoding="async" width="342" height="513" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
            {#if r.rating}<em><Icon name="star" size={11} /> {fmt1(r.rating)}</em>{/if}
            <i class="seen" title="Просмотрено"><Icon name="check" size={12} /></i>
          </span>
          <b>{r.title}</b>
          <span class="meta"><small>{[r.year, r.genre].filter(Boolean).join(' · ')}</small><small>{short(r.at)}</small></span>
        </a>
      </li>
    {/each}
  </ul>
  {#if list.length > limit}
    <div class="more">
      <button class="btn" type="button" onclick={() => (limit += 20)}>Показать ещё {Math.min(20, list.length - limit)}</button>
      <small class="muted">Показано {limit} из {list.length}</small>
    </div>
  {/if}
{:else}
  <p class="muted none">
    {loading ? 'Загружаем просмотренное…' : failed ? 'Не удалось загрузить просмотренное. Обновите страницу.' : q ? 'Ничего не нашлось.' : 'Просмотренного пока нет.'}
  </p>
{/if}

{#if me && rows.length}
  <section class="cta" aria-labelledby="wt-cta">
    <p class="eye">Повторим в хорошей компании?</p>
    <h3 id="wt-cta">Любимое кино - новый вечер.</h3>
    <p class="muted">Позови друзей на совместный просмотр.</p>
    <button class="btn primary" type="button" onclick={() => window.dispatchEvent(new Event('nova-room-link'))}>Создать комнату</button>
  </section>
{/if}

<p class="foot muted">Досмотренные фильмы и серии появляются здесь сами. <a href="/people/{personId}/watched?kind=movie">Все фильмы</a> · <a href="/people/{personId}/watched?kind=series">Все сериалы</a></p>

<style>
  .wh { margin-bottom: 16px; }
  .wh h2 { margin: 0; font-size: 22px; font-weight: 700; letter-spacing: -0.02em; }
  .wh p { margin: 4px 0 0; font-size: 13px; }
  .tools { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; gap: 10px; margin-bottom: 20px; }
  .find { position: relative; display: flex; align-items: center; gap: 10px; height: 44px; padding: 0 16px; border: 1px solid #ffffff1a; border-radius: 999px; background: #101012; color: var(--muted); }
  .find:focus-within { border-color: #fff; }
  .find input { flex: 1; min-width: 0; height: 100%; padding: 0; border: 0; background: none; color: #fff; font: inherit; font-size: 14px; outline: none; }
  .seg { display: flex; gap: 4px; padding: 4px; border: 1px solid #ffffff1a; border-radius: 999px; background: #101012; }
  .seg button { padding: 0 14px; border: 0; border-radius: 999px; background: none; color: #c4c4cc; font: inherit; font-size: 13px; font-weight: 600; white-space: nowrap; cursor: pointer; transition: background 0.2s, color 0.2s; }
  .seg button.on { background: #fff; color: #0a0a0b; }
  .seg button:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }

  .grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 26px 14px; margin: 0; padding: 0; list-style: none; }
  .grid a { display: grid; gap: 4px; }
  .art { position: relative; display: block; aspect-ratio: 4 / 5; margin-bottom: 8px; overflow: hidden; border-radius: 18px; transition: transform 0.3s var(--ease, ease); }
  .grid a:hover .art { transform: translate3d(0, -4px, 0); }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .art em { position: absolute; top: 10px; left: 10px; display: inline-flex; align-items: center; gap: 4px; padding: 4px 8px; border-radius: 999px; background: #000000c2; font-size: 11.5px; font-style: normal; font-weight: 700; }
  .seen { position: absolute; top: 10px; right: 10px; display: grid; place-items: center; width: 24px; height: 24px; border-radius: 50%; background: #000000c2; }
  .grid b { overflow: hidden; font-size: 14px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .meta { display: flex; justify-content: space-between; gap: 8px; color: var(--muted); font-size: 12px; }
  .meta small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .more { display: grid; justify-items: center; gap: 8px; margin-top: 30px; }
  .more .btn { min-width: 220px; justify-content: center; }
  .more small { font-size: 12px; }
  .none { padding: 30px 0; text-align: center; }

  .cta { display: grid; justify-items: start; gap: 8px; margin-top: 36px; padding: 26px; border: 1px solid #ffffff14; border-radius: 24px; background: #101012; }
  .cta .eye { margin: 0; color: var(--muted); font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
  .cta h3 { margin: 0; font-size: clamp(22px, 2.2vw, 28px); font-weight: 700; letter-spacing: -0.02em; }
  .cta p.muted { margin: 0 0 8px; font-size: 13px; }
  .foot { margin: 22px 0 0; font-size: 12px; }
  .foot a { color: #e4e4e7; text-decoration: underline; text-underline-offset: 3px; }

  @media (max-width: 1100px) { .grid { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
  @media (max-width: 760px) {
    .tools { grid-template-columns: minmax(0, 1fr); }
    .seg { overflow-x: auto; scrollbar-width: none; }
    .seg button { flex: 1; min-height: 34px; }
    .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
</style>
