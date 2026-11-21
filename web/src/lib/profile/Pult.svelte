<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, post } from '$lib/api';
  import FriendButton from '$lib/social/FriendButton.svelte';
  import PosterWall from '$lib/social/PosterWall.svelte';
  import { badges as unseen } from '$lib/social/badges.svelte';
  import { session } from '$lib/session.svelte';
  import type { NowItem } from '$lib/types';
  import Avatar from '$lib/ui/Avatar.svelte';
  import { plural } from '$lib/fmt';
  import { countUp } from '$lib/ui/countUp';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import { GOALS, levelOf, nextGoalOf, rankOf } from './level';
  import type { ProfilePage } from './types';

  // «Пульт» (компьютер, по макету владельца в Figma 21.11): стена постеров, портрет, уровень, цифры, кнопки, достижения,
  // «О себе», плитки разделов и что смотрят друзья. Колонка липкая и прокручивается сама.
  let { p = $bindable() }: { p: ProfilePage } = $props();

  const s = $derived(p.stats);
  const now = $derived(p.watching[0]);
  const nowTitle = $derived(now?.card?.title ?? now?.ext?.title ?? '');
  const lv = $derived(levelOf(s));
  const goals = $derived(s ? GOALS.map((g) => ({ ...g, got: g.of(s) >= g.need })) : []);
  const got = $derived(goals.filter((g) => g.got).length);
  const nextGoal = $derived(p.me ? nextGoalOf(s) : null);
  const hours = $derived(s ? (s.hours >= 10 ? Math.round(s.hours) : Math.round(s.hours * 10) / 10) : 0);
  const nums = $derived(
    s
      ? [
          { v: hours, d: hours % 1 ? 1 : 0, l: hours % 1 ? 'часа' : plural(hours, 'час', 'часа', 'часов') },
          { v: s.movies, d: 0, l: plural(s.movies, 'фильм', 'фильма', 'фильмов') },
          { v: s.episodes, d: 0, l: plural(s.episodes, 'серия', 'серии', 'серий') },
          { v: s.watched ?? 0, d: 0, l: plural(s.watched ?? 0, 'просмотр', 'просмотра', 'просмотров') },
          { v: s.reviews, d: 0, l: plural(s.reviews, 'отзыв', 'отзыва', 'отзывов') },
          { v: s.friends, d: 0, l: plural(s.friends, 'друг', 'друга', 'друзей') },
        ]
      : [],
  );

  // Стена постеров на паузе - по кнопке; движение остаётся выключенным до следующего входа.
  let paused = $state(false);

  // Что смотрят друзья: первый фильм и сколько ещё друзей с ним.
  let friendsNow = $state<NowItem[]>([]);
  $effect(() => {
    if (!p.me) return;
    let live = true;
    api<NowItem[]>('/api/friends/activity').then((r) => live && (friendsNow = r.filter((a) => a.person && (a.card || a.ext))), () => {});
    return () => (live = false);
  });
  const together = $derived.by(() => {
    const first = friendsNow[0];
    if (!first) return null;
    const key = (n: NowItem) => (n.card ? `t${n.card.id}` : `e${n.ext?.id}`);
    const same = new Set(friendsNow.filter((n) => key(n) === key(first)).map((n) => n.person!.id));
    return {
      who: first.person!.name,
      more: same.size - 1,
      title: first.card?.title ?? first.ext?.title ?? '',
      href: first.card ? `/title/${first.card.id}` : `/ext/${first.ext!.id}`,
    };
  });

  let shared = $state(false);
  async function share() {
    try {
      await navigator.clipboard.writeText(location.origin + `/people/${p.person.id}`);
      shared = true;
      setTimeout(() => (shared = false), 1800);
    } catch {
      /* без буфера обмена - ничего */
    }
  }
  async function logout() {
    await post('/api/auth/logout').catch(() => {});
    session.me = null;
    goto('/login');
  }
  const friendsWord = (n: number) => (n % 10 === 1 && n % 100 !== 11 ? 'друг' : 'друга');
</script>

