<script lang="ts">
  import { goto } from '$app/navigation';
  import { errorText, post } from '$lib/api';
  import { count, plural } from '$lib/fmt';
  import type { Card, ExtCard } from '$lib/types';
  import { placeholder } from '$lib/ui/art';
  import { getDetail, peekDetail } from '$lib/discover/details';
  import { artW } from '$lib/ui/art';
  import { countUp } from '$lib/ui/countUp';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import FavActors from './FavActors.svelte';
  import type { ProfilePage } from './types';

  // Вкладка «Обзор» (макет владельца в Figma 21.11): итоги четырьмя карточками, «Сейчас смотрит» на кадре,
  // любимые фильмы и сериалы карточками с оценкой, люди, жанры и лента последних просмотров.
  let { p = $bindable() }: { p: ProfilePage } = $props();

  const s = $derived(p.hidden.stats ? null : p.stats);
  const fmt1 = (n: number) => (n % 1 ? n.toFixed(1).replace('.', ',') : String(n));
  const cards = $derived.by(() => {
    if (!s) return [];
    const h = s.hours >= 100 ? Math.round(s.hours) : Math.round(s.hours * 10) / 10;
    const avg = s.ratings?.avg ?? null;
    return [
      { v: h, d: h % 1 ? 1 : 0, l: h % 1 ? 'часа в историях' : `${plural(h, 'час', 'часа', 'часов')} в историях`, sub: 'за всё время' },
      { v: s.movies, d: 0, l: plural(s.movies, 'фильм', 'фильма', 'фильмов'), sub: `${count(s.watched ?? 0, 'просмотр', 'просмотра', 'просмотров')} всего` },
      { v: s.episodes, d: 0, l: plural(s.episodes, 'серия', 'серии', 'серий'), sub: s.series ? count(s.series, 'сериал', 'сериала', 'сериалов') : 'сериалов пока нет' },
      avg != null
        ? { v: Math.round(avg * 10) / 10, d: 1, l: 'средняя оценка', sub: `на основе ${count(s.ratings?.count ?? 0, 'оценки', 'оценок', 'оценок')}` }
        : { v: s.reviews, d: 0, l: plural(s.reviews, 'отзыв', 'отзыва', 'отзывов'), sub: 'оценок пока нет' },
    ];
  });

  // «Сейчас смотрит»: первое из начатого, с кадром и местом в серии.
  const now = $derived(p.watching[0]);
  // Кадр 16:9: у своего фильма - backdrop, у фильма из ленты IMDb - из подробностей (грузим отдельно). Постер на
  // всю ширину не растягиваем (342 px давали мыло): без кадра постер стоит справа целиком.
  let extFrame = $state<string | null>(null);
  $effect(() => {
    const id = now?.card ? null : now?.ext?.id;
    extFrame = id ? (peekDetail(id)?.backdrop ?? null) : null;
    if (!id || extFrame) return;
    let live = true;
    getDetail(id).then((d) => live && (extFrame = d.backdrop ?? null), () => {});
    return () => (live = false);
  });
  const nowArt = $derived(now?.card?.backdrop ?? now?.card?.backdropSmall ?? (extFrame ? artW(extFrame, 1920) : null) ?? null);
  const nowPoster = $derived(nowArt ? null : (now?.card?.poster ?? now?.ext?.poster ?? null));
  const nowHref = $derived(now?.card ? `/title/${now.card.id}` : now?.ext ? `/ext/${now.ext.id}` : '/');
  const resume = $derived(p.me ? now?.card?.resume ?? null : null);
  const nowMeta = $derived.by(() => {
    const c = now?.card ?? now?.ext;
    if (!c) return '';
    const g = c.genres?.[0];
    const rt = now?.card?.runtime;
    return [g, c.year, rt ? `${rt} мин` : null].filter(Boolean).join(' · ');
  });
  let starting = $state(false);
  async function together() {
    if (!resume || starting) return;
    starting = true;
    try {
      const room = await post<{ id: string }>('/api/rooms', { fileId: resume.fileId, position: resume.position });
      goto(`/room/${room.id}`);
    } catch (e) {
      toast(errorText(e));
    } finally {
      starting = false;
    }
  }
  const mins = (sec: number) => Math.max(1, Math.round(sec / 60));

  // Любимое: фильм с сервиса или из ленты IMDb - одна карточка.
  type Fav = { key: string; card?: Card; ext?: ExtCard };
  const item = (f: Fav) => {
    const c = f.card ?? f.ext!;
    return {
      key: f.key,
      title: c.title,
      year: c.year,
      genre: c.genres?.[0] ?? null,
      poster: f.card?.posterLarge ?? c.poster,
      rating: c.rating,
      href: f.card ? `/title/${f.card.id}` : `/ext/${f.ext!.id}`,
      extra: f.card?.seasons ? `${f.card.seasons} сез.` : null,
    };
  };
  const favs = (kind: string) => (p.decor.favorites as Fav[]).filter((f) => (f.card ?? f.ext)?.kind === kind).slice(0, 5).map(item);
  const movies = $derived(favs('movie'));
  const series = $derived(favs('series'));

  // Люди: любимый режиссёр (сколько его фильмов смотрел) и актёры.
  const director = $derived(p.decor.directors[0] ?? null);
  const dirInfo = $derived(p.decor.directorsInfo?.find((d) => d.name === director) ?? null);

  const genres = $derived.by(() => {
    const g = (s?.genres ?? []).slice(0, 4);
    const max = Math.max(1, ...g.map((x) => x.count));
    return g.map((x) => ({ name: x.genre, count: x.count, pct: x.count / max }));
  });

  // Лента: последние досмотренные, оценка - из отзыва человека.
  const stars = $derived(new Map(p.reviews.filter((r) => r.title && r.stars).map((r) => [r.title!.id, r.stars!])));
  const recent = $derived(
    p.hidden.watched ? [] : [...p.watched].sort((a, b) => (b.watchedAt ?? 0) - (a.watchedAt ?? 0)).slice(0, 4),
  );
  const when = (ms: number | null) => {
    if (!ms) return '';
    const d = new Date(ms);
    const day = new Date();
    day.setHours(0, 0, 0, 0);
    const t = d.toLocaleTimeString('ru-RU', { hour: '2-digit', minute: '2-digit' });
    if (ms >= day.getTime()) return `Сегодня, ${t}`;
    if (ms >= day.getTime() - 864e5) return `Вчера, ${t}`;
    return `${d.toLocaleDateString('ru-RU', { day: 'numeric', month: 'long' })}, ${t}`;
  };
  const kindOf = (c: Card) => (c.kind === 'series' ? (c.seasons ? `Сериал · ${c.seasons} сез.` : 'Сериал') : 'Фильм · досмотрен');
  const empty = $derived(!now && !movies.length && !series.length && !director && !(p.decor.actors?.length) && !recent.length);
