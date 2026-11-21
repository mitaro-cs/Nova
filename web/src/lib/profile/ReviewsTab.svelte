<script lang="ts">
  import StarsShow from '$lib/social/StarsShow.svelte';
  import { placeholder } from '$lib/ui/art';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';
  import type { ProfilePage } from './types';

  // Вкладка «Отзывы» (макет в Figma 21.11): отбор, порядок, карточки с постером, звёздами, датой и текстом.
  let { reviews, me = false, toWatched }: { reviews: ProfilePage['reviews']; me?: boolean; toWatched?: () => void } = $props();

  type R = ProfilePage['reviews'][number];
  let filter = $state<'all' | 'text' | 'stars'>('all');
  let order = $state<string>('new');
  let limit = $state(5);
  const n = $derived({ all: reviews.length, text: reviews.filter((r) => r.body.trim()).length, stars: reviews.filter((r) => !r.body.trim() && r.stars).length });
  const list = $derived.by(() => {
    const out = reviews.filter((r) => filter === 'all' || (filter === 'text' ? r.body.trim() : !r.body.trim() && r.stars));
    const by = { new: (a: R, b: R) => b.createdAt - a.createdAt, high: (a: R, b: R) => (b.stars ?? 0) - (a.stars ?? 0), low: (a: R, b: R) => (a.stars ?? 99) - (b.stars ?? 99) };
    return out.sort(by[order as keyof typeof by]);
  });
  $effect(() => {
    void filter;
    limit = 5;
  });
  const what = (r: R) => r.ext ?? r.title!;
  const link = (r: R) => (r.ext ? `/ext/${r.ext.id}` : `/title/${r.title?.id}`);
  const day = (ms: number) => new Date(ms).toLocaleDateString('ru-RU', { day: 'numeric', month: 'long', year: 'numeric' }).replace(' г.', '');
  const plural = (x: number) => (x % 10 === 1 && x % 100 !== 11 ? 'отзыв' : x % 10 >= 2 && x % 10 <= 4 && (x % 100 < 10 || x % 100 >= 20) ? 'отзыва' : 'отзывов');
</script>

<div class="rh">
  <div>
    <h2>{me ? 'Мои отзывы' : 'Отзывы'}</h2>
    <p class="muted">{n.all} {plural(n.all)} о том, что осталось после титров</p>
  </div>
  <div class="sort"><Select label="Порядок" variant="sm" fit bind:value={order} options={[{ value: 'new', label: 'Сначала новые' }, { value: 'high', label: 'Высокие оценки' }, { value: 'low', label: 'Низкие оценки' }]} /></div>
</div>

