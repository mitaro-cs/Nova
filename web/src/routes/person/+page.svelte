<script lang="ts">
  import { hideBroken } from '$lib/ui/hideBroken';
  import { fadeIn } from '$lib/ui/fade';
  import { page } from '$app/state';
  import { api, errorText, patch, put } from '$lib/api';
  import { count } from '$lib/fmt';
  import Poster from '$lib/discover/Poster.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import { toneOfUrl } from '$lib/ui/tone';
  import type { Card, ExtCard } from '$lib/types';
  import { artW, placeholder } from '$lib/ui/art';
  import Icon from '$lib/ui/Icon.svelte';
  import { getDetail, peekDetail } from '$lib/discover/details';

  // Человек кино - «Ретроспектива» (выбор владельца, 23.11): фоном большой кадр его лучшего фильма, цветной
  // портрет крупно (ч/б - только как траур, не для живых) (фото 1200 px), имя, факты (работы, годы, средняя оценка IMDb, жанр), «Вы видели N из M»;
  // «Лучшее» - топ-5 с огромными контурными цифрами, «Хронология» - работы на линии лет (смотренное - в цвете с
  // галочкой), ниже биография, что есть на Nova и вся фильмография. Первый раз сервер берёт данные с IMDb - секунды.
  type Director = {
    name: string;
    imdbName: string | null;
    id: string | null;
    role: 'director' | 'actor';
    photo: string | null;
    works: ExtCard[];
    local: Card[];
    availability?: 'full' | 'partial' | 'unavailable';
    info?: { nameRu?: string | null; biography?: string | null; description?: string | null; birthDate?: string | null; source?: string; wiki?: string | null } | null;
  };

  const name = $derived(page.url.searchParams.get('name') ?? '');
  /** Русское имя из ссылки, когда Викиданные его не отдали. */
  const nameRuLink = $derived(page.url.searchParams.get('ru'));
  const pid = $derived(page.url.searchParams.get('id') ?? '');
  const role = $derived(page.url.searchParams.get('role') === 'actor' ? 'actor' : 'director');
  let d = $state<Director | null>(null);
  let error = $state('');
  let kind = $state<'all' | 'movie' | 'series'>('all');
  let retry = $state(0);
  let tone = $state('#1a1b20');
  let more = $state(false);

  $effect(() => {
    const want = name;
    void retry;
    const controller = new AbortController();
    const q = new URLSearchParams({ name: want, role });
    if (pid) q.set('id', pid);
    d = null;
    error = '';
    kind = 'all';
    more = false;
    if (!want) return void (error = 'Не указано, кто это');
    api<Director>(`/api/people/imdb?${q}`, { signal: controller.signal }).then(
      (r) => !controller.signal.aborted && (d = r),
      (e) => { if (!controller.signal.aborted) error = errorText(e); },
    );
    return () => controller.abort();
  });

  // «Нравится» и любимый режиссёр (один на профиль).
  let liked = $state<{ name: string; role: string }[]>([]);
  let favDirectors = $state<string[]>([]);
  $effect(() => {
    api<{ liked: { name: string; role: string }[]; directors: string[] }>('/api/me/people').then(
      (r) => { liked = r.liked; favDirectors = r.directors; },
      () => {},
    );
  });
  const isLiked = $derived(liked.some((p) => p.name === name && p.role === role));
  const isFavDirector = $derived(favDirectors.some((n) => n.toLowerCase() === name.toLowerCase()));
  async function like() {
    const value = !isLiked;
    try {
      const r = await put<{ liked: { name: string; role: string }[] }>('/api/me/people', { name, role, value });
      liked = r.liked;
    } catch (e) {
      toast(errorText(e));
    }
  }
  async function favDirector() {
    try {
      const next = isFavDirector ? [] : [name];
      await patch('/api/me/profile', { directors: next });
      favDirectors = next;
      toast(next.length ? 'Теперь это ваш любимый режиссёр' : 'Любимый режиссёр убран');
    } catch (e) {
      toast(errorText(e));
    }
  }

  const works = $derived(d ? d.works.filter((w) => kind === 'all' || w.kind === kind) : []);
  const years = $derived.by(() => {
    const ys = (d?.works ?? []).map((w) => w.year).filter((y): y is number => !!y);
    if (!ys.length) return '';
    const lo = Math.min(...ys), hi = Math.max(...ys);
    return lo === hi ? String(lo) : `${lo} - ${hi}`;
  });
  // Лучшие фильмы: по оценке IMDb, у кого уже набралось оценок, сначала фильмы.
  const best = $derived(
    (d?.works ?? [])
      .filter((w) => w.rating && w.poster)
      .sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0))
      .slice(0, 10),
  );
  const mosaic = $derived((d?.works ?? []).filter((w) => w.poster).slice(0, 18));
  const top5 = $derived(best.slice(0, 5));
  // Баннер: кадр (16:9, большой) лучшего фильма; пока не пришёл - мозаика постеров.
  let banner = $state<string | null>(null);
  $effect(() => {
    const id = best[0]?.id;
    banner = id ? (peekDetail(id)?.backdrop ?? null) : null;
    if (!id || banner) return;
    let live = true;
    getDetail(id).then((x) => live && (banner = x.backdrop ?? null), () => {});
    return () => (live = false);
  });
  // Что из его работ человек уже смотрел: отметка «Просмотрено» у фильма ленты или у своего фильма на Nova.
  const localStatus = $derived(new Map((d?.local ?? []).map((c) => [c.id, c.status])));
  const seen = (w: ExtCard) => w.mark === 'watched' || (w.local != null && localStatus.get(w.local) === 'watched');
  const seenCount = $derived((d?.works ?? []).filter(seen).length);
  const avgImdb = $derived.by(() => {
    const r = (d?.works ?? []).map((w) => w.rating).filter((x): x is number => !!x);
    return r.length ? (r.reduce((a, b) => a + b, 0) / r.length).toFixed(1).replace('.', ',') : null;
  });
  const topGenre = $derived.by(() => {
    const n = new Map<string, number>();
    for (const w of d?.works ?? []) for (const g of w.genres ?? []) n.set(g, (n.get(g) ?? 0) + 1);
    return [...n.entries()].sort((a, b) => b[1] - a[1])[0]?.[0] ?? null;
  });
  // Хронология: от ранних к поздним, только с постером и годом.
  const timeline = $derived([...(d?.works ?? [])].filter((w) => w.poster && w.year).sort((a, b) => (a.year ?? 0) - (b.year ?? 0)));
  $effect(() => {
    const src = best[0]?.poster;
    if (!src) return;
    let live = true;
    void toneOfUrl(artW(src, 342) ?? src).then((c) => { if (live && c) tone = c; });
    return () => { live = false; };
  });
  const initials = (s: string) =>
    s
      .split(/\s+/)
      .slice(0, 2)
      .map((w) => w[0])
      .join('');
  const bio = $derived(d?.info?.biography ?? d?.info?.description ?? '');