<aside class="pult" aria-label="Человек">
  <div class="cover" class:paused aria-hidden={p.decor.coverKind === 'image'}>
    {#if p.decor.coverKind === 'image' && p.decor.cover}
      <img use:fadeIn src={p.decor.cover} alt="" decoding="async" />
    {:else}
      <PosterWall posters={p.decor.ribbon} accent={p.decor.accent} prominent />
      <span class="tag">Из просмотренного</span>
      <button class="pause" type="button" aria-label={paused ? 'Запустить постеры' : 'Остановить постеры'} aria-pressed={paused} onclick={() => (paused = !paused)}>
        <Icon name={paused ? 'play' : 'pause'} size={14} />
      </button>
    {/if}
  </div>

  <div class="body">
    <div class="face">
      <Avatar avatar={p.person.avatar} photo={p.person.photo} size={84} />
      {#if now}<i class="live" title="Сейчас смотрит"></i>{/if}
    </div>
    <p class="eyebrow">{p.me ? 'Мой профиль' : 'Профиль'}</p>
    <h1>{p.person.name}</h1>
    {#if now}<p class="now">Сейчас смотрит · {nowTitle}</p>{/if}

    {#if s}
      <span class="chip">Уровень {lv.level} · {rankOf(lv.level)}</span>
      <p class="to">До {lv.level + 1} уровня - {Math.round(lv.toNext * 100)}%</p>
      <div class="bar" role="progressbar" aria-label="До следующего уровня" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(lv.toNext * 100)}>
        <i style:transform="scaleX({lv.toNext})"></i>
      </div>
      <ul class="nums" aria-label="Цифры">
        {#each nums as n (n.l)}<li><b use:countUp={{ value: n.v, decimals: n.d }}>0</b><small>{n.l}</small></li>{/each}
      </ul>
    {:else}
      <p class="locked"><Icon name="lock" size={14} /> Цифры скрыты владельцем</p>
    {/if}

    <div class="acts">
      {#if p.me}
        <a class="btn primary wide" href="/profile"><Icon name="pencil" size={15} /> Оформить профиль</a>
        <div class="row">
          <a class="btn" href="/people"><Icon name="users" size={15} /> Найти друзей</a>
          <button class="btn sq" type="button" onclick={share} aria-label={shared ? 'Ссылка скопирована' : 'Поделиться профилем'} title={shared ? 'Ссылка скопирована' : 'Поделиться профилем'}>
            <Icon name={shared ? 'check' : 'share'} size={16} />
          </button>
        </div>
      {:else if p.relation}
        <FriendButton id={p.person.id} bind:relation={p.relation} />
        {#if p.relation === 'friend'}<a class="btn wide" href="/chats/{p.person.id}"><Icon name="party" size={15} /> Позвать смотреть вместе</a>{/if}
      {/if}
    </div>

    {#if s}
      <section class="part" aria-labelledby="pult-ach">
        <div class="ph"><h2 id="pult-ach">Достижения</h2><small>{got} из {goals.length}</small></div>
        <ul class="ach">
          {#each goals as g, i (g.label)}
            <li class:got={g.got} style:--i={i} title={g.got ? g.label : `${g.label}: ${g.hint}`}>
              <span class="ic"><Icon name={g.icon} size={18} /></span><small>{g.label}</small>
            </li>
          {/each}
        </ul>
        {#if nextGoal}<p class="goal">{nextGoal}</p>{/if}
      </section>
    {/if}

    {#if p.decor.bio || s?.genres.length}
      <section class="part" aria-labelledby="pult-bio">
        <div class="ph"><h2 id="pult-bio">О себе</h2></div>
        {#if p.decor.bio}<p class="bio selectable">{p.decor.bio}</p>{/if}
        {#if s?.genres.length}
          <p class="tags">{#each s.genres.slice(0, 3) as g (g.genre)}<a href="/search?q={encodeURIComponent(g.genre)}">{g.genre}</a>{/each}</p>
        {/if}
      </section>
    {/if}

    {#if p.me}
      <section class="part" aria-labelledby="pult-nav">
        <div class="ph"><h2 id="pult-nav">Пульт</h2></div>
        <div class="tiles">
          <a href="/chats"><span><Icon name="mail" size={18} />{#if unseen.unread}<em>{unseen.unread}</em>{/if}</span>Чаты</a>
          <a href="/people"><span><Icon name="users" size={18} />{#if unseen.requests}<em>{unseen.requests}</em>{:else if s?.friends}<em class="soft">{s.friends}</em>{/if}</span>Друзья</a>
          <a href="/lists"><span><Icon name="grid" size={18} /></span>Подборки</a>
          <a href="/profile"><span><Icon name="image" size={18} /></span>Оформление</a>
          <button type="button" onclick={() => window.dispatchEvent(new Event('nova-room-link'))}><span><Icon name="play" size={18} /></span>Смотреть вместе</button>
          <a href="/profile?tab=account"><span><Icon name="sliders" size={18} /></span>Настройки</a>
        </div>
        {#if together}
          <a class="friends" href={together.href}>
            <span class="who"><i></i>{together.who}{#if together.more > 0} и ещё {together.more} {friendsWord(together.more)}{/if} {together.more > 0 ? 'смотрят' : 'смотрит'}</span>
            <span class="what">«{together.title}» · Присоединиться <Icon name="right" size={13} /></span>
          </a>
        {/if}
        <nav class="more" aria-label="Ещё">
          {#if session.me?.manage}<a href="/admin">Управление <small>{session.me?.user.role === 'host' ? 'Вы хост' : ''}</small></a>{/if}
          <button type="button" onclick={() => window.dispatchEvent(new Event('nova-room-link'))}>Комната по ссылке</button>
          <button type="button" onclick={() => window.dispatchEvent(new Event('nova-tour'))}>Экскурсия</button>
          {#if !session.me?.window}<button type="button" class="out" onclick={logout}>Выйти</button>{/if}
        </nav>
      </section>
    {/if}
  </div>
</aside>

<style>
  .pult { overflow: hidden; border: 1px solid #ffffff1a; border-radius: 28px; background: #101012; }
  .cover { position: relative; height: 190px; overflow: hidden; background: #17171a; }
  .cover img { width: 100%; height: 100%; object-fit: cover; }
  .cover :global(.wall) { position: absolute; inset: 0; }
  .cover.paused :global(*) { animation-play-state: paused !important; }
  .cover::after { content: ''; position: absolute; inset: 0; background: linear-gradient(180deg, #10101200 40%, #101012 100%); pointer-events: none; }
  .tag { position: absolute; z-index: 1; top: 14px; left: 14px; padding: 6px 12px; border-radius: 999px; background: #000000b8; font-size: 11.5px; font-weight: 600; }
  .pause { position: absolute; z-index: 1; top: 12px; right: 12px; display: grid; place-items: center; width: 32px; height: 32px; padding: 0; border: 0; border-radius: 50%; background: #000000b8; color: #fff; cursor: pointer; }
  .pause:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }

  .body { position: relative; padding: 0 20px 22px; }
  .face { position: relative; width: 92px; height: 92px; margin-top: -60px; padding: 3px; border: 1px solid #ffffff59; border-radius: 50%; background: #101012; }
  .face :global(img), .face :global(.avatar) { border-radius: 50%; }
  .live { position: absolute; right: 4px; bottom: 6px; width: 14px; height: 14px; border: 3px solid #101012; border-radius: 50%; background: #22c55e; }
  .eyebrow { margin: 14px 0 0; color: var(--muted); font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
  h1 { margin: 4px 0 0; font-size: 28px; font-weight: 800; letter-spacing: -0.03em; line-height: 1.1; overflow-wrap: anywhere; }
  .now { margin: 6px 0 0; color: #d4d4d8; font-size: 13px; }
  .chip { display: inline-block; margin-top: 14px; padding: 6px 12px; border-radius: 999px; background: #ffffff14; font-size: 12px; font-weight: 600; }
  .to { margin: 14px 0 8px; color: var(--muted); font-size: 12px; }
  .bar { height: 4px; overflow: hidden; border-radius: 2px; background: #ffffff1a; }
  .bar i { display: block; height: 100%; background: #fff; transform-origin: left; transition: transform 0.8s var(--ease, ease); }
  .nums { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 18px 10px; margin: 20px 0 0; padding: 0; list-style: none; }
  .nums b { display: block; font-size: 22px; font-weight: 700; letter-spacing: -0.02em; line-height: 1.1; font-variant-numeric: tabular-nums; }
  .nums small { color: var(--muted); font-size: 11.5px; }
  .locked { display: flex; align-items: center; gap: 8px; margin: 14px 0 0; color: var(--muted); font-size: 13px; }

  .acts { display: grid; gap: 10px; margin-top: 22px; }
  .acts .btn { min-height: 44px; justify-content: center; }
  .acts .row { display: grid; grid-template-columns: minmax(0, 1fr) 44px; gap: 10px; }
  .acts .sq { padding: 0; }
  .acts :global(.btn) { width: 100%; }

  .part { margin-top: 22px; padding-top: 20px; border-top: 1px solid #ffffff14; }
  .ph { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 14px; }
  .ph h2 { margin: 0; font-size: 15px; font-weight: 700; }
  .ph small { color: var(--muted); font-size: 12px; }
  .ach { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px 6px; margin: 0; padding: 0; list-style: none; text-align: center; }
  .ach li { display: grid; justify-items: center; gap: 6px; color: var(--muted); }
  .ach .ic { display: grid; place-items: center; width: 42px; height: 42px; border-radius: 14px; background: #ffffff0d; color: #ffffff4d; }
  .ach .got { color: #e4e4e7; }
  .ach .got .ic { background: #fff; color: #0a0a0b; }
  .ach small { font-size: 10.5px; line-height: 1.2; }
  .goal { margin: 14px 0 0; color: #d4d4d8; font-size: 12px; }
  .bio { margin: 0; color: #c4c4cc; font-size: 13px; line-height: 1.5; overflow-wrap: anywhere; }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; margin: 12px 0 0; }
  .tags a { padding: 6px 12px; border-radius: 999px; background: #ffffff12; font-size: 12px; font-weight: 600; }
  .tags a:hover { background: #ffffff21; }

  .tiles { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
  .tiles a, .tiles button { display: grid; gap: 14px; padding: 14px; border: 0; border-radius: 16px; background: #ffffff0a; color: #e4e4e7; font: inherit; font-size: 12.5px; font-weight: 600; text-align: left; cursor: pointer; transition: background 0.2s; }
  .tiles a:hover, .tiles button:hover { background: #ffffff14; }
  .tiles span { display: flex; align-items: center; justify-content: space-between; color: #fff; }
  .tiles em { min-width: 20px; padding: 2px 6px; border-radius: 999px; background: #fff; color: #0a0a0b; font-size: 11px; font-style: normal; font-weight: 700; text-align: center; }
  .tiles em.soft { background: none; color: var(--muted); }
  .friends { display: grid; gap: 6px; margin-top: 10px; padding: 14px; border-radius: 16px; background: #ffffff0a; font-size: 12px; }
  .friends:hover { background: #ffffff14; }
  .friends .who { display: flex; align-items: center; gap: 8px; color: #e4e4e7; }
  .friends .who i { flex: none; width: 7px; height: 7px; border-radius: 50%; background: #22c55e; }
  .friends .what { display: flex; align-items: center; gap: 4px; color: #fff; font-weight: 600; }
  .more { display: grid; margin-top: 14px; }
  .more a, .more button { display: flex; justify-content: space-between; align-items: center; padding: 10px 2px; border: 0; border-top: 1px solid #ffffff0d; background: none; color: #c4c4cc; font: inherit; font-size: 13px; text-align: left; cursor: pointer; }
  .more a:hover, .more button:hover { color: #fff; }
  .more small { color: var(--muted); font-size: 11.5px; }
  .more .out { color: #fca5a5; }
  .tiles a:focus-visible, .tiles button:focus-visible, .more a:focus-visible, .more button:focus-visible, .friends:focus-visible, .tags a:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }
</style>
