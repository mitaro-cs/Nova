<script lang="ts">
  import { fadeIn } from '$lib/ui/fade';
  import { page } from '$app/state';
  import { replaceState } from '$app/navigation';
  import { api, errorText, patch } from '$lib/api';
  import { ago } from '$lib/fmt';
  import Poster from '$lib/discover/Poster.svelte';
  import PosterRow from '$lib/discover/PosterRow.svelte';
  import DirectorCard from '$lib/profile/DirectorCard.svelte';
  import FavActors from '$lib/profile/FavActors.svelte';
  import FilmStrip from '$lib/profile/FilmStrip.svelte';
  import LockBox from '$lib/profile/LockBox.svelte';
  import ProfileCard from '$lib/profile/ProfileCard.svelte';
  import StatWidgets from '$lib/profile/StatWidgets.svelte';
  import { TABS, type ProfilePage, type ProfileTab } from '$lib/profile/types';
  import VisPick from '$lib/profile/VisPick.svelte';
  import StarsShow from '$lib/social/StarsShow.svelte';
  import type { VisLevel, ExtCard } from '$lib/types';
  import { placeholder } from '$lib/ui/art';
  import Icon from '$lib/ui/Icon.svelte';
  import { toast } from '$lib/ui/toast.svelte';

  // Страница человека: слева липкая карточка (портрет, цифры, значки, кнопки), справа вкладки «Обзор», «Статистика»,
  // «Просмотрено», «Отзывы» и внизу лента просмотренного. Что человек закрыл (Статистика, Просмотрено, Отзывы),
  // сервер не отдаёт: вместо раздела замок «Скрыто владельцем». Своя страница - с переключателями «кто видит».
  let p = $state<ProfilePage | null>(null);
  let error = $state('');
  const id = $derived(Number(page.params.id));

  $effect(() => {
    const want = id;
    p = null;
    error = '';
    const tz = -new Date().getTimezoneOffset();
    api<ProfilePage>(`/api/members/${want}?tz=${tz}`).then((r) => want === id && (p = r), (e) => (error = errorText(e)));
  });

  // ---------- вкладки ----------
  const fromUrl = (): ProfileTab => {
    const q = page.url.searchParams.get('tab');
    return (TABS as string[]).includes(q ?? '') ? (q as ProfileTab) : 'home';
  };
  let tab = $state<ProfileTab>(fromUrl()); // выбранная
  let shown = $state<ProfileTab>(fromUrl()); // показанная: сменяется после того, как прежняя уехала
  let dir = $state<'r' | 'l' | 'n'>('n');
  let leaving = $state<'r' | 'l' | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function choose(t: ProfileTab) {
    if (t === tab) return;
    const d = TABS.indexOf(t) > TABS.indexOf(tab) ? 'r' : 'l';
    tab = t;
    try {
      replaceState(t === 'home' ? page.url.pathname : `?tab=${t}`, {});
    } catch {
      /* адрес не обновился - вкладка всё равно переключена */
    }
    clearTimeout(timer);
    leaving = d;
    timer = setTimeout(() => {
      dir = d;
      shown = t;
      leaving = null;
    }, 200);
  }

  const hidden = $derived(p?.hidden ?? { stats: false, watched: false, reviews: false });
  let externalWatched = $state<ExtCard[]>([]);
  let watchedLoading = $state(false);
  let watchedError = $state(false);
  $effect(() => {
    const who = id;
    if (!p || hidden.watched || shown !== 'watched') return;
    const controller = new AbortController();
    externalWatched = [];
    watchedLoading = true;
    watchedError = false;
    Promise.all(['movie', 'series'].map(kind =>
      api<{ ext: ExtCard[] }>(`/api/people/${who}/watched?kind=${kind}`, { signal: controller.signal })
    )).then(results => {
      if (!controller.signal.aborted) externalWatched = results.flatMap(result => result.ext);
    }).catch(() => {
      if (!controller.signal.aborted) watchedError = true;
    }).finally(() => {
      if (!controller.signal.aborted) watchedLoading = false;
    });
    return () => controller.abort();
  });
  const isHidden = (t: ProfileTab) => t !== 'home' && (t === 'stats' ? hidden.stats : t === 'watched' ? hidden.watched : hidden.reviews);
  const defs = $derived<{ id: ProfileTab; label: string; n: number | null }[]>([
    { id: 'home', label: 'Обзор', n: null },
    { id: 'stats', label: 'Статистика', n: null },
    { id: 'watched', label: 'Просмотрено', n: hidden.watched ? null : (p?.stats?.watched ?? p?.watched.length ?? null) },
    { id: 'reviews', label: 'Отзывы', n: hidden.reviews ? null : (p?.stats?.reviews ?? p?.reviews.length ?? null) },
  ]);

  // Чёрта под выбранной вкладкой плавно переезжает.
  let btns = $state<Record<string, HTMLButtonElement | undefined>>({});
  let ink = $state({ x: 0, w: 0 });
  function measure() {
    const b = btns[tab];
    if (b) ink = { x: b.offsetLeft, w: b.offsetWidth };
  }
  $effect(() => {
    void tab;
    void p;
    void defs;
    requestAnimationFrame(measure);
  });
  $effect(() => {
    const on = () => measure();
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  });

  // ---------- приватность (только у себя) ----------
  const KEY = { stats: 'visStats', watched: 'visWatched', reviews: 'visReviews' } as const;
  const level = $derived<VisLevel>(p?.decor.visibility?.[tab as 'stats' | 'watched' | 'reviews'] ?? 'all');
  async function setVis(v: VisLevel) {
    if (!p || tab === 'home') return;
    const key = tab;
    const before = p.decor.visibility;
    if (before) p.decor.visibility = { ...before, [key]: v };
    try {
      await patch('/api/me/profile', { [KEY[key]]: v });
    } catch (x) {
      if (p) p.decor.visibility = before;
      toast(errorText(x));
    }
  }

  const link = (r: ProfilePage['reviews'][number]) => (r.ext ? `/ext/${r.ext.id}` : `/title/${r.title?.id}`);
  const what = (r: ProfilePage['reviews'][number]) => r.ext ?? r.title!;
  const favs = (kind: string) => (p?.decor.favorites ?? []).filter((f) => (f.card ?? f.ext)?.kind === kind);
  const seenOf = (name: string) => p?.decor.directorsInfo?.find((d) => d.name === name)?.seen ?? null;
  // Итоги и жанры - из статистики; закрытая статистика (`stats: null`) сюда не попадает.
  const tiles = $derived.by(() => {
    const s = p?.stats;
    if (!s || hidden.stats) return [];
    const h = s.hours < 10 && s.hours % 1 ? s.hours.toFixed(1).replace('.', ',') : String(Math.round(s.hours));
    return [
      { v: `${h} ч`, l: 'за всё время' },
      { v: String(s.watched ?? 0), l: 'просмотров' },
      { v: String(s.episodes ?? 0), l: 'серий' },
      { v: String(s.streak?.current ?? s.streak?.best ?? 0), l: 'дней подряд' },
    ];
  });
  const topGenres = $derived.by(() => {
    const g = (p?.stats?.genres ?? []).slice(0, 5);
    const max = Math.max(1, ...g.map((x) => x.count));
    return hidden.stats ? [] : g.map((x) => ({ name: x.genre, count: x.count, pct: Math.round((x.count / max) * 100) }));
  });
  const empty = $derived(!!p && !p.watching.length && !p.decor.favorites.length && !p.decor.directors.length && !(p.decor.actors?.length) && !(p.decor.likedPeople?.length));