<div class="seg" role="radiogroup" aria-label="Какие отзывы">
  {#each [{ id: 'all', l: 'Все отзывы', c: n.all }, { id: 'text', l: 'С текстом', c: n.text }, { id: 'stars', l: 'Только оценки', c: n.stars }] as k (k.id)}
    <button type="button" role="radio" aria-checked={filter === k.id} class:on={filter === k.id} onclick={() => (filter = k.id as typeof filter)}>{k.l} · {k.c}</button>
  {/each}
</div>

{#if list.length}
  <ul class="list">
    {#each list.slice(0, limit) as r, i (r.id)}
      {@const t = what(r)}
      <li class="rv rise" style:--i={i}>
        <div class="top">
          <a class="mini" href={link(r)} style:background={placeholder(t.title)} aria-hidden="true" tabindex="-1">
            {#if t.poster}<img use:fadeIn src={t.poster} alt="" loading="lazy" decoding="async" width="64" height="90" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
          </a>
          <div class="hd">
            <a href={link(r)}><b>{t.title}</b></a>
            {#if t.year}<small class="muted">{t.year}</small>{/if}
            {#if r.stars}<span class="st"><StarsShow value={r.stars} /> <b>{r.stars}/10</b></span>{/if}
          </div>
          <small class="when muted">{day(r.createdAt)}</small>
        </div>
        {#if r.body.trim()}<p class="txt selectable">{r.body}</p>{/if}
        {#if me}<div class="ft"><a href={link(r)}>Редактировать</a></div>{/if}
      </li>
    {/each}
  </ul>
  {#if list.length > limit}
    <div class="more">
      <button class="btn" type="button" onclick={() => (limit += 10)}>Показать ещё {Math.min(10, list.length - limit)}</button>
      <small class="muted">Показано {limit} из {list.length}</small>
    </div>
  {/if}
{:else}
  <p class="muted none">{reviews.length ? 'Таких отзывов нет.' : 'Отзывов пока нет.'}</p>
{/if}

{#if me}
  <section class="cta">
    <div><h3>У каждого фильма - своя история. Расскажи свою.</h3><p class="muted">Оставить отзыв можно на странице фильма или из просмотренного.</p></div>
    {#if toWatched}<button type="button" class="go" onclick={toWatched}>К просмотренному <Icon name="right" size={13} /></button>{/if}
  </section>
{/if}

<style>
  .rh { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 16px; }
  .rh h2 { margin: 0; font-size: 22px; font-weight: 700; letter-spacing: -0.02em; }
  .rh p { margin: 4px 0 0; font-size: 13px; }
  .seg { display: inline-flex; flex-wrap: wrap; gap: 4px; margin-bottom: 20px; padding: 4px; border: 1px solid #ffffff1a; border-radius: 999px; background: #101012; }
  .seg button { min-height: 34px; padding: 0 14px; border: 0; border-radius: 999px; background: none; color: #c4c4cc; font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; transition: background 0.2s, color 0.2s; }
  .seg button.on { background: #fff; color: #0a0a0b; }
  .seg button:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }

  .list { display: grid; gap: 14px; margin: 0; padding: 0; list-style: none; }
  .rv { padding: 20px 22px; border: 1px solid #ffffff14; border-radius: 22px; background: #101012; }
  .top { display: grid; grid-template-columns: 64px minmax(0, 1fr) auto; gap: 16px; align-items: start; }
  .mini { width: 64px; height: 90px; overflow: hidden; border-radius: 10px; }
  .mini img { width: 100%; height: 100%; object-fit: cover; }
  .hd { display: grid; gap: 4px; min-width: 0; padding-top: 4px; }
  .hd > a b { font-size: 17px; font-weight: 700; }
  .hd > a:hover b { text-decoration: underline; }
  .hd small { font-size: 12px; }
  .st { display: inline-flex; align-items: center; gap: 10px; margin-top: 6px; font-size: 13px; }
  .when { padding-top: 6px; font-size: 12px; white-space: nowrap; }
  .txt { margin: 16px 0 0; color: #d4d4d8; font-size: 14px; line-height: 1.55; overflow-wrap: anywhere; white-space: pre-line; }
  .ft { display: flex; justify-content: flex-end; margin-top: 12px; }
  .ft a { color: var(--muted); font-size: 12.5px; }
  .ft a:hover { color: #fff; }
  .more { display: grid; justify-items: center; gap: 8px; margin-top: 26px; }
  .more .btn { min-width: 220px; justify-content: center; }
  .more small { font-size: 12px; }
  .none { padding: 30px 0; text-align: center; }
  .cta { display: flex; align-items: center; justify-content: space-between; gap: 20px; margin-top: 30px; padding: 22px 26px; border: 1px solid #ffffff14; border-radius: 22px; background: #101012; }
  .cta h3 { margin: 0; font-size: 17px; font-weight: 700; }
  .cta p { margin: 6px 0 0; font-size: 12.5px; }
  .go { display: inline-flex; flex: none; align-items: center; gap: 4px; padding: 0; border: 0; background: none; color: #fff; font: inherit; font-size: 13px; font-weight: 700; cursor: pointer; }
  @media (max-width: 640px) {
    .rh { flex-direction: column; }
    .top { grid-template-columns: 56px minmax(0, 1fr); }
    .mini { width: 56px; height: 80px; }
    .when { grid-column: 2; padding-top: 0; }
    .cta { flex-direction: column; align-items: flex-start; }
  }
</style>
