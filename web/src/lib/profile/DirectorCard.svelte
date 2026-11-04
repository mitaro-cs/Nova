<script lang="ts">
  import { api } from '$lib/api';
  import { count } from '$lib/fmt';
  import Poster from '$lib/discover/Poster.svelte';
  import { releaseLabel } from '$lib/title/upcoming';
  import { fadeIn } from '$lib/ui/fade';
  import { placeholder } from '$lib/ui/art';
  import Icon from '$lib/ui/Icon.svelte';
  import type { ExtCard } from '$lib/types';

  // Любимый режиссёр на странице профиля: фото, имя, кто это, когда родился, коротко о нём, сколько его работ и
  // сколько из них человек смотрел (если «Просмотрено» открыто), лучшие фильмы по оценке IMDb. Данные - те же, что на
  // странице человека (`/api/people/imdb`): первый раз сервер берёт их с IMDb и Википедии, пока идёт запрос -
  // имя и заготовка.
  let { name, seen = null }: { name: string; seen?: { total: number; seen: number; avg: number | null } | null } = $props();

  type Info = { nameRu?: string | null; biography?: string | null; description?: string | null; birthDate?: string | null };
  type Person = { name: string; photo: string | null; works: ExtCard[]; info?: Info | null };
  let d = $state<Person | null>(null);
  let failed = $state(false);

  $effect(() => {
    const want = name;
    const controller = new AbortController();
    d = null;
    failed = false;
    api<Person>(`/api/people/imdb?${new URLSearchParams({ name: want, role: 'director' })}`, { signal: controller.signal }).then(
      (r) => !controller.signal.aborted && (d = r),
      () => { if (!controller.signal.aborted) failed = true; },
    );
    return () => controller.abort();
  });

  const title = $derived(d?.info?.nameRu || name);
  const best = $derived(
    (d?.works ?? [])
      .filter((w) => w.poster && w.rating)
      .sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0))
      .slice(0, 5),
  );
  const born = $derived(d?.info?.birthDate ? releaseLabel(d.info.birthDate) : '');
  const initials = $derived(name.split(/\s+/).slice(0, 2).map((w) => w[0]).join(''));
  const share = $derived(seen && seen.total ? Math.min(100, Math.round((seen.seen / seen.total) * 100)) : 0);
  const avg = $derived(seen?.avg != null ? seen.avg.toFixed(1).replace('.', ',') : '');
</script>

<article class="dir">
  <div class="top">
    <a class="photo" href="/person?name={encodeURIComponent(name)}&role=director" aria-label="{title} - страница режиссёра" style:background={d?.photo ? undefined : placeholder(name)}>
      {#if d?.photo}<img use:fadeIn src={d.photo} alt="" decoding="async" width="180" height="180" />{:else}<b>{initials}</b>{/if}
    </a>
    <div class="info">
      <span class="role">Режиссёр</span>
      <h3><a href="/person?name={encodeURIComponent(name)}&role=director">{title}</a></h3>
      {#if title !== name}<p class="orig">{name}</p>{/if}
      {#if d?.info?.description}<p class="desc">{d.info.description}</p>{/if}
      {#if born}<p class="born"><Icon name="clock" size={14} /> Родился {born}</p>{/if}
    </div>
  </div>

  {#if d?.info?.biography}
    <p class="bio selectable">{d.info.biography}</p>
  {:else if !d && !failed}
    <p class="bio muted">Собираем сведения…</p>
  {/if}

  {#if d?.works.length || avg}
    <div class="chips">
      {#if d?.works.length}<span>{count(d.works.length, 'работа', 'работы', 'работ')}</span>{/if}
      {#if avg}<span>Средняя оценка {avg}</span>{/if}
    </div>
  {/if}

  {#if seen && seen.total}
    <div class="prog" role="img" aria-label="Просмотрено {seen.seen} из {seen.total}">
      <div class="t"><span>Просмотрено {seen.seen} из {seen.total}</span><span>{share}%</span></div>
      <div class="track"><i style:width="{Math.max(share, seen.seen ? 2 : 0)}%"></i></div>
    </div>
  {/if}

  {#if best.length}
    <div class="best">
      <h4>Лучшие фильмы</h4>
      <ol>{#each best as film, i (film.id)}<li class="rise" style:--i={i}><Poster card={film} marks={false} /></li>{/each}</ol>
    </div>
  {/if}

  <div class="foot">
    <a class="more" href="/person?name={encodeURIComponent(name)}&role=director">Все фильмы и биография <Icon name="right" size={16} /></a>
  </div>
</article>

<style>
  .dir {
    padding: clamp(14px, 1.8vw, 20px);
    border: 1px solid var(--line);
    border-radius: 18px;
    background: var(--surface);
  }
  .top {
    display: flex;
    gap: 16px;
    align-items: center;
  }
  .photo {
    display: grid;
    place-items: center;
    flex: none;
    width: clamp(64px, 8vw, 84px);
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px #ffffff33;
  }
  .photo img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 20%;
  }
  .photo b {
    font-size: 24px;
  }
  .info {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .role {
    justify-self: start;
    padding: 2px 10px;
    border-radius: 999px;
    background: #ffffff1f;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  h3 {
    margin: 2px 0 0;
    font-size: clamp(18px, 2vw, 22px);
    letter-spacing: -0.02em;
    line-height: 1.15;
    overflow-wrap: anywhere;
  }
  .orig,
  .desc,
  .born {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
  .desc {
    color: var(--text);
    font-weight: 600;
  }
  .born {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .bio {
    margin: 12px 0 0;
    max-width: 74ch;
    color: #dcdcdf;
    font-size: 14px;
    line-height: 1.55;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .bio.muted {
    color: var(--muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .chips span {
    padding: 4px 12px;
    border-radius: 999px;
    background: #ffffff14;
    font-size: 12.5px;
  }
  .prog {
    margin-top: 14px;
  }
  .t {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 6px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .track {
    height: 5px;
    overflow: hidden;
    border-radius: 3px;
    background: #ffffff1c;
  }
  .track i {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: #fff;
    animation: fill 1.3s cubic-bezier(0.2, 0.8, 0.2, 1) 0.4s both;
  }
  @keyframes fill {
    from {
      width: 0;
    }
  }
  .best {
    margin-top: 16px;
  }
  .best h4 {
    margin: 0 0 12px;
    font-size: 14px;
  }
  .best ol {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: clamp(8px, 1.4vw, 16px);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .best li {
    min-width: 0;
  }
  .foot {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
  .more {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
    transition: color 0.2s;
  }
  .more:hover {
    color: var(--text);
  }
  @media (max-width: 700px) {
    .best ol {
      display: flex;
      overflow-x: auto;
      margin-inline: -4px;
      padding: 4px;
      scrollbar-width: none;
    }
    .best ol::-webkit-scrollbar {
      display: none;
    }
    .best li {
      flex: none;
      width: 112px;
    }
  }
</style>
