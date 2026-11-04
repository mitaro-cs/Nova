<script lang="ts">
  import { goto } from '$app/navigation';
  import { post } from '$lib/api';
  import { count, plural } from '$lib/fmt';
  import FriendButton from '$lib/social/FriendButton.svelte';
  import PosterWall from '$lib/social/PosterWall.svelte';
  import { badges as unseen } from '$lib/social/badges.svelte';
  import { session } from '$lib/session.svelte';
  import Avatar from '$lib/ui/Avatar.svelte';
  import { countUp } from '$lib/ui/countUp';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';
  import Medal from './Medal.svelte';
  import type { ProfilePage } from './types';

  // Боковая карточка человека (липкая): стена постеров, портрет, имя, «сейчас смотрит», цифры, значки, жанры,
  // кнопки по роли (я, друг, чужой) и быстрые разделы - у владельца. Закрытая статистика - без цифр и значков.
  let { p = $bindable() }: { p: ProfilePage } = $props();

  async function logout() {
    await post('/api/auth/logout').catch(() => {});
    session.me = null;
    goto('/login');
  }

  const s = $derived(p.stats);
  const hours = $derived(s ? (s.hours >= 10 ? Math.round(s.hours) : s.hours) : 0);
  const now = $derived(p.watching[0]);
  const nowTitle = $derived(now?.card?.title ?? now?.ext?.title ?? '');

  // Значки и цели: что уже получено, сколько осталось до ближайшего.
  const GOALS: { icon: IconName; label: string; need: number; of: (s: NonNullable<ProfilePage['stats']>) => number; word: [string, string, string]; hint: string }[] = [
    { icon: 'sparkle', label: 'Киноман', need: 50, of: (s) => s.hours, word: ['час', 'часа', 'часов'], hint: 'больше 50 часов просмотра' },
    { icon: 'tv', label: 'Марафонец', need: 3, of: (s) => s.series, word: ['сериал', 'сериала', 'сериалов'], hint: 'смотрел 3 сериала и больше' },
    { icon: 'edit', label: 'Критик', need: 5, of: (s) => s.reviews, word: ['отзыв', 'отзыва', 'отзывов'], hint: '5 отзывов и больше' },
    { icon: 'users', label: 'Душа компании', need: 5, of: (s) => s.friends, word: ['друг', 'друга', 'друзей'], hint: '5 друзей и больше' },
    { icon: 'globe', label: 'Исследователь', need: 10, of: (s) => s.outside, word: ['отметка', 'отметки', 'отметок'], hint: '10 отметок в Обзоре' },
  ];
  const earned = $derived(s ? GOALS.filter((g) => g.of(s) >= g.need) : []);
  const names = $derived(earned.length ? earned : s ? [{ icon: 'play' as IconName, label: 'Новичок', hint: 'всё только начинается' }] : []);
  const nextGoal = $derived.by(() => {
    if (!s || !p.me) return null;
    const open = GOALS.filter((g) => g.of(s) < g.need).sort((a, b) => b.of(s) / b.need - a.of(s) / a.need)[0];
    return open ? `до «${open.label}» - ${count(Math.ceil(open.need - open.of(s)), ...open.word)}` : null;
  });
  const topTitle = $derived(names[0]?.label ?? '');
</script>