</script>

{#if error}
  <div class="page empty"><Icon name="info" size={36} /> {error}</div>
{:else if p}
  <div class="pf">
    <div class="hero"><ProfileCard bind:p /></div>

    {#if topGenres.length}
      <aside class="side" aria-label="Любимые жанры">
        <section class="tile">
          <h2>Любимые жанры</h2>
          <p class="gnote">Сколько просмотренного в каждом жанре</p>
          <ul class="gbars">
            {#each topGenres as g (g.name)}
              <li><div class="gl"><span>{g.name}</span><span class="faint">{g.count}</span></div><div class="gt"><i style:width="{g.pct}%"></i></div></li>
            {/each}
          </ul>
        </section>
      </aside>
    {/if}

    <main class="main" class:solo={!topGenres.length}>
      <div class="tabs" role="tablist" aria-label="Разделы профиля">
        {#each defs as t (t.id)}
          <button
            type="button"
            role="tab"
            id="tab-{t.id}"
            bind:this={btns[t.id]}
            aria-selected={tab === t.id}
            aria-controls="pane"
            onclick={() => choose(t.id)}
          >{t.label}{#if t.n != null}<sup>{t.n}</sup>{/if}{#if isHidden(t.id)}<Icon name="lock" size={13} />{/if}</button>
        {/each}
        <i class="ink" aria-hidden="true" style:transform="translate3d({ink.x}px, 0, 0)" style:width="{ink.w}px"></i>
      </div>

      {#if p.me && tab !== 'home'}<VisPick level={level} pick={setVis} />{/if}

      <div id="pane" class="pf-pane" class:out-r={leaving === 'r'} class:out-l={leaving === 'l'} role="tabpanel" aria-labelledby="tab-{shown}">
        {#key shown}
          <div class="in in-{dir}">
            {#if shown === 'home'}
              {#if tiles.length}
                <section class="stat-tiles" aria-label="Итоги">
                  {#each tiles as t (t.l)}<div class="st"><b>{t.v}</b><small>{t.l}</small></div>{/each}
                </section>
              {/if}
              {#if p.watching.length}
                <section class="sec now">
                  <PosterRow title="Сейчас смотрит" items={p.watching.map((n) => ({ card: n.card, ext: n.ext, label: n.label }))} />
                </section>
              {/if}

              {#each [{ kind: 'movie', label: 'Любимые фильмы' }, { kind: 'series', label: 'Любимые сериалы' }] as g (g.kind)}
                {@const list = favs(g.kind)}
                {#if list.length}
                  <section class="sec" aria-labelledby="f-{g.kind}">
                    <div class="sh"><h2 id="f-{g.kind}">{g.label}</h2>{#if p.me}<a class="edit" href="/profile">Изменить</a>{/if}</div>
                    <ol class="favs">
                      {#each list as f, i (f.key)}
                        <li class="rise" style:--i={i}><span class="num">{i + 1}</span><Poster card={f.ext} local={f.card ?? null} /></li>
                      {/each}
                    </ol>
                  </section>
                {/if}
              {/each}

              {#if p.decor.directors.length}
                <section class="sec" aria-labelledby="d">
                  <div class="sh"><h2 id="d">Любимый режиссёр</h2>{#if p.me}<a class="edit" href="/profile">Изменить</a>{/if}</div>
                  {#each p.decor.directors as d (d)}<DirectorCard name={d} seen={seenOf(d)} />{/each}
                </section>
              {/if}

              {#if p.me || p.decor.actors?.length}
                <div class="sec">
                  <FavActors bind:actors={p.decor.actors} suggest={(p.stats?.actors ?? []).map((a) => a.name)} editable={p.me} />
                </div>
              {/if}

              {#if p.decor.likedPeople?.length}
                <section class="sec" aria-labelledby="lp">
                  <div class="sh"><h2 id="lp">Нравятся актёры и режиссёры</h2></div>
                  <ul class="dirs">
                    {#each p.decor.likedPeople as lp, i (lp.name + lp.role)}
                      <li class="rise" style:--i={i}>
                        <a class="dir" href="/person?name={encodeURIComponent(lp.name)}&role={lp.role}">
                          <span class="dph" style:background={lp.photo ? undefined : placeholder(lp.name)}>
                            {#if lp.photo}<img use:fadeIn src={lp.photo} alt="" loading="lazy" decoding="async" width="96" height="96" />{:else}{lp.name.split(/\s+/).slice(0, 2).map((w) => w[0]).join('')}{/if}
                          </span>
                          <span class="dn">{lp.name}<small class="faint">{lp.role === 'actor' ? 'Актёр' : 'Режиссёр'}</small></span>
                        </a>
                      </li>
                    {/each}
                  </ul>
                </section>
              {/if}

              {#if empty && !p.me}<p class="muted none">Пока здесь пусто: человек ещё ничего не выбрал.</p>{/if}
              {#if empty && p.me}<p class="muted none">Здесь появятся ваши любимые фильмы, сериалы, режиссёр и актёры. Выберите их кнопкой «Оформить профиль».</p>{/if}
            {:else if shown === 'stats'}
              {#if hidden.stats || !p.stats}
                <LockBox />
              {:else}
                <StatWidgets stats={p.stats} />
                {#if !hidden.watched}
                  <p class="seen-links">
                    <a class="btn sm" href="/people/{p.person.id}/watched?kind=movie"><Icon name="film" size={14} /> Какие фильмы смотрел</a>
                    <a class="btn sm" href="/people/{p.person.id}/watched?kind=series"><Icon name="tv" size={14} /> Какие сериалы смотрел</a>
                  </p>
                {/if}
              {/if}
            {:else if shown === 'watched'}
              {#if hidden.watched}
                <LockBox />
              {:else if p.watched.length || externalWatched.length}
                <ul class="posters">
                  {#each p.watched as c, i (c.id)}
                    <li class="rise" style:--i={i}>
                      <a href="/title/{c.id}" title={c.title} style:background={c.poster ? undefined : placeholder(c.title)}>
                        {#if c.poster}<img use:fadeIn src={c.poster} alt={c.title} loading="lazy" decoding="async" width="342" height="513" />{:else}<span>{c.title}</span>{/if}
                      </a>
                    </li>
                  {/each}
                  {#each externalWatched as c (c.id)}
                    <li><a href="/ext/{c.id}" title={c.title} style:background={c.poster ? undefined : placeholder(c.title)}>
                      {#if c.poster}<img use:fadeIn src={c.poster} alt={c.title} loading="lazy" width="342" height="513" />{:else}<span>{c.title}</span>{/if}
                    </a></li>
                  {/each}
                </ul>
                <p class="seen-links">
                  <a class="btn sm" href="/people/{p.person.id}/watched?kind=movie"><Icon name="film" size={14} /> Все фильмы</a>
                  <a class="btn sm" href="/people/{p.person.id}/watched?kind=series"><Icon name="tv" size={14} /> Все сериалы</a>
                </p>
              {:else}
                <p class="muted none">{watchedLoading ? 'Загружаем просмотренное…' : watchedError ? 'Не удалось загрузить просмотренное. Обновите страницу.' : 'Просмотренного пока нет.'}</p>
              {/if}
            {:else if hidden.reviews}
              <LockBox />
            {:else if p.reviews.length}
              <ul class="reviews">
                {#each p.reviews as r, i (r.id)}
                  {@const t = what(r)}
                  <li class="rv rise" style:--i={i}>
                    <a class="mini" href={link(r)} style:background={t.poster ? undefined : placeholder(t.title)}>
                      {#if t.poster}<img use:fadeIn src={t.poster} alt="" loading="lazy" decoding="async" width="56" height="84" />{/if}
                    </a>
                    <div class="body">
                      <a href={link(r)}><b>{t.title}</b>{#if t.year} <span class="faint">{t.year}</span>{/if}</a>
                      <div class="line">
                        {#if r.stars}<StarsShow value={r.stars} />{/if}
                        <small class="faint">{ago(r.createdAt)}</small>
                      </div>
                      {#if r.body}<p>{r.body}</p>{/if}
                    </div>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="muted none">Отзывов пока нет.</p>
            {/if}
          </div>
        {/key}
      </div>

      <FilmStrip items={p.watched} personId={p.person.id} total={p.stats?.watched ?? p.watched.length} hidden={hidden.watched} />
    </main>
  </div>
{:else}
  <div class="sk-hero skeleton" aria-busy="true"></div>
{/if}

<style>
  .hero { grid-column: 1 / -1; }
  .side { display: contents; }
  @media (min-width: 1001px) {
    .side { display: block; grid-column: 1; grid-row: 2; align-self: start; position: sticky; top: calc(var(--header) + 18px); }
    .main { grid-column: 2; grid-row: 2; min-width: 0; }
    .main.solo { grid-column: 1 / -1; }
  }
  .tile { padding: 22px; border: 1px solid #ffffff1a; border-radius: 28px; background: #101012; }
  .tile h2 { margin: 0 0 18px; font-size: 17px; font-weight: 700; }
  .gnote { margin: -10px 0 16px; color: var(--muted); font-size: 12.5px; }
  .gbars { display: grid; gap: 14px; margin: 0; padding: 0; list-style: none; }
  .gl { display: flex; justify-content: space-between; margin-bottom: 7px; font-size: 14px; font-weight: 600; }
  .gt { height: 8px; border-radius: 999px; background: #ffffff14; overflow: hidden; }
  .gt i { display: block; height: 100%; border-radius: 999px; background: #fff; }
  .stat-tiles { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin-bottom: 26px; }
  .st { min-width: 0; padding: 18px 20px; border: 1px solid #ffffff1a; border-radius: 26px; background: #101012; }
  .st b { display: block; font-size: clamp(26px, 3vw, 44px); font-weight: 800; letter-spacing: -0.04em; line-height: 1; font-variant-numeric: tabular-nums; }
  .st small { display: block; margin-top: 8px; color: var(--muted); font-size: 13px; }
  @media (max-width: 600px) { .stat-tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  .pf {
    display: grid;
    grid-template-columns: 360px minmax(0, 1fr);
    gap: 28px;
    align-items: start;
    max-width: none;
    margin: 0 auto;
    padding: calc(var(--header) + 18px) var(--gutter) calc(40px + var(--tabbar));
  }
  .main {
    min-width: 0;
  }

  /* ---------- вкладки ---------- */
  .tabs {
    position: relative;
    display: flex;
    gap: 26px;
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 12px 0;
    border: 0;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 14.5px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    transition: color 0.25s;
  }
  .tabs button:hover {
    color: #dcdcdf;
  }
  .tabs button[aria-selected='true'] {
    color: var(--text);
  }
  .tabs button:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }
  .tabs sup {
    color: var(--muted);
    font-size: 11px;
    font-weight: 500;
  }
  .ink {
    position: absolute;
    left: 0;
    bottom: -1px;
    height: 3px;
    border-radius: 3px;
    background: #fff;
    transition:
      transform 0.6s cubic-bezier(0.34, 1.4, 0.64, 1),
      width 0.6s cubic-bezier(0.34, 1.4, 0.64, 1);
  }

  /* Раздел уезжает и въезжает с той стороны, куда переключили. */
  /* Въезжающий раздел не выступает за край: обрезка только по ширине, чтобы страница не росла на время анимации. */
  .pf-pane {
    padding-top: 18px;
    overflow-x: clip;
  }
  .pf-pane.out-r {
    animation: out-r 0.2s ease-in both;
  }
  .pf-pane.out-l {
    animation: out-l 0.2s ease-in both;
  }
  .now {
    --pw: clamp(116px, 11vw, 158px);
  }
  .in-r {
    animation: in-r 0.75s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  .in-l {
    animation: in-l 0.75s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  @keyframes in-r {
    from {
      opacity: 0;
      transform: translate3d(70px, 0, 0);
    }
  }
  @keyframes in-l {
    from {
      opacity: 0;
      transform: translate3d(-70px, 0, 0);
    }
  }
  @keyframes out-r {
    to {
      opacity: 0;
      transform: translate3d(-45px, 0, 0);
    }
  }
  @keyframes out-l {
    to {
      opacity: 0;
      transform: translate3d(45px, 0, 0);
    }
  }
  .rise {
    animation: rise 0.8s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--i) * 90ms);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate3d(0, 26px, 0);
    }
  }

  /* ---------- разделы ---------- */
  .sec {
    margin-top: 26px;
  }
  .sec:first-child {
    margin-top: 0;
  }
  .sh {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 12px;
  }
  .sh h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .edit {
    flex: none;
    padding: 4px 13px;
    border: 1px solid var(--line-strong, #ffffff40);
    border-radius: 999px;
    color: var(--muted);
    font-size: 12.5px;
    transition:
      background 0.2s,
      color 0.2s;
  }
  .edit:hover {
    background: #ffffff14;
    color: var(--text);
  }
  .none {
    margin: 0;
  }
  .favs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 14px;
  }
  .favs li {
    position: relative;
    min-width: 0;
  }
  /* Номер, как в «Топ-5» на IMDb: крупная цифра поверх угла постера. */
  .num {
    position: absolute;
    z-index: 3;
    left: 8px;
    bottom: 58px;
    font-size: clamp(44px, 5vw, 72px);
    letter-spacing: -0.04em;
    font-weight: 800;
    line-height: 1;
    color: #fff;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.85);
    pointer-events: none;
  }
  .dirs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
  }
  .dir {
    display: grid;
    justify-items: center;
    gap: 8px;
    width: 104px;
    text-align: center;
  }
  .dph {
    display: grid;
    place-items: center;
    width: 88px;
    height: 88px;
    border-radius: 50%;
    overflow: hidden;
    font-weight: 800;
    font-size: 22px;
    box-shadow: inset 0 0 0 1.5px #ffffff33;
    transition: transform 0.3s cubic-bezier(0.2, 0.9, 0.3, 1.4);
  }
  .dph img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .dir:hover .dph {
    transform: translateY(-4px) scale(1.05);
  }
  .dn {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }
  .posters {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 12px;
  }
  .posters a {
    display: grid;
    align-items: end;
    aspect-ratio: 2 / 3;
    border-radius: 10px;
    overflow: hidden;
    transition: transform 0.25s cubic-bezier(0.2, 0.9, 0.3, 1.3);
  }
  .posters a:hover {
    transform: translateY(-4px) scale(1.02);
  }
  .posters img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .posters span {
    padding: 8px;
    font-size: 12px;
    font-weight: 700;
  }
  .seen-links {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin: 18px 0 0;
  }
  .reviews {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 380px), 1fr));
    gap: 12px;
  }
  .rv {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr);
    gap: 14px;
    padding: 14px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--line);
  }
  .mini {
    display: block;
    width: 56px;
    aspect-ratio: 2 / 3;
    border-radius: 8px;
    overflow: hidden;
  }
  .mini img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .body {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .body p {
    margin: 2px 0 0;
    font-size: 13.5px;
    color: #d4d4d8;
    white-space: pre-line;
    overflow-wrap: anywhere;
  }
  .sk-hero {
    height: 70vh;
  }

  @media (max-width: 900px) {
    .pf {
      grid-template-columns: minmax(0, 1fr);
      gap: 16px;
    }
    .favs {
      gap: 8px;
    }
    .num {
      font-size: 26px;
      bottom: 46px;
    }
    .tabs {
      justify-content: space-between;
      gap: 12px;
    }
    .tabs button {
      font-size: 13px;
      gap: 3px;
    }
  }
  /* Самое узкое окно или сильное увеличение: вкладки в две колонки, чёрту заменяет подчёркивание самой кнопки. */
  @media (max-width: 600px) {
    .pf { padding-top:0;padding-inline:0;gap:10px; }
    .main { padding-inline:16px; }
    .tabs { gap:10px; }
    .tabs button { font-size:12px;padding:14px 0; }
    .tabs sup { font-size:10px; }
  }
  @media (max-width: 360px) {
    .tabs {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0 10px;
    }
    .tabs button {
      justify-content: center;
    }
    .tabs button[aria-selected='true'] {
      box-shadow: inset 0 -3px 0 #fff;
    }
    .ink {
      display: none;
    }
    .favs {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  /* Вкладки-капсула (профиль D): плашка переезжает под выбранной. */
  @media (min-width: 601px) {
    .tabs { gap: 4px; width: max-content; max-width: 100%; padding: 5px; border: 1px solid #ffffff1a; border-radius: 999px; background: #0c0c0e; }
    .tabs button { position: relative; z-index: 1; padding: 10px 22px; border-radius: 999px; font-size: 14px; }
    .tabs button[aria-selected='true'] { color: #0a0a0b; }
    .tabs button[aria-selected='true'] sup { color: #52525b; }
    .ink { top: 5px; bottom: 5px; height: auto; border-radius: 999px; }
  }
</style>
