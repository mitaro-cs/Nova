<script lang="ts">
  import { goto } from '$app/navigation';
  import { post } from '$lib/api';
  import { badges as unseen } from '$lib/social/badges.svelte';
  import { session } from '$lib/session.svelte';
  import type { Card, ExtCard } from '$lib/types';
  import { artW, placeholder } from '$lib/ui/art';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';
  import Badge from './Badge.svelte';
  import FavActors from './FavActors.svelte';
  import { GOALS, nextGoalOf } from './level';
  import type { ProfilePage } from './types';

  // Вкладка «Профиль» - «Касса кинотеатра» (выбор владельца, 23.11): значки-«цветки» друг за другом, веер билетов
  // «недавно смотрел(а)» (то, что смотрели у нас, в Nova), любимые фильмы и сериалы в световых коробах, любимый
  // режиссёр и актёры, у владельца - быстрые разделы плитками.
  let { p = $bindable(), toReviews }: { p: ProfilePage; toReviews: () => void } = $props();

  type Fav = { key: string; card?: Card; ext?: ExtCard };
  const tile = (f: Fav) => {
    const c = f.card ?? f.ext!;
    return { key: f.key, title: c.title, year: c.year, poster: f.card?.posterLarge ?? c.poster, href: f.card ? `/title/${f.card.id}` : `/ext/${f.ext!.id}` };
  };
  const favs = (kind: string) => (p.decor.favorites as Fav[]).filter((f) => (f.card ?? f.ext)?.kind === kind).slice(0, 6).map(tile);
  const movies = $derived(favs('movie'));
  const series = $derived(favs('series'));

  const s = $derived(p.hidden.stats ? null : p.stats);
  const goals = $derived(s ? GOALS.map((g) => ({ ...g, got: g.of(s) >= g.need, progress: g.of(s) / g.need })) : []);
  const nextGoal = $derived(p.me ? nextGoalOf(s) : null);

  // Билеты: сначала то, что идёт «на экране» сейчас, потом последние досмотренные. Номер сеанса - по счёту просмотров.
  const starsOf = $derived(new Map(p.reviews.filter((r) => r.title && r.stars).map((r) => [r.title!.id, r.stars!])));
  const recent = $derived(p.hidden.watched ? [] : [...p.watched].sort((a, b) => (b.watchedAt ?? 0) - (a.watchedAt ?? 0)));
  const total = $derived(Math.max(s?.watched ?? 0, recent.length));
  const day = (ms: number | null | undefined) => {
    if (!ms) return '';
    const d = new Date(ms), today = new Date();
    const diff = Math.round((new Date(today.toDateString()).getTime() - new Date(d.toDateString()).getTime()) / 864e5);
    return diff === 0 ? 'сегодня' : diff === 1 ? 'вчера' : d.toLocaleDateString('ru-RU', { day: 'numeric', month: 'long' });
  };
  type Ticket = { key: string; href: string; title: string; poster: string | null; label: string; note: string; stars: number | null; now: boolean };
  const tickets = $derived.by<Ticket[]>(() => {
    const out: Ticket[] = [];
    const now = p.watching[0];
    const nc = now?.card ?? now?.ext;
    if (now && nc) {
      out.push({ key: `now:${now.card ? `t${now.card.id}` : now.ext?.id}`, href: now.card ? `/title/${now.card.id}` : `/ext/${now.ext!.id}`, title: nc.title, poster: nc.poster ?? null, label: 'Сейчас на экране', note: now.label, stars: null, now: true });
    }
    recent.slice(0, 5 - out.length).forEach((c, i) => {
      out.push({ key: `t${c.id}`, href: `/title/${c.id}`, title: c.title, poster: c.poster ?? null, label: `Сеанс № ${total - i}`, note: day(c.watchedAt), stars: starsOf.get(c.id) ?? null, now: false });
    });
    return out;
  });
  const recentTitle = $derived(p.me ? 'Вы недавно смотрели' : 'Недавно смотрел(а)');

  // Свежие отзывы - два последних, карточкой с постером (вместо одинокой кнопки «Отзывы · N» внизу).
  // Свежие отзывы карточками (26.11 «отзывы не очень выглядят»): постер, название со ссылкой, пять звёзд с половинками,
  // дата; текст - если он есть, иначе одна тихая строка «Только оценка».
  const fresh = $derived(
    p.reviews.slice(0, 3).map((r) => ({
      id: r.id, stars: r.stars, body: r.body?.trim() ?? '', at: r.createdAt,
      name: r.title?.title ?? r.ext?.title ?? '', year: r.title?.year ?? r.ext?.year ?? null,
      poster: r.title?.poster ?? r.ext?.poster ?? null,
      href: r.title ? `/title/${r.title.id}` : r.ext ? `/ext/${r.ext.id}` : null,
    })),
  );
  const fill = (stars: number, i: number) => Math.max(0, Math.min(1, (stars - i * 2) / 2));
  const director = $derived(p.decor.directorsInfo?.[0] ?? (p.decor.directors[0] ? { name: p.decor.directors[0], photo: null, seen: null } : null));

  type Quick = { href?: string; act?: () => void; icon: IconName; label: string; hint: string; count?: number; out?: boolean };
  async function logout() {
    await post('/api/auth/logout').catch(() => {});
    session.me = null;
    goto('/login');
  }
  const quick = $derived.by<Quick[]>(() => [
    { href: '/chats', icon: 'mail', label: 'Чаты', hint: 'Переписка с друзьями', count: unseen.unread },
    { href: '/people', icon: 'users', label: 'Друзья', hint: 'Найти и позвать', count: unseen.requests },
    { href: '/my', icon: 'grid', label: 'Подборки', hint: 'Отметки и общие подборки' },
    { act: () => window.dispatchEvent(new Event('nova-room-link')), icon: 'party', label: 'Создать комнату', hint: 'Смотреть вместе' },
    ...(session.me?.manage ? [{ href: '/admin', icon: 'sliders' as IconName, label: 'Управление', hint: 'Карта дома и настройки' }] : []),
    { act: () => window.dispatchEvent(new Event('nova-tour')), icon: 'sparkle', label: 'Экскурсия', hint: 'Показать, что где' },
    ...(!session.me?.window ? [{ act: logout, icon: 'logout' as IconName, label: 'Выйти', hint: 'Из этого браузера', out: true }] : []),
  ]);
