<script lang="ts">
  import { api } from '$lib/api';
  import Poster from '$lib/discover/Poster.svelte';
  import { releaseLabel } from '$lib/title/upcoming';
  import { fadeIn } from '$lib/ui/fade';
  import { placeholder } from '$lib/ui/art';
  import Icon from '$lib/ui/Icon.svelte';
  import type { ExtCard } from '$lib/types';

  // Любимый режиссёр на странице профиля: большое фото, имя, кто это, когда родился, коротко о нём и его лучшие
  // фильмы по оценке IMDb. Данные - те же, что на странице человека (`/api/people/imdb`): первый раз сервер
  // берёт их с IMDb и Википедии, пока идёт запрос - имя и заготовка.
  let { name }: { name: string } = $props();

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
</script>

<article class="dir">
  <a class="who" href="/person?name={encodeURIComponent(name)}&role=director" aria-label="{title} - страница режиссёра">
    <span class="photo" style:background={d?.photo ? undefined : placeholder(name)}>
      {#if d?.photo}<img use:fadeIn src={d.photo} alt="" decoding="async" width="180" height="180" />{:else}<b>{initials}</b>{/if}
    </span>
  </a>
  <div class="info">
    <span class="role">Режиссёр</span>
    <h3><a href="/person?name={encodeURIComponent(name)}&role=director">{title}</a></h3>
    {#if title !== name}<p class="orig">{name}</p>{/if}
    {#if d?.info?.description}<p class="desc">{d.info.description}</p>{/if}
    {#if born}<p class="born"><Icon name="clock" size={14} /> Родился {born}</p>{/if}
    {#if d?.info?.biography}
      <p class="bio selectable">{d.info.biography}</p>
    {:else if !d && !failed}
      <p class="bio muted">Собираем сведения…</p>
    {/if}
    <a class="more" href="/person?name={encodeURIComponent(name)}&role=director">Все фильмы и биография <Icon name="right" size={16} /></a>
  </div>
  {#if best.length}
    <div class="best">
      <h4>Лучшие фильмы</h4>
      <ol>{#each best as film, i (film.id)}<li class="rise" style:--i={i}><Poster card={film} marks={false} /></li>{/each}</ol>
    </div>
  {/if}
</article>

<style>
  .dir {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 6px clamp(18px, 2.4vw, 34px);
    align-items: start;
    padding: clamp(18px, 2vw, 28px);
    border-radius: 28px;
    background: #ffffff0d;
  }
  .photo {
    display: grid;
    place-items: center;
    width: clamp(104px, 11vw, 168px);
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: 50%;
    box-shadow: 0 18px 44px -18px #000c;
  }
  .photo img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 20%;
  }
  .photo b {
    font-size: 40px;
  }
  .info {
    display: grid;
    gap: 6px;
    min-width: 0;
    align-content: start;
  }
  .role {
    justify-self: start;
    padding: 3px 12px;
    border-radius: 999px;
    background: #ffffff1f;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  h3 {
    margin: 4px 0 0;
    font-size: clamp(24px, 2.6vw, 38px);
    letter-spacing: -0.03em;
    line-height: 1.1;
  }
  .orig,
  .desc,
  .born {
    margin: 0;
    color: var(--muted);
    font-size: 14px;
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
    margin: 6px 0 0;
    max-width: 78ch;
    font-size: 15px;
    line-height: 1.6;
    color: #dcdcdf;
    display: -webkit-box;
    -webkit-line-clamp: 5;
    line-clamp: 5;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .more {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 8px;
    font-weight: 600;
    font-size: 14px;
    color: #fff;
  }
  .more:hover {
    text-decoration: underline;
  }
  .best {
    grid-column: 1 / -1;
    margin-top: 14px;
  }
  .best h4 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .best ol {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: clamp(8px, 1.4vw, 18px);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .best li {
    min-width: 0;
  }
  @media (max-width: 700px) {
    .dir {
      grid-template-columns: minmax(0, 1fr);
      justify-items: start;
    }
    .photo {
      width: 112px;
    }
    .best ol {
      display: flex;
      overflow-x: auto;
      margin-inline: -4px;
      padding: 4px;
    }
    .best li {
      flex: none;
      width: 124px;
    }
  }
</style>
