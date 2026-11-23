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

  const director = $derived(p.decor.directorsInfo?.[0] ?? (p.decor.directors[0] ? { name: p.decor.directors[0], photo: null, seen: null } : null));

  type Quick = { href?: string; act?: () => void; icon: IconName; label: string; hint: string; count?: number; out?: boolean };
  async function logout() {
    await post('/api/auth/logout').catch(() => {});
    session.me = null;
    goto('/login');
  }
  const quick = $derived.by<Quick[]>(() => [
    { href: '/profile', icon: 'pencil', label: 'Оформить', hint: 'Обложка, любимые, о себе' },
    { href: '/chats', icon: 'mail', label: 'Чаты', hint: 'Переписка с друзьями', count: unseen.unread },
    { href: '/people', icon: 'users', label: 'Друзья', hint: 'Найти и позвать', count: unseen.requests },
    { href: '/lists', icon: 'grid', label: 'Подборки', hint: 'Общие списки фильмов' },
    { act: () => window.dispatchEvent(new Event('nova-room-link')), icon: 'party', label: 'Создать комнату', hint: 'Смотреть вместе' },
    ...(session.me?.manage ? [{ href: '/admin', icon: 'sliders' as IconName, label: 'Управление', hint: 'Карта дома и настройки' }] : []),
    { act: () => window.dispatchEvent(new Event('nova-tour')), icon: 'sparkle', label: 'Экскурсия', hint: 'Показать, что где' },
    ...(!session.me?.window ? [{ act: logout, icon: 'logout' as IconName, label: 'Выйти', hint: 'Из этого браузера', out: true }] : []),
  ]);
</script>

