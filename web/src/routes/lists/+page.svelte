<script lang="ts">
  import { artW } from '$lib/ui/art';
  import { goto } from '$app/navigation';
  import { api, errorText } from '$lib/api';
  import PosterRow from '$lib/discover/PosterRow.svelte';
  import { extSheet } from '$lib/discover/sheet.svelte';
  import NewList from '$lib/social/NewList.svelte';
  import type { Marks, SharedList } from '$lib/types';
  import Avatar from '$lib/ui/Avatar.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Sheet from '$lib/ui/Sheet.svelte';

  // Мои списки: «Смотрю», «Буду смотреть» и «Просмотрел» (с сервиса и из ленты IMDb), ниже -
  // общие списки с друзьями.
  let lists = $state<SharedList[] | null>(null);
  let marks = $state<Marks | null>(null);
  let error = $state('');
  let creating = $state(false);

  $effect(() => {
    void extSheet.version;
    // Вместе: иначе общие списки сдвигались бы, когда приходят «Буду смотреть» (CLS).
    Promise.all([
      api<SharedList[]>('/api/lists'),
      api<Marks>('/api/me/marks').catch(() => ({ want: [], watching: [], watched: [] })),
    ]).then(
      ([l, m]) => {
        marks = m;
        lists = l;
      },
      (e) => (error = errorText(e)),
    );
  });
</script>

<Sheet title="Списки" subtitle="Ваши отметки и общие списки с друзьями">
  {#snippet actions()}
    <button class="btn primary" onclick={() => (creating = true)}><Icon name="plus" size={16} /> Новый список</button>
  {/snippet}
  {#if error}
    <div class="empty"><Icon name="info" size={36} /> {error}</div>
  {:else if marks === null || lists === null}
    <div class="ph-row skeleton"></div>
  {:else}
    {#each [{ title: 'Смотрю', items: marks.watching }, { title: 'Буду смотреть', items: marks.want }, { title: 'Просмотрел', items: marks.watched }] as g (g.title)}
      {#if g.items.length}
        <div class="rw-block"><PosterRow title={g.title} items={g.items} /></div>
      {/if}
    {/each}
    {#if !marks.want.length && !marks.watched.length && !marks.watching.length}
      <p class="rw-block muted hint">
        Отмечайте фильмы «Буду смотреть» и «Просмотрел» - на странице фильма или в <a href="/discover">Обзоре</a>, они соберутся здесь.
      </p>
    {/if}

    <section class="rw-block" aria-label="Смотрим вместе">
      <h2 class="rw-h2">Смотрим вместе</h2>
      <p class="muted lead">Общие списки с друзьями: добавляйте фильмы и отмечайте, что уже посмотрели вместе.</p>

      {#if lists.length === 0}
        <div class="empty"><Icon name="users" size={40} /> Пока нет ни одного списка - создайте первый и позовите друга.</div>
      {:else}
        <div class="grid">
          {#each lists as l (l.id)}
            <a class="list" href="/lists/{l.id}">
              <span class="art" class:many={l.art.length > 1}>
                {#each l.art.slice(0, 4) as a, i (i)}<img src={artW(a, 342)} alt="" loading="lazy" decoding="async" />{/each}
                {#if !l.art.length}<span class="none"><Icon name="film" size={30} /></span>{/if}
              </span>
              <span class="name">{l.name}</span>
              <span class="sub">
                <span class="faces">
                  {#each l.members.slice(0, 4) as m (m.id)}<Avatar avatar={m.avatar} photo={m.photo} size={22} />{/each}
                </span>
                {l.count ? `${l.watched} из ${l.count} посмотрели` : 'пока пусто'}
              </span>
            </a>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</Sheet>
<NewList bind:open={creating} oncreated={(id) => goto(`/lists/${id}`)} />

<style>
  .hint {
    margin: 0;
  }
  .hint a {
    text-decoration: underline;
  }
  .ph-row {
    height: 280px;
    border-radius: 12px;
    margin-bottom: 30px;
  }
  .lead {
    margin: -8px 0 20px;
  }
  .list {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .art {
    position: relative;
    display: grid;
    aspect-ratio: 16 / 9;
    margin-bottom: 8px;
    border-radius: 12px;
    overflow: hidden;
    background: var(--bg-3);
    transition: transform 0.35s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .art.many {
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    min-height: 0;
  }
  .art.many img:first-child:nth-last-child(2),
  .art.many img:first-child:nth-last-child(2) ~ img {
    grid-row: span 2;
  }
  .art.many img:first-child:nth-last-child(3) {
    grid-row: span 2;
  }
  .none {
    display: grid;
    place-items: center;
    color: var(--faint);
  }
  .list:hover .art {
    transform: scale(1.03);
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 13px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .faces {
    display: flex;
    flex: none;
  }
  .faces :global(*:not(:first-child)) {
    margin-left: -6px;
  }
</style>
