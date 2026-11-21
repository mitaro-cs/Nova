<script lang="ts">
  import { goto } from '$app/navigation';
  import { post } from '$lib/api';
  import StarsShow from '$lib/social/StarsShow.svelte';
  import { badges as unseen } from '$lib/social/badges.svelte';
  import { session } from '$lib/session.svelte';
  import type { Card, ExtCard } from '$lib/types';
  import { artW, placeholder } from '$lib/ui/art';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import FavActors from './FavActors.svelte';
  import { GOALS, nextGoalOf } from './level';
  import type { ProfilePage } from './types';

  // Вкладка «Профиль» в виде дневника (макет Y, 21.11): слева любимые, недавнее и свежие отзывы, справа узкая
  // колонка - сейчас смотрит, оценки, дневник по датам, любимые люди, достижения и быстрые разделы владельца.
  let { p = $bindable(), toReviews }: { p: ProfilePage; toReviews: () => void } = $props();

  type Fav = { key: string; card?: Card; ext?: ExtCard };
  const tile = (f: Fav) => {
    const c = f.card ?? f.ext!;
    return { key: f.key, title: c.title, year: c.year, poster: f.card?.posterLarge ?? c.poster, href: f.card ? `/title/${f.card.id}` : `/ext/${f.ext!.id}` };
  };
  const favs = (kind: string) => (p.decor.favorites as Fav[]).filter((f) => (f.card ?? f.ext)?.kind === kind).slice(0, 4).map(tile);
  const movies = $derived(favs('movie'));
  const series = $derived(favs('series'));
  const favIds = $derived(new Set((p.decor.favorites as Fav[]).flatMap((f) => (f.card ? [f.card.id] : []))));

  // Оценка - из отзыва человека (звёзды 1..10 = половинки).
  const starsOf = $derived(new Map(p.reviews.filter((r) => r.title && r.stars).map((r) => [r.title!.id, r.stars!])));
  const recent = $derived(p.hidden.watched ? [] : [...p.watched].sort((a, b) => (b.watchedAt ?? 0) - (a.watchedAt ?? 0)));
  const MONTHS = ['янв', 'фев', 'мар', 'апр', 'май', 'июн', 'июл', 'авг', 'сен', 'окт', 'ноя', 'дек'];
  const diary = $derived(
    recent.slice(0, 7).map((c, i, all) => {
      const d = c.watchedAt ? new Date(c.watchedAt) : null;
      const prev = i > 0 && all[i - 1].watchedAt ? new Date(all[i - 1].watchedAt!) : null;
      const month = d && (!prev || prev.getMonth() !== d.getMonth() || prev.getFullYear() !== d.getFullYear()) ? MONTHS[d.getMonth()] : '';
      return { c, month, day: d ? d.getDate() : '' };
    }),
  );
  const reviews = $derived([...p.reviews].sort((a, b) => b.createdAt - a.createdAt).slice(0, 2));
  const what = (r: ProfilePage['reviews'][number]) => r.ext ?? r.title!;
  const link = (r: ProfilePage['reviews'][number]) => (r.ext ? `/ext/${r.ext.id}` : `/title/${r.title?.id}`);
  const day = (ms: number) => new Date(ms).toLocaleDateString('ru-RU', { day: 'numeric', month: 'long' });

  const now = $derived(p.watching[0] ?? null);
  const nowCard = $derived(now?.card ?? now?.ext ?? null);
  const nowHref = $derived(now?.card ? `/title/${now.card.id}` : now?.ext ? `/ext/${now.ext.id}` : '/');
  const resume = $derived(p.me ? (now?.card?.resume ?? null) : null);

  const s = $derived(p.hidden.stats ? null : p.stats);
  const hist = $derived(s?.ratings?.hist ?? []);
  const histMax = $derived(Math.max(1, ...hist));
  const avg = $derived(s?.ratings?.avg ?? null);

  const goals = $derived(s ? GOALS.map((g) => ({ ...g, got: g.of(s) >= g.need })) : []);
  const nextGoal = $derived(p.me ? nextGoalOf(s) : null);
  const director = $derived(p.decor.directors[0] ?? null);

  async function logout() {
    await post('/api/auth/logout').catch(() => {});
    session.me = null;
    goto('/login');
  }
  const empty = $derived(!movies.length && !series.length && !recent.length && !reviews.length);