</script>

{#if cards.length}
  <section class="sums" aria-label="Итоги">
    {#each cards as c, i (c.l)}
      <div class="sum rise" style:--i={i}><b use:countUp={{ value: c.v, decimals: c.d }}>0</b><span>{c.l}</span><small>{c.sub}</small></div>
    {/each}
  </section>
{/if}

{#if now}
  <section class="now" aria-labelledby="ov-now">
    {#if nowArt}<img class="frame" use:fadeIn src={nowArt} alt="" decoding="async" />{:else if nowPoster}<img class="side" use:fadeIn src={artW(nowPoster, 342)} alt="" decoding="async" />{/if}
    <div class="nb">
      <p class="eye"><i></i>Сейчас смотрит</p>
      <h3 id="ov-now"><a href={nowHref}>{now.card?.title ?? now.ext?.title}</a></h3>
      {#if now.label || resume?.label}<p class="ep">{resume?.label ?? now.label}</p>{/if}
      {#if nowMeta}<p class="meta">{nowMeta}</p>{/if}
      <div class="nbtn">
        {#if resume}
          <a class="btn primary" href="/watch/{resume.fileId}"><Icon name="play" size={15} /> Продолжить</a>
          <button class="btn" type="button" disabled={starting} onclick={together}><Icon name="party" size={15} /> Смотреть вместе</button>
        {:else}
          <a class="btn primary" href={nowHref}><Icon name="play" size={15} /> Открыть</a>
        {/if}
      </div>
    </div>
    {#if resume && resume.duration > 0}
      <div class="prog">
        <div class="bar"><i style:transform="scaleX({Math.min(1, resume.position / resume.duration)})"></i></div>
        <small>{mins(resume.position)} из {mins(resume.duration)} мин</small>
      </div>
    {/if}
  </section>
{/if}

{#each [{ list: movies, label: 'Любимые фильмы', id: 'ov-m' }, { list: series, label: 'Любимые сериалы', id: 'ov-s' }] as g (g.id)}
  {#if g.list.length || (p.me && g.id === 'ov-s' && movies.length)}
    <section class="sec" aria-labelledby={g.id}>
      <div class="sh"><h2 id={g.id}>{g.label}</h2>{#if p.me}<a href="/profile">{g.list.length ? 'Изменить' : 'Выбрать'} <Icon name="right" size={13} /></a>{/if}</div>
      <ul class="favs" class:short={g.id === 'ov-s' && p.me}>
        {#each g.list as f, i (f.key)}
          <li class="rise" style:--i={i}>
            <a class="fav" href={f.href}>
              <span class="art" style:background={placeholder(f.title)}>
                {#if f.poster}<img use:fadeIn src={f.poster} alt="" loading="lazy" decoding="async" width="342" height="513" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
                {#if f.rating}<em><Icon name="star" size={11} /> {fmt1(f.rating)}</em>{/if}
              </span>
              <b>{f.title}</b>
              <small>{[f.year, f.genre, f.extra].filter(Boolean).join(' · ')}</small>
            </a>
          </li>
        {/each}
        {#if g.id === 'ov-s' && p.me}
          <li class="cta">
            <p class="eye">Твоя коллекция</p>
            <h3>Хорошие истории лучше вместе.</h3>
            <p>Собери любимое в подборку и поделись с друзьями.</p>
            <a href="/lists">Создать подборку <Icon name="right" size={13} /></a>
          </li>
        {/if}
      </ul>
    </section>
  {/if}
{/each}

{#if director || p.decor.actors?.length || genres.length || p.me}
  <div class="duo">
    {#if director || p.decor.actors?.length || p.me}
      <section class="box" aria-labelledby="ov-people">
        <h2 id="ov-people">Те, чьи истории нравятся</h2>
        {#if director}
          <p class="eye">Любимый режиссёр</p>
          <div class="dir">
            <a href="/person?name={encodeURIComponent(director)}&role=director">{director}</a>
            {#if dirInfo?.seen?.avg}<em><Icon name="star" size={11} /> {fmt1(Math.round(dirInfo.seen.avg * 10) / 10)}</em>{/if}
          </div>
          {#if dirInfo?.seen && dirInfo.seen.total}
            <p class="muted sm">Просмотрено {dirInfo.seen.seen} из {dirInfo.seen.total}</p>
            <div class="bar"><i style:transform="scaleX({dirInfo.seen.seen / dirInfo.seen.total})"></i></div>
          {/if}
        {:else if p.me}
          <p class="muted sm">Любимого режиссёра можно выбрать в оформлении профиля. <a class="lnk" href="/profile">Выбрать</a></p>
        {/if}
        {#if p.me || p.decor.actors?.length}
          <div class="actors"><FavActors bind:actors={p.decor.actors} suggest={(p.stats?.actors ?? []).map((a) => a.name)} editable={p.me} /></div>
        {/if}
      </section>
    {/if}
    {#if genres.length}
      <section class="box" aria-labelledby="ov-g">
        <h2 id="ov-g">Любимые жанры</h2>
        <p class="muted sm top">По числу просмотров</p>
        <ul class="gen">
          {#each genres as g, i (g.name)}
            <li><span>{g.name}</span><span class="track"><i class:lead={i === 0} style:transform="scaleX({g.pct})"></i></span><small>{g.count}</small></li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>
{/if}

{#if recent.length}
  <section class="sec" aria-labelledby="ov-r">
    <div class="sh"><h2 id="ov-r">Лента просмотренного</h2><a href="/people/{p.person.id}/watched?kind=movie">Вся история <Icon name="right" size={13} /></a></div>
    <ul class="feed">
      {#each recent as c (c.id)}
        <li>
          <a href="/title/{c.id}">
            <span class="mini" style:background={placeholder(c.title)}>{#if c.poster}<img use:fadeIn src={c.poster} alt="" loading="lazy" decoding="async" width="44" height="62" />{/if}</span>
            <span class="t"><b>{c.title}</b><small>{kindOf(c)}</small></span>
            <small class="w">{when(c.watchedAt)}</small>
            <span class="r">{#if stars.get(c.id)}<Icon name="star" size={12} /> {stars.get(c.id)}{/if}</span>
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}

{#if empty}
  <p class="muted none">{p.me ? 'Здесь появятся любимые фильмы, сериалы, режиссёр и актёры. Выберите их кнопкой «Оформить профиль».' : 'Пока здесь пусто: человек ещё ничего не выбрал.'}</p>
{/if}

<style>
  .sums { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin-bottom: 18px; }
  .sum { display: grid; gap: 2px; padding: 18px 20px; border: 1px solid #ffffff14; border-radius: 22px; background: #101012; }
  .sum b { font-size: clamp(26px, 2.4vw, 34px); font-weight: 700; letter-spacing: -0.03em; line-height: 1.1; font-variant-numeric: tabular-nums; }
  .sum span { margin-top: 6px; font-size: 13px; font-weight: 600; }
  .sum small { color: var(--muted); font-size: 11.5px; }

  .eye { display: flex; align-items: center; gap: 8px; margin: 0; color: #d4d4d8; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
  .eye i { width: 7px; height: 7px; border-radius: 50%; background: #22c55e; }
  .now { position: relative; display: grid; align-content: end; min-height: 230px; margin-bottom: 30px; overflow: hidden; border: 1px solid #ffffff14; border-radius: 26px; background: #101012; isolation: isolate; }
  .now img.frame { position: absolute; inset: 0; z-index: -2; width: 100%; height: 100%; object-fit: cover; object-position: 50% 30%; }
  .now img.side { position: absolute; top: 18px; right: 18px; bottom: 18px; z-index: 1; height: calc(100% - 36px); aspect-ratio: 2 / 3; border-radius: 14px; object-fit: cover; box-shadow: 0 18px 40px -16px #000; }
  @media (max-width: 640px) { .now img.side { display: none; } }
  .now::before { content: ''; position: absolute; inset: 0; z-index: -1; background: linear-gradient(90deg, #0a0a0bf2 0%, #0a0a0bcc 42%, #0a0a0b33 100%); }
  .nb { padding: 26px 26px 24px; }
  .now h3 { margin: 10px 0 0; font-size: clamp(26px, 2.6vw, 36px); font-weight: 800; letter-spacing: -0.03em; line-height: 1.05; }
  .ep { margin: 8px 0 0; color: #e4e4e7; font-size: 14px; }
  .meta { margin: 4px 0 0; color: var(--muted); font-size: 12.5px; }
  .nbtn { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 18px; }
  .nbtn .btn { min-height: 42px; }
  .prog { position: absolute; right: 26px; bottom: 24px; display: grid; gap: 8px; width: min(36%, 300px); }
  .prog small { color: #d4d4d8; font-size: 11.5px; }
  .bar { height: 4px; overflow: hidden; border-radius: 2px; background: #ffffff26; }
  .bar i { display: block; height: 100%; background: #fff; transform-origin: left; }

  .sec { margin-bottom: 30px; }
  .sh { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 14px; }
  .sh h2, .box h2 { margin: 0; font-size: 20px; font-weight: 700; letter-spacing: -0.02em; }
  .sh a { display: inline-flex; align-items: center; gap: 4px; color: var(--muted); font-size: 13px; }
  .sh a:hover { color: #fff; }
  .favs { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 14px; margin: 0; padding: 0; list-style: none; }
  .fav { display: grid; gap: 4px; }
  .art { position: relative; display: block; aspect-ratio: 2 / 3; margin-bottom: 8px; overflow: hidden; border-radius: 18px; transition: transform 0.3s var(--ease, ease); }
  .fav:hover .art { transform: translate3d(0, -4px, 0); }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .art em { position: absolute; top: 10px; left: 10px; display: inline-flex; align-items: center; gap: 4px; padding: 4px 8px; border-radius: 999px; background: #000000c2; font-size: 11.5px; font-style: normal; font-weight: 700; }
  .fav b { overflow: hidden; font-size: 14px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .fav small { color: var(--muted); font-size: 12px; }
  .cta { grid-column: span 2; display: grid; align-content: center; gap: 10px; padding: 24px; border: 1px solid #ffffff14; border-radius: 22px; background: #101012; }
  .cta h3 { margin: 0; font-size: clamp(20px, 1.9vw, 26px); font-weight: 700; letter-spacing: -0.02em; line-height: 1.15; }
  .cta p:not(.eye) { margin: 0; color: var(--muted); font-size: 13px; }
  .cta a { display: inline-flex; align-items: center; gap: 4px; font-size: 13px; font-weight: 700; }

  .duo { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; margin-bottom: 30px; }
  .box { padding: 22px; border: 1px solid #ffffff14; border-radius: 22px; background: #101012; }
  .box h2 { margin-bottom: 16px; font-size: 17px; }
  .box .eye { color: var(--muted); font-size: 10.5px; }
  .dir { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin: 8px 0 6px; }
  .dir a { font-size: 19px; font-weight: 700; letter-spacing: -0.01em; }
  .dir em { display: inline-flex; align-items: center; gap: 4px; padding: 5px 10px; border-radius: 999px; background: #ffffff12; font-size: 12px; font-style: normal; font-weight: 700; }
  .sm { margin: 0 0 8px; font-size: 12.5px; }
  .top { margin-top: -10px; }
  .actors { margin-top: 20px; }
  .lnk { padding: 0; border: 0; background: none; color: #fff; font: inherit; font-size: 12.5px; font-weight: 600; cursor: pointer; text-decoration: underline; text-underline-offset: 3px; }
  .gen { display: grid; gap: 14px; margin: 0; padding: 0; list-style: none; }
  .gen li { display: grid; grid-template-columns: 96px minmax(0, 1fr) 28px; align-items: center; gap: 12px; font-size: 13px; }
  .gen span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track { height: 5px; overflow: hidden; border-radius: 3px; background: #ffffff14; }
  .track i { display: block; height: 100%; background: #ffffff73; transform-origin: left; }
  .track i.lead { background: #fff; }
  .gen small { color: var(--muted); text-align: right; }

  .feed { margin: 0; padding: 6px 18px; border: 1px solid #ffffff14; border-radius: 22px; background: #101012; list-style: none; }
  .feed li + li { border-top: 1px solid #ffffff0f; }
  .feed a { display: grid; grid-template-columns: 44px minmax(0, 1fr) auto 52px; align-items: center; gap: 16px; padding: 12px 0; }
  .feed a:hover b { text-decoration: underline; }
  .mini { width: 44px; height: 62px; overflow: hidden; border-radius: 8px; }
  .mini img { width: 100%; height: 100%; object-fit: cover; }
  .t { display: grid; gap: 3px; min-width: 0; }
  .t b { overflow: hidden; font-size: 14px; text-overflow: ellipsis; white-space: nowrap; }
  .t small, .w { color: var(--muted); font-size: 12px; }
  .r { display: inline-flex; align-items: center; justify-content: flex-end; gap: 4px; font-size: 13px; font-weight: 700; }
  .none { padding: 30px 0; text-align: center; }

  @media (max-width: 1200px) {
    .favs { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  }
  @media (max-width: 1000px) {
    .sums { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .favs { grid-template-columns: repeat(3, minmax(0, 1fr)); }
    .cta { grid-column: 1 / -1; }
    .duo { grid-template-columns: minmax(0, 1fr); }
    .prog { position: static; width: auto; padding: 0 26px 22px; }
  }
  @media (max-width: 640px) {
    .favs { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .feed a { grid-template-columns: 44px minmax(0, 1fr) 40px; }
    .w { display: none; }
    .nb { padding: 20px; }
  }
</style>