<aside class="card" aria-label="Человек">
  <div class="cover" aria-hidden="true">
    {#if p.decor.coverKind === 'image' && p.decor.cover}
      <img use:fadeIn src={p.decor.cover} alt="" decoding="async" />
    {:else}
      <PosterWall posters={p.decor.ribbon} accent={p.decor.accent} />
    {/if}
  </div>

  <div class="who" style:--acc={p.decor.accent ?? '#4a4a53'}>
    <div class="ring">
      <Avatar avatar={p.person.avatar} photo={p.person.photo} size={108} />
      {#if now}<i class="live" title="Сейчас смотрит"></i>{/if}
    </div>
    <h1>{p.person.name}</h1>
    {#if now}<p class="now"><Icon name="play" size={12} /> Сейчас смотрит · {nowTitle}</p>{/if}

    {#if s}
      <ul class="caps" aria-label="Цифры">
        <li style:--i={0}><Icon name="clock" size={14} /> <span use:countUp={{ value: hours, decimals: hours < 10 && hours % 1 ? 1 : 0 }}>0</span> {plural(Math.floor(s.hours), 'час', 'часа', 'часов')}</li>
        <li style:--i={1}><Icon name="film" size={14} /> {count(s.movies, 'фильм', 'фильма', 'фильмов')}</li>
        <li style:--i={2}><Icon name="tv" size={14} /> {count(s.episodes, 'серия', 'серии', 'серий')}</li>
      </ul>
      <ul class="bdg" aria-label="Значки">
        {#each names as b, i (b.label)}<li style:--i={i} title={b.hint}><Icon name={b.icon} size={13} /> {b.label}</li>{/each}
      </ul>
    {:else}
      <p class="locked"><Icon name="lock" size={14} /> Цифры скрыты</p>
    {/if}

    {#if p.decor.bio}<p class="bio">{p.decor.bio}</p>{/if}
    {#if s?.genres.length}
      <p class="genres">{#each s.genres.slice(0, 4) as g (g.genre)}<a href="/search?q={encodeURIComponent(g.genre)}">{g.genre}</a>{/each}</p>
    {/if}

    {#if s}
      <div class="box">
        <div><b use:countUp={{ value: s.watched ?? 0 }}>0</b><small>просмотров</small></div>
        <div><b use:countUp={{ value: s.reviews }}>0</b><small>отзывов</small></div>
        <div><b use:countUp={{ value: s.friends }}>0</b><small>друзей</small></div>
      </div>
    {/if}

    <div class="btns">
      {#if p.me}
        <a class="btn primary" href="/profile"><Icon name="edit" size={14} /> Оформить профиль</a>
        <a class="btn" href="/people"><Icon name="users" size={14} /> Найти друзей</a>
      {:else if p.relation}
        <FriendButton id={p.person.id} bind:relation={p.relation} />
        {#if p.relation === 'friend'}<a class="btn" href="/chats/{p.person.id}"><Icon name="party" size={14} /> Позвать смотреть вместе</a>{/if}
      {/if}
    </div>

    {#if s && names.length}
      <div class="meds">
        <span class="m">{#each names.slice(0, 5) as b, i (b.label)}<Medal icon={b.icon} tone={(i % 3) as 0 | 1 | 2} label={b.label} {i} />{/each}</span>
        {#if nextGoal}<small>{nextGoal}</small>{:else if topTitle}<small class="sr">{topTitle}</small>{/if}
      </div>
    {/if}

    {#if p.me}
      <nav class="quick" aria-label="Быстрые разделы">
        <a href="/chats"><span><Icon name="mail" size={17} /> Чаты</span>{#if unseen.unread}<em>{unseen.unread}</em>{/if}</a>
        <a href="/lists"><span><Icon name="list" size={17} /> Подборки</span><Icon name="right" size={16} /></a>
        {#if session.me?.manage}<a href="/admin"><span><Icon name="sliders" size={17} /> Управление</span><Icon name="right" size={16} /></a>{/if}
        <button type="button" onclick={() => window.dispatchEvent(new Event('nova-room-link'))}><span><Icon name="party" size={17} /> Комната по ссылке</span><Icon name="right" size={16} /></button>
        <button type="button" onclick={() => window.dispatchEvent(new Event('nova-tour'))}><span><Icon name="sparkle" size={17} /> Экскурсия по Nova</span><Icon name="right" size={16} /></button>
        {#if !session.me?.window}<button type="button" onclick={logout}><span><Icon name="logout" size={17} /> Выйти</span></button>{/if}
      </nav>
    {/if}
  </div>
</aside>

<style>
  .card {
    position: sticky;
    top: calc(var(--header) + 14px);
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: 20px;
    background: var(--surface);
    animation: rise 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  .cover {
    position: relative;
    height: 116px;
    overflow: hidden;
    background: #101013;
  }
  .cover :global(.wall) {
    position: absolute;
    inset: 0;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, #17171a00 10%, var(--surface) 100%);
  }
  .who {
    display: grid;
    justify-items: center;
    gap: 0;
    padding: 0 18px 18px;
    text-align: center;
  }
  /* Портрет: спокойный ободок цвета профиля, без движения. */
  .ring {
    position: relative;
    display: grid;
    place-items: center;
    width: 108px;
    height: 108px;
    margin-top: -54px;
    border-radius: 50%;
    background: #d9d9df;
    box-shadow:
      0 0 0 4px var(--surface),
      0 0 0 6px color-mix(in srgb, var(--acc) 55%, #fff);
  }
  .ring :global(.ava) {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
  }
  .live {
    position: absolute;
    right: 2px;
    bottom: 4px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--ok);
    border: 4px solid var(--surface);
  }
  h1 {
    margin: 14px 0 0;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.02em;
    overflow-wrap: anywhere;
  }
  .now {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 6px 0 0;
    color: #dcdcdf;
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }
  .caps,
  .bdg {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 6px;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
  }
  .caps li {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 11px;
    border-radius: 999px;
    background: #00000066;
    box-shadow: inset 0 0 0 1px #ffffff22;
    font-size: 12px;
    font-weight: 600;
    animation: rise 0.5s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--i) * 70ms + 250ms);
  }
  .bdg li {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: 8px;
    background: linear-gradient(135deg, color-mix(in srgb, var(--acc) 50%, #2a2a2e), #1c1c1f);
    box-shadow: inset 0 0 0 1px #ffffff26;
    font-size: 11.5px;
    font-weight: 600;
    animation: badge 0.55s cubic-bezier(0.2, 0.9, 0.3, 1.4) both;
    animation-delay: calc(var(--i) * 90ms + 450ms);
  }
  .locked {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 14px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .bio {
    margin: 12px 0 0;
    color: #dcdcdf;
    font-size: 13.5px;
    white-space: pre-line;
    overflow-wrap: anywhere;
  }
  .genres {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 5px;
    margin: 10px 0 0;
  }
  .genres a {
    padding: 4px 11px;
    border-radius: 999px;
    background: #ffffff1a;
    font-size: 12px;
    font-weight: 500;
    transition: background 0.2s;
  }
  .genres a:hover {
    background: #ffffff30;
  }
  .box {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    width: 100%;
    margin-top: 12px;
    padding: 12px 6px;
    border: 1px solid var(--line);
    border-radius: 16px;
  }
  .box div + div {
    border-left: 1px solid var(--line);
  }
  .box b {
    display: block;
    font-size: 22px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .box small {
    color: var(--muted);
    font-size: 11.5px;
  }
  .btns {
    display: grid;
    gap: 8px;
    width: 100%;
    margin-top: 12px;
  }
  .btns :global(.fb) {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }
  .btns :global(.btn),
  .btns > .btn {
    justify-content: center;
    width: 100%;
    border-radius: 999px;
  }
  .meds {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    margin-top: 16px;
    text-align: left;
  }
  .m {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .meds small {
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.3;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .quick {
    display: grid;
    width: 100%;
    margin-top: 12px;
    text-align: left;
  }
  .quick a,
  .quick button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 11px 2px;
    border: 0;
    border-top: 1px solid var(--line);
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 14px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
    transition: padding 0.25s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .quick a:hover,
  .quick button:hover {
    padding-left: 8px;
  }
  .quick span {
    display: inline-flex;
    align-items: center;
    gap: 9px;
  }
  .quick em {
    color: var(--muted);
    font-style: normal;
    font-weight: 500;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate3d(0, 18px, 0);
    }
  }
  @keyframes badge {
    from {
      opacity: 0;
      transform: scale(0.5) rotate(-8deg);
    }
  }
  @media (max-width: 900px) {
    .card {
      position: relative;
      top: 0;
    }
  }
</style>