</script>

<div class="dy">
  <div class="main">
    <div class="favrow" class:pair={movies.length > 0 && series.length > 0}>
    {#each [{ list: movies, label: 'Любимые фильмы', id: 'dy-m' }, { list: series, label: 'Любимые сериалы', id: 'dy-s' }] as g (g.id)}
      {#if g.list.length || (p.me && g.id === 'dy-m')}
        <section aria-labelledby={g.id}>
          <div class="h2"><h2 id={g.id}>{g.label}</h2>{#if p.me}<a href="/profile">{g.list.length ? 'Изменить' : 'Выбрать'}</a>{/if}</div>
          {#if g.list.length}
            <ul class="favs">
              {#each g.list as f, i (f.key)}
                <li class="rise" style:--i={i}>
                  <a class="po" href={f.href} title="{f.title}{f.year ? `, ${f.year}` : ''}" style:background={placeholder(f.title)}>
                    {#if f.poster}<img use:fadeIn src={artW(f.poster, 500)} alt={f.title} loading="lazy" decoding="async" width="342" height="513" />{:else}<span>{f.title}</span>{/if}
                  </a>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="muted sm">Четыре любимых фильма - визитная карточка профиля. Выберите их в оформлении.</p>
          {/if}
        </section>
      {/if}
    {/each}
    </div>

    {#if recent.length}
      <section aria-labelledby="dy-r">
        <div class="h2"><h2 id="dy-r">Недавно</h2><a href="/people/{p.person.id}/watched?kind=movie">Все</a></div>
        <ul class="recent">
          {#each recent.slice(0, 16) as c, i (c.id)}
            <li class="rise" style:--i={i}>
              <a class="po" href="/title/{c.id}" title={c.title} style:background={placeholder(c.title)}>
                {#if c.poster}<img use:fadeIn src={c.poster} alt={c.title} loading="lazy" decoding="async" width="342" height="513" />{:else}<span>{c.title}</span>{/if}
              </a>
              <div class="under">
                {#if starsOf.get(c.id)}<StarsShow value={starsOf.get(c.id)!} size={12} />{/if}
                {#if favIds.has(c.id)}<span class="heart" title="Любимый"><Icon name="heart" size={12} /></span>{/if}
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if reviews.length}
      <section aria-labelledby="dy-rv">
        <div class="h2"><h2 id="dy-rv">Свежие отзывы</h2><button type="button" onclick={toReviews}>Все {p.stats?.reviews ?? p.reviews.length}</button></div>
        <ul class="revs">
          {#each reviews as r (r.id)}
            {@const t = what(r)}
            <li>
              <a class="po mini" href={link(r)} style:background={placeholder(t.title)} aria-hidden="true" tabindex="-1">
                {#if t.poster}<img use:fadeIn src={t.poster} alt="" loading="lazy" decoding="async" width="70" height="105" />{/if}
              </a>
              <div>
                <a class="rt" href={link(r)}><b>{t.title}</b>{#if t.year}{' '}<span>{t.year}</span>{/if}</a>
                <div class="rm">{#if r.stars}<StarsShow value={r.stars} size={12} />{/if}<small>{day(r.createdAt)}</small></div>
                {#if r.body.trim()}<p class="selectable">{r.body}</p>{/if}
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if empty && !p.me}<p class="muted none">Пока здесь пусто: человек ещё ничего не отметил.</p>{/if}
  </div>

  <aside class="side" aria-label="Коротко">
    {#if now && nowCard}
      <section aria-labelledby="dy-now">
        <div class="h2"><h2 id="dy-now">Сейчас смотрит</h2></div>
        <a class="now" href={resume ? `/watch/${resume.fileId}` : nowHref}>
          <span class="po mini" style:background={placeholder(nowCard.title)}>{#if nowCard.poster}<img src={nowCard.poster} alt="" loading="lazy" width="62" height="93" />{/if}</span>
          <span class="nt">
            <b>{nowCard.title}</b>
            <small>{resume?.label ?? now.label}</small>
            {#if resume && resume.duration > 0}<i class="bar"><i style:transform="scaleX({Math.min(1, resume.position / resume.duration)})"></i></i>{/if}
            <em>{resume ? 'Продолжить' : 'Открыть'} <Icon name="right" size={12} /></em>
          </span>
        </a>
      </section>
    {/if}

    {#if hist.length && s?.ratings?.count}
      <section aria-labelledby="dy-h">
        <div class="h2"><h2 id="dy-h">Оценки</h2>{#if avg != null}<small>в среднем {avg.toFixed(1).replace('.', ',')} из 10</small>{/if}</div>
        <div class="hist" role="img" aria-label="Разброс оценок">
          {#each hist as v, i (i)}<i class:top={v === histMax && v > 0} style:height="{Math.round((v / histMax) * 64) + 3}px" title="{(i + 1) / 2} ★: {v}"></i>{/each}
        </div>
        <div class="hl"><span>½</span><span>★★★★★</span></div>
      </section>
    {/if}

    {#if diary.length}
      <section aria-labelledby="dy-d">
        <div class="h2"><h2 id="dy-d">Дневник</h2><a href="/people/{p.person.id}/watched?kind=movie">Весь</a></div>
        <ol class="diary">
          {#each diary as d (d.c.id)}
            <li><span class="mo">{d.month}</span><span class="dd">{d.day}</span><a href="/title/{d.c.id}">{d.c.title}</a>{#if starsOf.get(d.c.id)}<StarsShow value={starsOf.get(d.c.id)!} size={11} />{/if}</li>
          {/each}
        </ol>
      </section>
    {/if}

    {#if director || p.me || p.decor.actors?.length}
      <section aria-labelledby="dy-p">
        <div class="h2"><h2 id="dy-p">Любимые люди</h2></div>
        {#if director}<a class="dir" href="/person?name={encodeURIComponent(director)}&role=director"><small>Режиссёр</small>{director}</a>{/if}
        <div class="actors"><FavActors bind:actors={p.decor.actors} suggest={(p.stats?.actors ?? []).map((a) => a.name)} editable={p.me} /></div>
      </section>
    {/if}

    {#if goals.length}
      <section aria-labelledby="dy-a">
        <div class="h2"><h2 id="dy-a">Достижения</h2><small>{goals.filter((g) => g.got).length} из {goals.length}</small></div>
        <ul class="ach" aria-label="Значки">
          {#each goals as g (g.label)}<li class:got={g.got} title={g.got ? g.label : `${g.label}: ${g.hint}`}><span><Icon name={g.icon} size={16} /></span><small>{g.label}</small></li>{/each}
        </ul>
        {#if nextGoal}<p class="goal">{nextGoal}</p>{/if}
      </section>
    {/if}

    {#if p.me}
      <nav class="quick" aria-label="Быстрые разделы">
        <a href="/chats"><Icon name="mail" size={15} /> Чаты{#if unseen.unread}<em>{unseen.unread}</em>{/if}</a>
        <a href="/people"><Icon name="users" size={15} /> Друзья{#if unseen.requests}<em>{unseen.requests}</em>{/if}</a>
        <a href="/lists"><Icon name="grid" size={15} /> Подборки</a>
        {#if session.me?.manage}<a href="/admin"><Icon name="sliders" size={15} /> Управление</a>{/if}
        <button type="button" onclick={() => window.dispatchEvent(new Event('nova-room-link'))}><Icon name="party" size={15} /> Комната по ссылке</button>
        <button type="button" onclick={() => window.dispatchEvent(new Event('nova-tour'))}><Icon name="sparkle" size={15} /> Экскурсия</button>
        {#if !session.me?.window}<button type="button" class="out" onclick={logout}><Icon name="logout" size={15} /> Выйти</button>{/if}
      </nav>
    {/if}
  </aside>
</div>

<style>
  .dy { display: grid; grid-template-columns: minmax(0, 1fr) 300px; gap: 56px; }
  .main { display: grid; align-content: start; gap: 40px; min-width: 0; }
  .side { display: grid; align-content: start; gap: 34px; min-width: 0; }
  .h2 { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 16px; padding-bottom: 10px; border-bottom: 1px solid #ffffff1a; }
  .h2 h2 { margin: 0; color: #8c8c96; font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; }
  .h2 a, .h2 button { padding: 0; border: 0; background: none; color: #8c8c96; font: inherit; font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; cursor: pointer; }
  .h2 a:hover, .h2 button:hover { color: #fff; }
  .h2 small { color: #8c8c96; font-size: 12px; }
  .sm { margin: 0; font-size: 13px; }
  .none { padding: 30px 0; text-align: center; }

  .po { position: relative; display: block; aspect-ratio: 2 / 3; overflow: hidden; border-radius: 6px; box-shadow: 0 12px 26px -16px #000; transition: transform 0.3s var(--ease, ease); }
  .po::after { content: ''; position: absolute; inset: 0; border-radius: 6px; box-shadow: inset 0 0 0 1px #ffffff24; pointer-events: none; }
  a.po:hover { transform: translate3d(0, -4px, 0); }
  .po img { display: block; width: 100%; height: 100%; object-fit: cover; }
  .po span { display: grid; place-items: center; height: 100%; box-sizing: border-box; padding: 10px; font-size: 13px; font-weight: 700; text-align: center; }
  .po:focus-visible { outline: 2px solid #fff; outline-offset: 3px; }

  .favs { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 14px; margin: 0; padding: 0; list-style: none; }
  .recent { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 12px; margin: 0; padding: 0; list-style: none; }
  .under { display: flex; align-items: center; gap: 6px; min-height: 16px; margin-top: 8px; }
  .heart { display: inline-grid; color: #fff; }

  .revs { margin: 0; padding: 0; list-style: none; }
  .revs .po { width: 70px; }
  .revs li { display: grid; grid-template-columns: 70px minmax(0, 1fr); gap: 18px; padding: 18px 0; border-bottom: 1px solid #ffffff12; }
  .revs li:first-child { padding-top: 0; }
  .rt b { color: #fff; font-size: 18px; font-weight: 800; letter-spacing: -0.01em; }
  .rt span { color: #8c8c96; font-size: 14px; font-weight: 500; }
  .rt:hover b { text-decoration: underline; }
  .rm { display: flex; align-items: center; gap: 10px; margin-top: 4px; }
  .rm small { color: #8c8c96; font-size: 12px; }
  .revs p { margin: 10px 0 0; color: #c9c9cf; font-size: 14px; line-height: 1.6; overflow-wrap: anywhere; white-space: pre-line; display: -webkit-box; -webkit-line-clamp: 4; line-clamp: 4; -webkit-box-orient: vertical; overflow: hidden; }

  .now .po { width: 62px; }
  .now { display: grid; grid-template-columns: 62px minmax(0, 1fr); gap: 14px; align-items: center; }
  .nt { display: grid; gap: 3px; min-width: 0; }
  .nt b { overflow: hidden; color: #fff; font-size: 15px; text-overflow: ellipsis; white-space: nowrap; }
  .nt small { color: #8c8c96; font-size: 12px; }
  .nt em { display: inline-flex; align-items: center; gap: 4px; margin-top: 6px; font-size: 12px; font-style: normal; font-weight: 700; }
  .now:hover em { text-decoration: underline; }
  .bar { display: block; height: 3px; margin-top: 6px; overflow: hidden; border-radius: 2px; background: #ffffff1f; }
  .bar i { display: block; height: 100%; background: #fff; transform-origin: left; }

  .hist { display: flex; align-items: flex-end; gap: 3px; height: 68px; }
  .hist i { flex: 1; border-radius: 2px 2px 0 0; background: #4a4a52; }
  .hist i.top { background: #e9e9ec; }
  .hl { display: flex; justify-content: space-between; margin-top: 6px; color: #8c8c96; font-size: 11px; }

  .diary { margin: 0; padding: 0; list-style: none; }
  .diary li { display: grid; grid-template-columns: 34px 24px minmax(0, 1fr) auto; gap: 10px; align-items: center; padding: 9px 0; border-bottom: 1px solid #ffffff10; }
  .mo { color: #8c8c96; font-size: 10px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
  .dd { color: #fff; font-size: 15px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .diary a { overflow: hidden; color: #d4d4d8; font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .diary a:hover { color: #fff; }

  .dir { display: grid; gap: 2px; margin-bottom: 14px; color: #fff; font-size: 16px; font-weight: 700; }
  .dir small { color: #8c8c96; font-size: 10px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
  .dir:hover { text-decoration: underline; }
  .actors :global(.fa h2) { display: none; }
  .actors :global(p) { font-size: 13px; line-height: 1.5; }

  .ach { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px 6px; margin: 0; padding: 0; list-style: none; text-align: center; }
  .ach li { display: grid; justify-items: center; gap: 6px; color: #8c8c96; }
  .ach span { display: grid; place-items: center; width: 38px; height: 38px; border-radius: 50%; background: #ffffff0d; color: #ffffff4d; }
  .ach .got { color: #e4e4e7; }
  .ach .got span { background: #fff; color: #0a0a0b; }
  .ach small { font-size: 10.5px; line-height: 1.2; }
  .goal { margin: 12px 0 0; color: #b4b4bc; font-size: 12px; }

  .quick { display: grid; border-top: 1px solid #ffffff1a; }
  .quick a, .quick button { display: flex; align-items: center; gap: 10px; padding: 11px 2px; border: 0; border-bottom: 1px solid #ffffff10; background: none; color: #c4c4cc; font: inherit; font-size: 13.5px; text-align: left; cursor: pointer; }
  .quick a:hover, .quick button:hover { color: #fff; }
  .quick em { margin-left: auto; min-width: 20px; padding: 2px 6px; border-radius: 999px; background: #fff; color: #0a0a0b; font-size: 11px; font-style: normal; font-weight: 700; text-align: center; }
  .quick .out { color: #fca5a5; }
  .quick a:focus-visible, .quick button:focus-visible, .h2 a:focus-visible, .h2 button:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }

  @media (max-width: 1100px) {
    .dy { grid-template-columns: minmax(0, 1fr); gap: 40px; }
    .side { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 34px 32px; }
  }
  @media (max-width: 760px) {
    .recent { grid-template-columns: repeat(3, minmax(0, 1fr)); }
    .favs { gap: 10px; }
    .side { grid-template-columns: minmax(0, 1fr); }
  }
  /* Вся ширина окна (владелец, 21.11): на широком экране любимые фильмы и сериалы - рядом, «Недавно» - на весь ряд
     (лишние постеры прячутся, а не переносятся), отзывы - в две колонки. */
  .favrow { display: grid; gap: 40px; }
  .recent { grid-template-columns: repeat(auto-fill, minmax(118px, 1fr)); grid-template-rows: auto; grid-auto-rows: 0; row-gap: 0; overflow: hidden; margin-top: -8px; padding-top: 8px; }
  .recent li { min-height: 0; overflow: hidden; }
  @media (min-width: 1400px) {
    .dy { grid-template-columns: minmax(0, 1fr) 340px; gap: 64px; }
    .favrow.pair { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 40px; }
    .revs { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); column-gap: 40px; }
    .revs li:nth-child(2) { padding-top: 0; }
  }
</style>