<div class="ks">
  {#if goals.length}
    <section class="medals" aria-labelledby="ks-b">
      <div class="h2"><h2 id="ks-b">Значки</h2><small>{goals.filter((g) => g.got).length} из {goals.length}{nextGoal ? ` · ${nextGoal}` : ''}</small></div>
      <ul>
        {#each goals as g, i (g.label)}
          <li class:got={g.got} class="rise" style:--i={i} title={g.got ? g.label : `${g.label}: ${g.hint}`}>
            <Badge icon={g.icon} tone={i} got={g.got} progress={g.progress} />
            <b>{g.label}</b><small>{g.hint}</small>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if tickets.length}
    <section aria-labelledby="ks-t">
      <div class="h2"><h2 id="ks-t">{recentTitle}</h2>{#if recent.length}<a href="/people/{p.person.id}/watched?kind=movie">Все</a>{/if}</div>
      <div class="fanwrap">
        <ul class="fan" style:--n={tickets.length}>
          {#each tickets as t, i (t.key)}
            <li style:--r="{(i - (tickets.length - 1) / 2) * 4}deg" style:--y="{Math.abs(i - (tickets.length - 1) / 2) * 8}px">
              <a class="tk" class:now={t.now} href={t.href}>
                <span class="tp" style:background={placeholder(t.title)}>{#if t.poster}<img use:fadeIn src={artW(t.poster, 200)} alt="" loading="lazy" decoding="async" width="66" height="99" />{/if}</span>
                <span class="tt"><small>{t.label}</small><b>{t.title}</b><em>{t.note}{t.stars ? ` · ★ ${(t.stars / 2).toFixed(1).replace('.0', '').replace('.', ',')}` : ''}</em></span>
              </a>
            </li>
          {/each}
        </ul>
      </div>
    </section>
  {/if}

  {#each [{ list: movies, label: 'Любимые фильмы', id: 'ks-m' }, { list: series, label: 'Любимые сериалы', id: 'ks-s' }] as g (g.id)}
    {#if g.list.length || (p.me && g.id === 'ks-m')}
      <section aria-labelledby={g.id}>
        <div class="h2"><h2 id={g.id}>{g.label}</h2>{#if p.me}<a href="/profile">{g.list.length ? 'Изменить' : 'Выбрать'}</a>{/if}</div>
        {#if g.list.length}
          <ul class="boxes">
            {#each g.list as f, i (f.key)}
              <li class="rise" style:--i={i}>
                <a class="lb" href={f.href} title="{f.title}{f.year ? `, ${f.year}` : ''}">
                  <span class="po" style:background={placeholder(f.title)}>{#if f.poster}<img use:fadeIn src={artW(f.poster, 500)} alt={f.title} loading="lazy" decoding="async" width="342" height="513" />{:else}<span>{f.title}</span>{/if}</span>
                </a>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted sm">Любимые фильмы - афиша вашего профиля. Выберите их в оформлении.</p>
        {/if}
      </section>
    {/if}
  {/each}

  {#if director || p.decor.actors?.length || p.me}
    <section aria-labelledby="ks-p">
      <div class="h2"><h2 id="ks-p">Любимые люди</h2></div>
      <div class="people">
        {#if director}
          <a class="dir" href="/person?name={encodeURIComponent(director.name)}&role=director">
            <span class="ph">{#if director.photo}<img use:fadeIn src={director.photo} alt="" loading="lazy" width="88" height="88" />{:else}<Icon name="film" size={26} />{/if}</span>
            <span><small>Любимый режиссёр</small><b>{director.name}</b>{#if director.seen}<em>просмотрено {director.seen.seen} из {director.seen.total}{director.seen.avg != null ? ` · в среднем ${director.seen.avg.toFixed(1).replace('.', ',')}` : ''}</em>{/if}</span>
          </a>
        {/if}
        <div class="actors"><FavActors bind:actors={p.decor.actors} suggest={(p.stats?.actors ?? []).map((a) => a.name)} editable={p.me} /></div>
      </div>
    </section>
  {/if}

  {#if p.me}
    <nav class="quick" aria-label="Быстрые разделы">
      {#each quick as q (q.label)}
        {#if q.href}
          <a href={q.href}><span class="qi"><Icon name={q.icon} size={18} /></span><span><b>{q.label}{#if q.count}<em>{q.count}</em>{/if}</b><small>{q.hint}</small></span></a>
        {:else}
          <button type="button" class:out={q.out} onclick={q.act}><span class="qi"><Icon name={q.icon} size={18} /></span><span><b>{q.label}</b><small>{q.hint}</small></span></button>
        {/if}
      {/each}
    </nav>
  {/if}

  {#if !tickets.length && !movies.length && !series.length && !p.me}<p class="muted none">Пока здесь пусто: человек ещё ничего не посмотрел.</p>{/if}
  {#if p.reviews.length}<p class="more"><button type="button" class="btn sm" onclick={toReviews}><Icon name="edit" size={14} /> Отзывы · {p.stats?.reviews ?? p.reviews.length}</button></p>{/if}
</div>

<style>
  .ks { display: grid; gap: 44px; min-width: 0; }
  section { min-width: 0; }
  .h2 { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 18px; padding-bottom: 10px; border-bottom: 1px solid #ffffff1a; }
  .h2 h2 { margin: 0; color: #8c8c96; font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; }
  .h2 a { color: #8c8c96; font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; text-decoration: none; }
  .h2 a:hover { color: #fff; }
  .h2 small { color: #8c8c96; font-size: 12px; text-align: right; }
  .sm { margin: 0; font-size: 13px; }
  .none { padding: 30px 0; text-align: center; }

  /* Значки - друг за другом в один ряд (на узком - прокрутка вбок). */
  .medals ul { display: flex; gap: 10px; margin: 0; padding: 4px 0 8px; list-style: none; overflow-x: auto; scrollbar-width: none; }
  .medals li { flex: 1 0 120px; display: grid; justify-items: center; gap: 6px; text-align: center; padding: 14px 8px; border-radius: 18px; }
  .medals li:hover :global(.badge) { transform: rotate(-12deg) scale(1.08); }
  .medals b { font-size: 13px; }
  .medals li:not(.got) b { color: #8c8c96; }
  .medals small { color: #8c8c96; font-size: 11.5px; line-height: 1.3; }

  /* Веер билетов. */
  .fanwrap { overflow-x: auto; scrollbar-width: none; padding: 18px 0 14px; }
  .fan { display: flex; justify-content: center; margin: 0; padding: 0 20px; list-style: none; min-width: min-content; }
  .fan li { flex: none; margin: 0 -10px; transform: rotate(var(--r)) translateY(var(--y)); transition: transform 0.3s var(--ease, ease); }
  .fan li:hover { transform: rotate(0deg) translateY(-8px); z-index: 2; }
  .tk { position: relative; display: grid; grid-template-columns: 66px minmax(0, 1fr); gap: 12px; align-items: center; width: 236px; padding: 10px; border-radius: 16px; background: #f4f4f6; color: #111; text-decoration: none; box-shadow: 0 22px 40px -18px #000; }
  .tk::before, .tk::after { content: ''; position: absolute; left: 82px; width: 14px; height: 14px; border-radius: 50%; background: #0b0b0d; }
  .tk::before { top: -7px; } .tk::after { bottom: -7px; }
  .tk.now { background: #fff; box-shadow: 0 0 0 2px #fff, 0 22px 40px -18px #000; }
  .tp { width: 66px; aspect-ratio: 2 / 3; border-radius: 7px; overflow: hidden; }
  .tp img { width: 100%; height: 100%; object-fit: cover; }
  .tt { display: grid; gap: 2px; min-width: 0; padding-left: 6px; }
  .tt small { color: #0009; font-size: 10px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
  .tk.now .tt small { color: #111; }
  .tt b { font-size: 15px; line-height: 1.3; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tt em { color: #000a; font-size: 12px; font-style: normal; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  /* Любимые - световые короба, как на фасаде кинотеатра. */
  .boxes { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 16px; margin: 0; padding: 0; list-style: none; }
  .lb { display: block; padding: 8px; border-radius: 14px; background: #1a1a1e; box-shadow: inset 0 0 0 1px #ffffff1f, 0 0 34px -10px #ffffff40; transition: transform 0.3s var(--ease, ease), box-shadow 0.3s; }
  .lb:hover { transform: translateY(-4px); }
  .po { display: grid; place-items: center; aspect-ratio: 2 / 3; border-radius: 7px; overflow: hidden; color: #fff; font-size: 13px; font-weight: 700; text-align: center; }
  .po img { width: 100%; height: 100%; object-fit: cover; }

  .people { display: grid; grid-template-columns: minmax(0, 340px) minmax(0, 1fr); gap: 24px; align-items: start; }
  .dir { display: flex; align-items: center; gap: 16px; padding: 14px; border-radius: 18px; background: #151518; color: #fff; text-decoration: none; }
  .dir:hover { background: #1c1c20; }
  .dir .ph { width: 72px; height: 72px; flex: none; border-radius: 50%; overflow: hidden; display: grid; place-items: center; background: #ffffff12; color: #8c8c96; }
  .dir .ph img { width: 100%; height: 100%; object-fit: cover; }
  .dir small { display: block; color: #8c8c96; font-size: 11px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
  .dir b { display: block; font-size: 18px; margin-top: 2px; }
  .dir em { display: block; color: #a1a1aa; font-size: 12.5px; font-style: normal; margin-top: 2px; }
  .actors { min-width: 0; }

  .quick { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
  .quick a, .quick button { display: flex; align-items: center; gap: 12px; padding: 14px; border: 0; border-radius: 16px; background: #151518; color: #fff; font: inherit; text-align: left; text-decoration: none; cursor: pointer; transition: background 0.2s; }
  .quick a:hover, .quick button:hover { background: #1f1f24; }
  .qi { width: 38px; height: 38px; flex: none; display: grid; place-items: center; border-radius: 12px; background: #ffffff14; }
  .quick b { display: flex; align-items: center; gap: 6px; font-size: 14px; }
  .quick b em { padding: 1px 7px; border-radius: 999px; background: #fff; color: #000; font-size: 11px; font-style: normal; }
  .quick small { display: block; color: #8c8c96; font-size: 12px; margin-top: 1px; }
  .quick .out { color: #ffb4b4; }
  .more { margin: 0; }

  @media (max-width: 1000px) {
    .boxes { grid-template-columns: repeat(4, minmax(0, 1fr)); }
    .quick { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .people { grid-template-columns: minmax(0, 1fr); }
  }
  @media (max-width: 640px) {
    .boxes { grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
    .lb { padding: 5px; border-radius: 10px; }
    .fan { justify-content: flex-start; }
    .tk { width: 210px; }
    .medals li { flex-basis: 96px; padding-inline: 4px; }
    .quick { grid-template-columns: minmax(0, 1fr); }
  }
</style>