</script>

<div class="ks" class:me={p.me}>
  <!-- «Широкий экран» A1 (выбор владельца 25.11): три колонки на всю ширину - слева быстрые разделы, в центре главное,
       справа значки и любимые люди. Уже 1280 px - разделы капсулами сверху, ещё уже - одна колонка. -->
  {#if p.me}
    <nav class="quick" aria-label="Быстрые разделы">
      <h2 class="qh">Быстрые разделы</h2>
      {#each quick as q (q.label)}
        {#if q.href}
          <a href={q.href}><span class="qi"><Icon name={q.icon} size={17} /></span><b>{q.label}</b>{#if q.count}<em>{q.count}</em>{/if}</a>
        {:else}
          <button type="button" class:out={q.out} onclick={q.act}><span class="qi"><Icon name={q.icon} size={17} /></span><b>{q.label}</b></button>
        {/if}
      {/each}
    </nav>
  {/if}

  <div class="main">
    {#if tickets.length}
      <section class="box" aria-labelledby="ks-t">
        <div class="h2"><h2 id="ks-t">{recentTitle}</h2>{#if recent.length}<a href="/people/{p.person.id}/watched?kind=movie">Все</a>{/if}</div>
        <ul class="tickets">
          {#each tickets as t (t.key)}
            <li>
              <a class="tk" class:now={t.now} href={t.href}>
                <span class="tp" style:background={placeholder(t.title)}>{#if t.poster}<img use:fadeIn src={artW(t.poster, 200)} alt="" loading="lazy" decoding="async" width="56" height="84" />{/if}</span>
                <span class="tt"><small>{t.label}</small><b>{t.title}</b><em>{t.note}{t.stars ? ` · ★ ${(t.stars / 2).toFixed(1).replace('.0', '').replace('.', ',')}` : ''}</em></span>
              </a>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#each [{ list: movies, label: 'Любимые фильмы', id: 'ks-m' }, { list: series, label: 'Любимые сериалы', id: 'ks-s' }] as g (g.id)}
      {#if g.list.length || (p.me && g.id === 'ks-m')}
        <section class="box" aria-labelledby={g.id}>
          <div class="h2"><h2 id={g.id}>{g.label}</h2>{#if p.me}<a href="/profile">{g.list.length ? 'Изменить' : 'Выбрать'}</a>{/if}</div>
          <ul class="boxes" class:blank={!g.list.length}>
            {#each g.list as f, i (f.key)}
              <li class="rise" style:--i={i}>
                <a class="lb" href={f.href} title="{f.title}{f.year ? `, ${f.year}` : ''}">
                  <span class="po" style:background={placeholder(f.title)}>{#if f.poster}<img use:fadeIn src={artW(f.poster, 500)} alt={f.title} loading="lazy" decoding="async" width="342" height="513" />{:else}<span>{f.title}</span>{/if}</span>
                </a>
              </li>
            {/each}
            {#if p.me}
              {#each Array(Math.max(0, 5 - g.list.length)) as _, i (i)}<li><a class="slot" href="/profile" aria-label="Добавить: {g.label.toLowerCase()}"><Icon name="plus" size={20} /></a></li>{/each}
            {/if}
          </ul>
        </section>
      {/if}
    {/each}

    {#if fresh.length}
      <section class="box" aria-labelledby="ks-r">
        <div class="h2"><h2 id="ks-r">Свежие отзывы</h2><button type="button" class="lnk" onclick={toReviews}>Все {p.stats?.reviews ?? p.reviews.length}</button></div>
        <ul class="revs">
          {#each fresh as r (r.id)}
            <li>
              <svelte:element this={r.href ? 'a' : 'div'} class="rv" href={r.href ?? undefined}>
                <span class="rp" style:background={placeholder(r.name)}>{#if r.poster}<img src={artW(r.poster, 200)} alt="" loading="lazy" decoding="async" width="56" height="84" />{/if}</span>
                <span class="rt">
                  <b>{r.name}{#if r.year}<small> · {r.year}</small>{/if}</b>
                  {#if r.stars}
                    <span class="stars" role="img" aria-label="Оценка {(r.stars / 2).toFixed(1).replace('.0', '').replace('.', ',')} из 5">
                      {#each [0, 1, 2, 3, 4] as i (i)}<i style:--f={fill(r.stars, i)}></i>{/each}
                      <em>{(r.stars / 2).toFixed(1).replace('.0', '').replace('.', ',')}</em>
                    </span>
                  {/if}
                  {#if r.body}<span class="tx">{r.body}</span>{:else}<span class="only">Только оценка</span>{/if}
                  <small class="dt">{day(r.at)}</small>
                </span>
              </svelte:element>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if !tickets.length && !movies.length && !series.length && !p.me}<p class="muted none">Пока здесь пусто: человек ещё ничего не посмотрел.</p>{/if}
  </div>

  <aside class="side" aria-label="Значки и любимые люди">
    {#if goals.length}
      <section class="box medals" aria-labelledby="ks-b">
        <div class="h2"><h2 id="ks-b">Значки</h2><small>{goals.filter((g) => g.got).length} из {goals.length}</small></div>
        <ul>
          {#each goals.slice(0, 6) as g, i (g.label)}
            <li class:got={g.got} title={g.got ? g.label : `${g.label}: ${g.hint}`}>
              <Badge icon={g.icon} tone={i} got={g.got} progress={g.progress} />
              <b>{g.label}</b><small>{g.hint}</small>
            </li>
          {/each}
        </ul>
        {#if nextGoal}<p class="next">Дальше: {nextGoal}</p>{/if}
      </section>
    {/if}

    {#if director || p.decor.actors?.length || p.me}
      <section class="box" aria-labelledby="ks-p">
        <div class="h2"><h2 id="ks-p">Любимые люди</h2></div>
        {#if director}
          <a class="dir" href="/person?name={encodeURIComponent(director.name)}&role=director">
            <span class="ph">{#if director.photo}<img use:fadeIn src={director.photo} alt="" loading="lazy" width="56" height="56" />{:else}<Icon name="film" size={22} />{/if}</span>
            <span><small>Режиссёр</small><b>{director.name}</b>{#if director.seen}<em>просмотрено {director.seen.seen} из {director.seen.total}</em>{/if}</span>
          </a>
        {/if}
        <div class="actors"><FavActors bind:actors={p.decor.actors} suggest={(p.stats?.actors ?? []).map((a) => a.name)} editable={p.me} /></div>
      </section>
    {/if}
  </aside>
</div>

<style>
  /* Три колонки во всю ширину (A1): у своего профиля слева разделы 280 px, справа 360 px; у чужого - главное и справа. */
  .ks { display: grid; grid-template-columns: minmax(0, 1fr) 360px; grid-template-areas: 'main side'; gap: 20px; min-width: 0; }
  .ks.me { grid-template-columns: 280px minmax(0, 1fr) 360px; grid-template-areas: 'quick main side'; }
  .qh { display: none; }
  .quick { grid-area: quick; }
  .main { grid-area: main; display: grid; gap: 16px; align-content: start; min-width: 0; }
  .side { grid-area: side; display: grid; gap: 16px; align-content: start; min-width: 0; }
  section { min-width: 0; }
  .box { padding: 18px; border: 1px solid #ffffff12; border-radius: 20px; background: #131316; }
  .h2 { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 14px; }
  .h2 h2 { margin: 0; color: #8c8c96; font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; }
  .h2 a, .lnk { padding: 0; border: 0; background: none; color: #fff; font: inherit; font-size: 11px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; text-decoration: none; cursor: pointer; }
  .h2 small { color: #8c8c96; font-size: 12px; text-align: right; }
  .none { padding: 30px 0; text-align: center; }

  /* Быстрые разделы - сверху, одной строкой капсул (на узком - лентой вбок). */
  .quick { display: flex; gap: 8px; overflow-x: auto; scrollbar-width: none; padding: 2px; margin: -2px; }
  .quick::-webkit-scrollbar { display: none; }
  .quick a, .quick button { flex: none; display: inline-flex; align-items: center; gap: 8px; height: 44px; padding: 0 16px 0 6px; border: 1px solid #ffffff14; border-radius: 999px; background: #151518; color: #fff; font: inherit; font-size: 14px; text-decoration: none; cursor: pointer; transition: background 0.2s; }
  .quick a:hover, .quick button:hover { background: #1f1f24; }
  .qi { width: 32px; height: 32px; flex: none; display: grid; place-items: center; border-radius: 50%; background: #ffffff14; }
  .quick b { font-weight: 600; }
  .quick em { padding: 1px 7px; border-radius: 999px; background: #fff; color: #000; font-size: 11px; font-style: normal; font-weight: 700; }
  .quick .out { color: #ffb4b4; }
  /* Широкое окно: разделы - список в своей панели слева, панель липнет под шапкой. */
  @media (min-width: 1281px) {
    .quick { position: sticky; top: calc(var(--header) + 16px); align-self: start; flex-direction: column; gap: 2px; margin: 0; padding: 14px 10px; overflow: visible; border: 1px solid #ffffff12; border-radius: 20px; background: #131316; }
    .qh { display: block; margin: 0 8px 8px; color: #8c8c96; font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; }
    .quick a, .quick button { height: 46px; padding: 0 12px 0 6px; border: 0; border-radius: 14px; background: none; text-align: left; }
    .quick a:hover, .quick button:hover { background: #ffffff0f; }
    .quick em { margin-left: auto; }
  }
  @media (max-width: 1280px) {
    .ks.me { grid-template-columns: minmax(0, 1fr) 340px; grid-template-areas: 'quick quick' 'main side'; }
  }

  /* Билеты сеансов - прямым рядом (было: веер по центру с пустотой по бокам). */
  .tickets { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 10px; margin: 0; padding: 0; list-style: none; }
  .tk { position: relative; display: grid; grid-template-columns: 56px minmax(0, 1fr); gap: 12px; align-items: center; padding: 8px; border-radius: 14px; background: #f4f4f6; color: #111; text-decoration: none; transition: transform 0.25s var(--ease, ease); }
  .tk:hover { transform: translateY(-3px); }
  .tk::before, .tk::after { content: ''; position: absolute; left: 70px; width: 12px; height: 12px; border-radius: 50%; background: #131316; }
  .tk::before { top: -6px; } .tk::after { bottom: -6px; }
  .tk.now { box-shadow: 0 0 0 2px #fff; }
  .tp { width: 56px; aspect-ratio: 2 / 3; border-radius: 7px; overflow: hidden; }
  .tp img { width: 100%; height: 100%; object-fit: cover; }
  .tt { display: grid; gap: 2px; min-width: 0; padding-left: 4px; }
  .tt small { color: #0009; font-size: 9.5px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
  .tt b { font-size: 14px; line-height: 1.25; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tt em { color: #000a; font-size: 12px; font-style: normal; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  /* Любимые - световые короба; пустые места - «+» (без длинной строки текста). */
  .boxes { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; margin: 0; padding: 0; list-style: none; }
  .lb { display: block; padding: 6px; border-radius: 14px; background: #1a1a1e; box-shadow: inset 0 0 0 1px #ffffff1f, 0 0 30px -12px #ffffff40; transition: transform 0.3s var(--ease, ease); }
  .lb:hover { transform: translateY(-4px); }
  .po { display: grid; place-items: center; aspect-ratio: 2 / 3; border-radius: 8px; overflow: hidden; color: #fff; font-size: 13px; font-weight: 700; text-align: center; }
  .po img { width: 100%; height: 100%; object-fit: cover; }
  .slot { display: grid; place-items: center; aspect-ratio: 2 / 3; border: 1.5px dashed #ffffff2e; border-radius: 14px; color: #8c8c96; transition: border-color 0.2s, color 0.2s; }
  .slot:hover { border-color: #ffffff80; color: #fff; }
  /* Пока любимых нет - пять маленьких мест, а не пять больших пустых коробов. */
  .boxes.blank { grid-template-columns: repeat(5, minmax(0, 72px)); gap: 8px; }
  .boxes.blank .slot { border-radius: 10px; }

  /* Отзывы - карточки в ряд (на узком - лента вбок): постер, звёзды, текст или «Только оценка», дата. */
  .revs { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 260px), 1fr)); gap: 10px; margin: 0; padding: 0; list-style: none; }
  .rv { display: grid; grid-template-columns: 56px minmax(0, 1fr); gap: 14px; height: 100%; padding: 12px; border: 1px solid #ffffff0f; border-radius: 16px; background: #1a1a1e; color: inherit; text-decoration: none; transition: background 0.2s; }
  a.rv:hover { background: #212126; }
  .rp { width: 56px; aspect-ratio: 2 / 3; border-radius: 8px; overflow: hidden; }
  .rp img { width: 100%; height: 100%; object-fit: cover; }
  .rt { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .rt b { overflow: hidden; font-size: 14px; font-weight: 700; text-overflow: ellipsis; white-space: nowrap; }
  .rt b small { color: #8c8c96; font-size: 12px; font-weight: 500; }
  .stars { display: inline-flex; align-items: center; gap: 2px; }
  .stars i { position: relative; width: 14px; height: 14px; background: #ffffff2e; clip-path: polygon(50% 0, 61% 35%, 98% 35%, 68% 57%, 79% 91%, 50% 70%, 21% 91%, 32% 57%, 2% 35%, 39% 35%); }
  .stars i::after { content: ''; position: absolute; inset: 0; background: #fff; transform-origin: left; transform: scaleX(var(--f)); }
  .stars em { margin-left: 6px; color: #d4d4d8; font-size: 12px; font-style: normal; font-weight: 700; font-variant-numeric: tabular-nums; }
  .tx { color: #c9c9cf; font-size: 13.5px; line-height: 1.45; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .only { color: #8c8c96; font-size: 12.5px; }
  .dt { margin-top: auto; color: #71717a; font-size: 11.5px; }

  /* Значки - сетка 3 × 2 в узкой колонке. */
  .medals ul { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px 6px; margin: 0; padding: 0; list-style: none; }
  .medals li { display: grid; justify-items: center; gap: 4px; text-align: center; }
  .medals li:hover :global(.badge) { transform: rotate(-12deg) scale(1.08); }
  .medals b { font-size: 12px; }
  .medals li:not(.got) b { color: #8c8c96; }
  .medals small { color: #8c8c96; font-size: 10.5px; line-height: 1.25; }
  .next { margin: 12px 0 0; color: #a1a1aa; font-size: 12px; }

  .dir { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; color: #fff; text-decoration: none; }
  .dir .ph { width: 48px; height: 48px; flex: none; border-radius: 50%; overflow: hidden; display: grid; place-items: center; background: #ffffff12; color: #8c8c96; }
  .dir .ph img { width: 100%; height: 100%; object-fit: cover; }
  .dir small { display: block; color: #8c8c96; font-size: 11px; }
  .dir b { display: block; font-size: 15px; }
  .dir em { display: block; color: #a1a1aa; font-size: 12px; font-style: normal; }
  .actors { min-width: 0; }

  /* Телефон и планшет: одна колонка - быстрые разделы, главное, потом значки и люди. */
  @media (max-width: 1000px) {
    .ks, .ks.me { grid-template-columns: minmax(0, 1fr); grid-template-areas: 'quick' 'main' 'side'; gap: 12px; }
    .main, .side { gap: 12px; }
    .box { padding: 14px; border-radius: 18px; }
    .tickets { display: flex; overflow-x: auto; scrollbar-width: none; }
    .tickets li { flex: 0 0 min(240px, 74vw); }
    .medals ul { grid-template-columns: repeat(6, minmax(0, 1fr)); }
  }
  @media (max-width: 640px) {
    .boxes { grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
    .boxes li:nth-child(n + 5) { display: none; }
    .lb { padding: 4px; border-radius: 10px; }
    .slot { border-radius: 10px; }
    .medals ul { grid-template-columns: repeat(3, minmax(0, 1fr)); }
    .quick a, .quick button { height: 40px; font-size: 13px; }
  }
</style>