</script>

<div class="page" style:--tone={tone}>
  {#if error}
    <div class="empty"><Icon name="info" size={36} /> {error}</div>
  {:else}
    <header class="hero" class:has-banner={!!banner}>
      {#if banner}
        <img class="banner" use:fadeIn src={artW(banner, 1920)} alt="" decoding="async" aria-hidden="true" />
      {:else}
        <div class="wall" aria-hidden="true">
          {#each mosaic as w (w.id)}<img use:hideBroken src={artW(w.poster, 342)} alt="" loading="lazy" decoding="async" />{/each}
        </div>
      {/if}
      <div class="veil" aria-hidden="true"></div>
      <div class="head">
        <span class="ph" style:background={d?.photo ? undefined : placeholder(name)}>
          {#if d?.photo}<img use:fadeIn src={artW(d.photo, 1200)} alt={d?.info?.nameRu ?? nameRuLink ?? name} width="600" height="750" decoding="async" />{:else if d}{initials(d?.info?.nameRu ?? nameRuLink ?? name)}{/if}
        </span>
        <div class="who">
          <small>{role === 'actor' ? 'Актёр' : 'Режиссёр'} · ретроспектива</small>
          <h1>{d?.info?.nameRu ?? nameRuLink ?? name}</h1>
          {#if d?.imdbName && d.imdbName !== name}<p class="orig">{d.imdbName}</p>{/if}
          {#if d}
            <dl class="facts">
              {#if d.works.length}<div><dt>{count(d.works.length, 'работа', 'работы', 'работ').split(' ')[1]}</dt><dd>{d.works.length}</dd></div>{/if}
              {#if years}<div><dt>годы</dt><dd>{years.replace(' - ', '-')}</dd></div>{/if}
              {#if avgImdb}<div><dt>в среднем на IMDb</dt><dd>{avgImdb}</dd></div>{/if}
              {#if topGenre}<div><dt>чаще всего</dt><dd>{topGenre}</dd></div>{/if}
            </dl>
            <div class="mine">
              <span>Вы видели <b>{seenCount} из {d.works.length}</b></span>
              <span class="mbar" aria-hidden="true"><i style:transform="scaleX({d.works.length ? seenCount / d.works.length : 0})"></i></span>
              {#if d.local.length}<span class="here"><Icon name="play" size={13} /> {d.local.length} на Nova</span>{/if}
              {#if d.id}<a class="imdb-l" href="https://www.imdb.com/name/{d.id}/" target="_blank" rel="noopener noreferrer"><b class="imdb">IMDb</b></a>{/if}
            </div>
            <div class="acts">
              <button class="act" class:on={isLiked} aria-pressed={isLiked} onclick={like}>
                <Icon name="heart" size={18} /> {isLiked ? 'Вам нравится' : 'Нравится'}
              </button>
              {#if role === 'director'}
                <button class="act" class:on={isFavDirector} aria-pressed={isFavDirector} onclick={favDirector}>
                  <Icon name="star" size={18} /> {isFavDirector ? 'Любимый режиссёр' : 'Сделать любимым режиссёром'}
                </button>
              {/if}
            </div>
          {:else}
            <p class="orig">Собираем фильмографию с IMDb…</p>
          {/if}
        </div>
      </div>
    </header>

    {#if d === null}
      <div class="grid">{#each Array(12) as _, i (i)}<span class="sk skeleton"></span>{/each}</div>
    {:else}
      {#if top5.length}
        <section aria-labelledby="h-best">
          <h2 id="h-best" class="lab">Лучшее по IMDb</h2>
          <ol class="best">
            {#each top5 as w, i (w.id)}
              <li style:--i={i}>
                <a class="pw" href="/ext/{w.id}" title="{w.title}{w.year ? `, ${w.year}` : ''}">
                  <span class="rank" aria-hidden="true">{i + 1}</span>
                  <span class="bp" style:background={placeholder(w.title)}>
                    {#if w.poster}<img use:hideBroken src={artW(w.poster, 342)} alt={w.title} loading="lazy" decoding="async" width="228" height="342" />{/if}
                    {#if seen(w)}<i class="ck" aria-label="смотрели"><Icon name="check" size={12} /></i>{/if}
                  </span>
                  <span class="cap"><b>{w.title}</b><small>{[w.year, w.rating ? `★ ${w.rating.toFixed(1)}` : null].filter(Boolean).join(' · ')}</small></span>
                </a>
              </li>
            {/each}
          </ol>
        </section>
      {/if}

      {#if timeline.length > 1}
        <section aria-labelledby="h-tl">
          <div class="bar"><h2 id="h-tl" class="lab">Хронология</h2><span class="legend"><span><i></i>смотрели</span><span><i class="u"></i>ещё нет</span></span></div>
          <div class="tlwrap">
            <ol class="tl" style:--n={timeline.length}>
              {#each timeline as w, i (w.id)}
                <li class:seen={seen(w)} class:up={i % 2 === 1}>
                  <a href="/ext/{w.id}" title="{w.title}, {w.year}{seen(w) ? ' - смотрели' : ''}">
                    <img use:hideBroken src={artW(w.poster, 342)} alt={w.title} loading="lazy" decoding="async" width="114" height="171" />
                    {#if seen(w)}<i class="ck" aria-label="смотрели"><Icon name="check" size={12} /></i>{/if}
                  </a>
                  <span class="yr">{w.year}</span>
                </li>
              {/each}
            </ol>
          </div>
        </section>
      {/if}

      <section class="person-summary" aria-label="Кратко о человеке">
        <h2>Биография</h2>
        {#if d.info?.birthDate}<p class="muted">Дата рождения: {new Date(d.info.birthDate).toLocaleDateString('ru-RU', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' })}</p>{/if}
        {#if bio}
          <p class="text" class:clamp={!more && bio.length > 420}>{bio}</p>
          {#if bio.length > 420}<button class="link" onclick={() => (more = !more)}>{more ? 'Свернуть' : 'Читать дальше'}</button>{/if}
        {:else}<p class="muted">Не удалось загрузить биографию на русском. Попробуйте обновить сведения или откройте страницу источника.</p><button class="btn sm" onclick={() => retry++}>Обновить сведения</button>{/if}
        {#if d.info?.source}<p class="sources">Источники: <a href={d.info.source} target="_blank" rel="noopener noreferrer">Викиданные</a>{#if d.info.wiki} · <a href={d.info.wiki} target="_blank" rel="noopener noreferrer">Википедия</a>{/if}</p>{/if}
      </section>
      {#if d.local.length}
        <section aria-labelledby="h-local">
          <h2 id="h-local">Есть на Nova</h2>
          <div class="grid">{#each d.local as c, i (c.id)}<Poster local={c} {i} />{/each}</div>
        </section>
      {/if}
      <section aria-labelledby="h-works">
        {#if d.availability === 'partial'}<p class="muted">Показаны работы из Викиданных и сохранённые сведения. Полную фильмографию IMDb сейчас загрузить не удалось.</p>{/if}
        <div class="bar">
          <h2 id="h-works">Фильмография</h2>
          <div class="chips" role="tablist" aria-label="Что показывать">
            {#each [{ id: 'all', l: 'Всё' }, { id: 'movie', l: 'Фильмы' }, { id: 'series', l: 'Сериалы' }] as k (k.id)}
              <button class="chip" class:on={kind === k.id} role="tab" aria-selected={kind === k.id} onclick={() => (kind = k.id as typeof kind)}>{k.l}</button>
            {/each}
          </div>
        </div>
        {#if works.length}
          <div class="grid">{#each works as w, i (w.id)}<Poster card={w} {i} />{/each}</div>
        {:else}
          <p class="muted">{d.availability === 'unavailable' ? 'Сведения о работах сейчас недоступны. Это ошибка загрузки, а не отсутствие фильмографии.' : d.id ? 'Для этого фильтра работы не найдены.' : 'Не удалось определить человека. Попробуйте поиск по оригинальному имени.'}</p><button class="btn sm" onclick={() => retry++}>Повторить загрузку</button>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .hero {
    position: relative;
    isolation: isolate;
    margin: 0 calc(-1 * var(--gutter)) 34px;
    padding: calc(var(--header) + 28px) var(--gutter) 34px;
    overflow: hidden;
    background: color-mix(in srgb, var(--tone) 50%, #000);
    transition: background-color 0.8s ease;
  }
  /* Мозаика постеров лучших работ плывёт на фоне: справа ярче, слева растворяется в цвет. */
  .wall {
    position: absolute;
    inset: 0 0 0 28%;
    z-index: -2;
    display: grid;
    grid-template-columns: repeat(9, minmax(0, 1fr));
    grid-auto-rows: 1fr;
    gap: 8px;
    padding: 8px;
    opacity: 0.55;
    transform: rotate(-4deg) scale(1.25);
  }
  .wall img {
    width: 100%;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 8px;
  }
  .veil {
    position: absolute;
    inset: 0;
    z-index: -1;
    background:
      linear-gradient(90deg, color-mix(in srgb, var(--tone) 50%, #000) 18%, transparent 75%),
      linear-gradient(0deg, var(--bg) 0%, transparent 55%);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 30px;
    max-width: 1440px;
    margin: 0 auto;
  }
  .ph {
    flex: none;
    display: grid;
    place-items: center;
    width: clamp(120px, 14vw, 210px);
    aspect-ratio: 1;
    border-radius: 28%;
    overflow: hidden;
    font-size: 44px;
    font-weight: 800;
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.18),
      0 30px 60px -20px rgba(0, 0, 0, 0.9);
    animation: pop 0.6s cubic-bezier(0.2, 0.9, 0.3, 1.3) both;
  }
  .ph img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.85);
    }
  }
  .who {
    display: grid;
    gap: 8px;
    min-width: 0;
  }
  .who small {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #ffffffb3;
  }
  h1 {
    font-size: clamp(30px, 4.4vw, 60px);
    line-height: 1.05;
    font-weight: 800;
    letter-spacing: -0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding-bottom: 0.08em;
  }
  .orig {
    margin: 0;
    color: #ffffffa6;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 4px 0 0;
  }
  .imdb-l {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.1);
    font-size: 13px;
    font-weight: 600;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 10px;
  }
  .act {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 42px;
    padding: 0 18px;
    border: 0;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.14);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.18);
    color: #fff;
    font: inherit;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition:
      background 0.2s,
      transform 0.2s cubic-bezier(0.2, 0.9, 0.3, 1.4);
  }
  .act:hover {
    background: rgba(255, 255, 255, 0.24);
    transform: translateY(-2px);
  }
  .act:active {
    transform: scale(0.96);
  }
  .act.on {
    background: #fff;
    color: #0a0a0a;
  }
  .act.on :global(svg) {
    fill: currentColor;
  }

  section {
    margin-bottom: 38px;
  }
  .best {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 26px 16px;
  }
  .best li {
    position: relative;
    animation: rise-in 0.5s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--i) * 40ms);
  }
  .rank {
    position: absolute;
    z-index: 2;
    left: -6px;
    top: -10px;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: #fff;
    color: #0a0a0a;
    font-weight: 800;
    font-size: 15px;
    box-shadow: 0 6px 16px -6px #000;
    pointer-events: none;
  }
  .person-summary {
    padding: 28px 32px;
    border-radius: 26px;
    background: #1b1b1b;
  }
  .person-summary h2 {
    margin-bottom: 12px;
  }
  .person-summary p {
    max-width: 95ch;
    margin: 0 0 10px;
    line-height: 1.65;
  }
  .person-summary .text.clamp {
    display: -webkit-box;
    -webkit-line-clamp: 5;
    line-clamp: 5;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .person-summary .link {
    padding: 0;
    border: 0;
    background: none;
    color: #fff;
    font: inherit;
    font-weight: 600;
    text-decoration: underline;
    cursor: pointer;
  }
  .person-summary a {
    color: var(--brand);
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .person-summary .sources {
    font-size: 12px;
    margin-top: 16px;
    color: var(--muted);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px 16px;
    margin-bottom: 16px;
  }
  h2 {
    font-size: clamp(19px, 1.6vw, 24px);
    font-weight: 700;
    margin-bottom: 16px;
  }
  .bar h2 {
    margin: 0;
  }
  .chips {
    display: flex;
    gap: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, calc(var(--pw) - 8px)), 1fr));
    gap: 22px 16px;
  }
  .sk {
    aspect-ratio: 2 / 3;
    border-radius: 12px;
  }
  @media (max-width: 980px) {
    .best {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  @media (max-width: 640px) {
    .head {
      flex-direction: column;
      text-align: center;
      gap: 16px;
    }
    .facts,
    .acts {
      justify-content: center;
    }
    .wall {
      inset: 0;
      grid-template-columns: repeat(5, minmax(0, 1fr));
    }
    h1 {
      white-space: normal;
    }
    .best,
    .grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 16px 10px;
    }
    .person-summary {
      padding: 20px;
    }
  }
  :global(.no-motion) :is(.ph, .best li) {
    animation: none;
  }
  /* «Ретроспектива»: большой кадр фоном, цветной портрет 4:5 (чёрно-белый читается как траурный - владелец, 23.11), факты крупными цифрами. */
  .banner { position: absolute; inset: 0; z-index: -2; width: 100%; height: 100%; object-fit: cover; object-position: 50% 30%; opacity: 0.5; }
  .hero.has-banner { background: #000; }
  .hero.has-banner .veil { background: linear-gradient(90deg, #000 0%, #000000c0 32%, #00000060 70%, #000 100%), linear-gradient(0deg, var(--bg) 0%, transparent 50%); }
  .hero .ph { width: clamp(150px, 21vw, 300px); aspect-ratio: 4 / 5; border-radius: 22px; }
  /* Строка чуть выше 1: у «Й» и «Ё» надстрочные знаки, при 0.92 они наезжали на подпись сверху. */
  .hero h1 { font-size: clamp(36px, 6.4vw, 92px); font-weight: 800; letter-spacing: -0.05em; line-height: 1.02; padding-top: 0.04em; white-space: normal; overflow: visible; }
  .hero .who > small { letter-spacing: 0.2em; }
  dl.facts { display: flex; flex-wrap: wrap; gap: 12px 28px; margin: 10px 0 0; }
  dl.facts div { display: flex; flex-direction: column-reverse; }
  dl.facts dd { margin: 0; font-size: clamp(20px, 2vw, 28px); font-weight: 800; letter-spacing: -0.02em; }
  dl.facts dt { color: #ffffff99; font-size: 12px; }
  .mine { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 14px; width: fit-content; max-width: 100%; margin-top: 8px; padding: 10px 14px; border-radius: 16px; background: #ffffff14; font-size: 14px; }
  .mbar { width: 140px; height: 6px; border-radius: 6px; background: #ffffff26; overflow: hidden; }
  .mbar i { display: block; height: 100%; background: #fff; transform-origin: left; }
  .mine .here { display: inline-flex; align-items: center; gap: 6px; padding: 3px 10px; border-radius: 999px; background: #fff; color: #000; font-size: 12px; font-weight: 700; }
  h2.lab { color: #8c8c96; font-size: 12px; font-weight: 700; letter-spacing: 0.16em; text-transform: uppercase; }

  /* Лучшее: огромные контурные цифры слева от постера (как чарты стримингов). Цифра - своя колонка, низом ровно по
     постеру, постер её не закрывает; подпись - отдельной строкой под постером (раньше цифра наезжала на текст). */
  .best { align-items: start; }
  .best .pw { display: grid; grid-template-columns: auto minmax(0, 1fr); grid-template-rows: auto auto; align-items: end; align-content: start; color: inherit; text-decoration: none; }
  .best .rank { position: static; grid-row: 1; grid-column: 1; width: 0.62em; height: auto; display: block; text-align: right; margin-right: -0.1em; background: none; box-shadow: none; color: #0b0b0d; font-size: clamp(84px, 8.4vw, 150px); font-weight: 800; line-height: 0.74; letter-spacing: -0.06em; -webkit-text-stroke: 2px #ffffffe6; paint-order: stroke fill; }
  .best .bp { position: relative; z-index: 1; grid-row: 1; grid-column: 2; display: block; aspect-ratio: 2 / 3; border-radius: 12px; overflow: hidden; box-shadow: 0 10px 28px #0008; transition: transform 0.3s var(--ease, ease); }
  .best .bp img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .best .pw:hover .bp { transform: translateY(-4px); }
  .best .cap { grid-row: 2; grid-column: 2; display: grid; gap: 2px; min-width: 0; padding-top: 10px; }
  .best .cap b { overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; font-size: 14px; font-weight: 600; line-height: 1.25; }
  .best .cap small { color: #a1a1aa; font-size: 12px; }

  /* Хронология: постеры на линии лет; смотренное в цвете с галочкой, остальное серым. */
  .legend { display: flex; gap: 16px; color: #8c8c96; font-size: 12.5px; }
  .legend i { display: inline-block; width: 10px; height: 10px; margin-right: 6px; border-radius: 50%; background: #fff; vertical-align: -1px; }
  .legend i.u { background: #3d3d44; }
  .tlwrap { overflow-x: auto; scrollbar-width: thin; padding-bottom: 6px; }
  .tl { position: relative; display: grid; grid-template-columns: repeat(var(--n), minmax(104px, 1fr)); gap: 12px; align-items: end; margin: 0; padding: 0 0 4px; list-style: none; min-width: min-content; }
  .tl::before { content: ''; position: absolute; left: 0; right: 0; bottom: 27px; height: 2px; background: #ffffff26; }
  .tl li { position: relative; display: grid; justify-items: center; gap: 6px; padding-bottom: 22px; }
  .tl li::after { content: ''; position: absolute; bottom: 23px; width: 10px; height: 10px; border-radius: 50%; background: #3d3d44; }
  .tl li.seen::after { background: #fff; }
  .tl a { position: relative; display: block; width: 100%; margin-bottom: 14px; }
  .tl li.up a { margin-bottom: 40px; }
  .tl img { display: block; width: 100%; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 9px; box-shadow: 0 14px 30px -14px #000; filter: grayscale(1) brightness(0.6); transition: filter 0.3s, transform 0.3s var(--ease, ease); }
  .tl li.seen img, .tl a:hover img { filter: none; }
  .tl a:hover img { transform: translateY(-4px); }
  .ck { position: absolute; top: 6px; right: 6px; width: 22px; height: 22px; display: grid; place-items: center; border-radius: 50%; background: #fff; color: #000; }
  .yr { position: absolute; bottom: 0; color: #c9c9cf; font-size: 12px; font-weight: 700; }
  @media (max-width: 640px) {
    .hero .ph { width: 180px; }
    .mine, dl.facts { justify-content: center; margin-inline: auto; }
    .best { grid-template-columns: repeat(5, minmax(150px, 1fr)); overflow-x: auto; padding-bottom: 6px; }
  }
</style>
