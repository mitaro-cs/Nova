<script lang="ts">
  import { untrack, type Component } from 'svelte';
  import { goto } from '$app/navigation';
  import { page, navigating } from '$app/state';
  import { tabOf } from '$lib/library';
  import { session } from '$lib/session.svelte';
  import { badges, watchBadges } from '$lib/social/badges.svelte';
  import { scrolledBy } from '$lib/scroll';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import Logo from './Logo.svelte';

  // Шапка: слева знак, справа колокольчик (просьбы дружбы, сообщения, просьбы зрителей) и у хоста «добавить».
  // Разделы - одна панель из пяти вкладок внизу на телефоне, планшете и компьютере (на телевизоре такая же,
  // `web/tv`): Главное, Что смотреть, Профиль, Коллекция, Поиск. Люди, Чаты, Подборки и остальное - на странице
  // «Профиль».
  let scrolled = $state(false);
  let menu = $state(false);
  // Лист колокольчика грузится по нажатию: бюджет JS главной.
  let Menu = $state<Component<any> | null>(null);
  $effect(() => {
    if (menu && !Menu) void import('./HeaderMenu.svelte').then((m) => (Menu = m.default));
  });

  const navigationUrl = $derived(navigating.to?.url ?? page.url);
  const path = $derived(navigationUrl.pathname);
  const profile = $derived(session.me?.user.profile);
  const alert = $derived(badges.requests + badges.unread);

  $effect(() => {
    if (session.me) return watchBadges();
  });

  $effect(() => {
    const on = () => (scrolled = scrolledBy() > 8);
    on();
    // Окно прокручивается на компьютере, body - на телефоне и планшете (оболочка).
    window.addEventListener('scroll', on, { passive: true });
    document.body.addEventListener('scroll', on, { passive: true });
    return () => {
      window.removeEventListener('scroll', on);
      document.body.removeEventListener('scroll', on);
    };
  });

  $effect(() => {
    if (!menu) return;
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && (menu = false);
    addEventListener('keydown', esc);
    return () => removeEventListener('keydown', esc);
  });

  // Перешли куда-то - лист закрывается сам.
  $effect(() => {
    void path;
    menu = false;
  });

  // Компьютер («Витрина»): вся навигация сверху - знак, разделы текстом с «бегущей» чертой под выбранным, поиск с
  // фильтром, колокольчик и карточка профиля. Нижней панели на компьютере нет (она остаётся на телефоне и планшете).
  const NAV: { id: string; to: string; label: string; wide?: boolean }[] = [
    { id: 'home', to: '/', label: 'Главная' },
    { id: 'feed', to: '/feed', label: 'Что смотреть' },
    { id: 'movies', to: '/?tab=movies', label: 'Фильмы' },
    { id: 'series', to: '/?tab=series', label: 'Сериалы' },
    { id: 'anime', to: '/?tab=anime', label: 'Аниме', wide: true },
    { id: 'discover', to: '/discover', label: 'Обзор' },
    { id: 'my', to: '/my', label: 'Коллекция' },
  ];
  const activeNav = $derived(
    path === '/' ? tabOf(navigationUrl.searchParams) : path.startsWith('/feed') ? 'feed' : path.startsWith('/discover') ? 'discover'
      : path.startsWith('/my') || path.startsWith('/lists') ? 'my' : null,
  );
  // Черта под выбранным разделом: одна на всех, плавно переезжает (только transform и ширина через scaleX).
  let navEl = $state<HTMLElement>();
  let ink = $state<{ x: number; w: number } | null>(null);
  // Первый раз черта встаёт на место сразу (без «заезда» слева), дальше - переезжает плавно.
  let inkSlides = $state(false);
  function placeInk() {
    const on = navEl?.querySelector<HTMLElement>('a.on');
    const next = on && navEl && on.offsetWidth > 0 ? { x: on.offsetLeft, w: on.offsetWidth } : null;
    // Только если место и вправду изменилось: иначе запись в `ink` будила бы эффект снова и снова.
    if (next?.x === ink?.x && next?.w === ink?.w) return;
    ink = next;
    if (next && !inkSlides) requestAnimationFrame(() => requestAnimationFrame(() => (inkSlides = true)));
  }
  $effect(() => {
    void activeNav;
    untrack(placeInk);
    if (!navEl) return;
    const ro = new ResizeObserver(placeInk);
    ro.observe(navEl);
    void document.fonts?.ready.then(placeInk);
    return () => ro.disconnect();
  });
  let q = $state('');
  function find(e: SubmitEvent) {
    e.preventDefault();
    const v = q.trim();
    void goto(v ? `/search?q=${encodeURIComponent(v)}` : '/search');
  }
  const inHome = $derived(path === '/' || path.startsWith('/discover'));
  const inFeed = $derived(path.startsWith('/feed'));
  const inCollection = $derived(path.startsWith('/my') || path.startsWith('/lists'));
  // Вкладка «Профиль»: своя страница, оформление, люди, чаты и управление.
  const inProfile = $derived(['/people', '/chats', '/profile', '/admin'].some((p) => path.startsWith(p)));
  const news = $derived(badges.requests + badges.unread + (session.me?.manage ? badges.wishes : 0));
</script>

<header class:scrolled class:home={path === "/"} class:detail={path.startsWith("/title/") || path.startsWith("/ext/")}>
  <a href="/" class="brand" aria-label="На главную"><Logo /></a>

  <nav class="pills" aria-label="Разделы кинотеатра" bind:this={navEl}>
    {#each NAV as n (n.id)}
      <a href={n.to} class="sec" class:on={activeNav === n.id} class:wide={n.wide} aria-current={activeNav === n.id ? 'page' : undefined}>{n.label}</a>
    {/each}
    {#if ink}<i class="ink" class:slide={inkSlides} aria-hidden="true" style:--x="{ink.x}px" style:--w={ink.w}></i>{/if}
  </nav>

  <form class="find" role="search" onsubmit={find}>
    <Icon name="search" size={17} />
    <input type="search" bind:value={q} placeholder="Поиск фильмов и актёров" aria-label="Быстрый поиск" autocomplete="off" />
    <a class="tune" href="/search" aria-label="Все фильтры поиска" title="Фильтры"><Icon name="sliders" size={17} /></a>
  </form>

  <div class="right">
    <a class="round srch" href="/search" aria-label="Поиск" title="Поиск"><Icon name="search" size={21} /></a>
    <button class="round bell" aria-label={news ? `Уведомления: ${news}` : 'Уведомления'} title="Уведомления" aria-expanded={menu} onclick={() => (menu = !menu)}>
      <Icon name="bell" size={22} />{#if news}<i class="dot"></i>{/if}
    </button>
    {#if session.me?.manage}
      {#if badges.wishes}
        <a class="round" href="/admin?tab=wishes" aria-label="Просьбы зрителей: {badges.wishes}" title="Просьбы зрителей">
          <Icon name="flag" size={20} /><i class="num">{badges.wishes}</i>
        </a>
      {/if}
      <a class="round" href="/admin?tab=folders" aria-label="Добавить фильмы" title="Добавить фильмы"><Icon name="plus" size={22} /></a>
    {/if}
    {#if profile}
      <a class="chip" href="/people/{profile.id}" aria-label="Мой профиль" title="Мой профиль">
        <Avatar avatar={profile.avatar ?? 0} photo={profile.photo} size={30} />
        <span>{session.me?.user.name}</span>
      </a>
    {/if}
  </div>
</header>

{#if menu}
  <button class="scrim" aria-label="Закрыть уведомления" onclick={() => (menu = false)}></button>
  {#if Menu}<Menu close={() => (menu = false)} />{/if}
{/if}

<!-- Одна панель на всех экранах (телефон, планшет, компьютер, телевизор), под большим пальцем. Фильмы, сериалы,
     аниме и обзор - полосой разделов на главной; люди, чаты и подборки - в «Профиле» и «Коллекции». -->
<nav class="tabs" aria-label="Разделы">
  <a href="/" class:on={inHome} aria-current={inHome ? 'page' : undefined}><Icon name="home" size={24} /><span>Главное</span></a>
  <a href="/feed" class:on={inFeed} aria-current={inFeed ? 'page' : undefined}><Icon name="squareplay" size={24} /><span>Что смотреть</span></a>
  <a href={profile ? `/people/${profile.id}` : '/profile'} class:on={inProfile} aria-current={inProfile ? 'page' : undefined}>
    <span class="ic"><Avatar avatar={profile?.avatar ?? 0} photo={profile?.photo} size={24} />{#if alert}<i class="num">{alert}</i>{/if}</span>
    <span>Профиль</span>
  </a>
  <a href="/my" class:on={inCollection} aria-current={inCollection ? 'page' : undefined}><Icon name="bookmark" size={24} /><span>Коллекция</span></a>
  <a href="/search" class:on={path === '/search'} aria-current={path === '/search' ? 'page' : undefined}><Icon name="search" size={24} /><span>Поиск</span></a>
</nav>

<style>
  header {
    position: fixed;
    inset: 0 0 auto;
    z-index: 50;
    height: var(--header);
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 12px;
    padding: env(safe-area-inset-top) var(--gutter) 0;
    box-sizing: content-box;
    /* Над героем - только мягкая тень сверху, чтобы кнопки читались на светлом кадре. */
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.55), transparent);
    animation: head-in 0.7s var(--ease) both;
  }
  /* Над кадром фильма (страница фильма и главная): подложка гуще, чтобы разделы читались на светлом. */
  header.detail,
  header.home {
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.78), rgba(0, 0, 0, 0.4) 62%, transparent);
  }
  @keyframes head-in {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
  }
  header::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background: rgba(8, 8, 10, 0.94);
    border-bottom: 1px solid var(--line);
    opacity: 0;
    transition: opacity 0.25s;
  }
  header.scrolled::before {
    opacity: 1;
  }
  .brand {
    justify-self: start;
    min-width: 0;
  }
  .dot {
    position: absolute;
    top: -2px;
    right: -3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ff5c7a;
    box-shadow: 0 0 0 2px rgba(28, 28, 30, 1);
  }
  .num {
    position: absolute;
    top: -3px;
    right: -2px;
    min-width: 17px;
    height: 17px;
    padding: 0 4px;
    border-radius: 99px;
    background: #ff5c7a;
    color: #fff;
    font-size: 10.5px;
    font-style: normal;
    font-weight: 800;
    line-height: 17px;
    text-align: center;
    animation: badge 0.5s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  @keyframes badge {
    from {
      transform: scale(0);
    }
  }

  .right {
    justify-self: end;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .round {
    position: relative;
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    color: var(--text);
    /* «Жидкое стекло», как у Apple: только круглые кнопки шапки - маленькая площадь, видеокарту хоста
       не грузит (на всю страницу размытие убрали в 0.6). */
    background: rgba(255, 255, 255, 0.1);
    -webkit-backdrop-filter: blur(14px);
    backdrop-filter: blur(14px);
    border: 1px solid rgba(255, 255, 255, 0.22);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.28),
      inset 0 -1px 0 rgba(255, 255, 255, 0.06),
      0 10px 30px -12px rgba(0, 0, 0, 0.6);
    transition: background 0.15s;
    cursor: pointer;
  }
  .round:hover {
    background: rgba(255, 255, 255, 0.18);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: transparent;
    border: 0;
    cursor: default;
  }

  /* Панель разделов: одна на всех экранах. На широком окне вкладки собраны в середине (до 640 px), фон - во всю
     ширину. */
  .tabs {
    position: fixed;
    inset: auto 0 0;
    z-index: 50;
    /* Свой слой: на iPhone панель не уезжает со страницей при прокрутке и смене панелей Safari. */
    transform: translate3d(0, 0, 0);
    backface-visibility: hidden;
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    padding: 8px max(4px, calc((100% - 640px) / 2)) calc(6px + env(safe-area-inset-bottom));
    background: #141414;
    border-top: 1px solid #ffffff12;
  }
  .tabs a {
    display: grid;
    justify-items: center;
    gap: 2px;
    min-width: 0;
    padding: 4px 0;
    border: 0;
    background: none;
    font: inherit;
    /* Пять вкладок на 320 px: подпись уменьшается с экраном и не выходит за свою кнопку. */
    font-size: clamp(9.5px, 2.65vw, 12px);
    font-weight: 500;
    color: #8e8e93;
    cursor: pointer;
    transition: color 0.15s;
  }
  .tabs a:hover {
    color: #fff;
  }
  .tabs a > span:last-child {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tabs .on {
    color: #fff;
  }
  /* Значок раздела пружинит при нажатии и подпрыгивает, когда раздел выбран. */
  .tabs :global(svg),
  .tabs .ic {
    transition: transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .tabs a:active :global(svg),
  .tabs a:active .ic {
    transform: scale(0.85);
  }
  .tabs a.on :global(svg),
  .tabs a.on .ic {
    animation: tab-pop 0.4s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  @keyframes tab-pop {
    40% {
      transform: translateY(-3px) scale(1.12);
    }
  }
  .tabs .ic {
    position: relative;
    display: grid;
    height: 22px;
    place-items: center;
  }
  .tabs a:not(.on) .ic {
    opacity: 0.75;
  }
  .tabs .num {
    top: -6px;
    right: -10px;
  }

  .pills,
  .find,
  .chip {
    display: none;
  }

  /* Компьютер («Витрина»): навигация - вся сверху. Разделы текстом, под выбранным - чёрта, которая плавно переезжает;
     поиск с фильтром, колокольчик и карточка профиля. Нижней панели нет. */
  @media (min-width: 1001px) {
    header {
      display: flex;
      gap: clamp(14px, 1.8vw, 30px);
    }
    .tabs,
    .srch {
      display: none;
    }
    .brand {
      flex: none;
    }
    .pills {
      position: relative;
      display: flex;
      align-items: center;
      gap: clamp(16px, 1.7vw, 30px);
      height: 100%;
    }
    .pills .sec {
      position: relative;
      padding: 10px 2px;
      color: #ffffff94;
      font-size: 15px;
      font-weight: 600;
      white-space: nowrap;
      transition: color 0.25s cubic-bezier(0.22, 1, 0.36, 1);
    }
    .pills .sec:hover {
      color: #fff;
    }
    .pills .sec.on {
      color: #fff;
    }
    .pills .wide {
      display: none;
    }
    /* Одна чёрта на всех: двигается и растягивается за выбранным разделом. */
    .ink {
      position: absolute;
      left: 0;
      bottom: 14px;
      width: 1px;
      height: 3px;
      border-radius: 3px;
      background: #fff;
      transform-origin: 0 50%;
      transform: translateX(var(--x)) scaleX(var(--w));
      will-change: transform;
    }
    .ink.slide {
      transition: transform 0.5s cubic-bezier(0.22, 1, 0.36, 1);
    }
    .pills .sec:active {
      transform: scale(0.96);
    }
    .find {
      display: flex;
      align-items: center;
      gap: 8px;
      flex: 1;
      max-width: 360px;
      min-width: 150px;
      margin-left: auto;
      height: 42px;
      padding: 0 6px 0 16px;
      border: 1px solid transparent;
      border-radius: 999px;
      background: #ffffff14;
      color: #ffffff94;
      transition: background 0.25s, border-color 0.25s, color 0.25s, transform 0.3s cubic-bezier(0.22, 1, 0.36, 1);
    }
    .find:hover {
      background: #ffffff1c;
    }
    .find:focus-within {
      border-color: #ffffffa0;
      background: #ffffff1f;
      color: #fff;
      transform: scale(1.015);
    }
    .find input {
      flex: 1;
      min-width: 0;
      border: 0;
      outline: 0;
      background: none;
      color: #fff;
      font: inherit;
      font-size: 14px;
    }
    .tune {
      display: grid;
      place-items: center;
      flex: none;
      width: 32px;
      height: 32px;
      border-radius: 50%;
      color: #ffffffb0;
      transition: background 0.2s, color 0.2s;
    }
    .tune:hover {
      background: #ffffff1f;
      color: #fff;
    }
    .right {
      flex: none;
    }
    .round {
      border: 0;
      background: #ffffff14;
      box-shadow: none;
      -webkit-backdrop-filter: none;
      backdrop-filter: none;
      transition: background 0.2s, transform 0.2s cubic-bezier(0.22, 1, 0.36, 1);
    }
    .round:hover {
      transform: translateY(-1px);
    }
    .chip {
      display: flex;
      align-items: center;
      gap: 8px;
      height: 42px;
      max-width: 200px;
      padding: 0 14px 0 5px;
      border-radius: 999px;
      background: #ffffff14;
      font-size: 14px;
      font-weight: 600;
      transition: background 0.25s, transform 0.2s cubic-bezier(0.22, 1, 0.36, 1);
    }
    .chip:hover {
      background: #ffffff22;
    }
    .chip span {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }
  @media (min-width: 1001px) and (max-width: 1239px) {
    .chip span {
      display: none;
    }
    .chip {
      padding: 0 5px;
    }
  }
  @media (min-width: 1280px) {
    .pills .wide {
      display: block;
    }
  }

  @media (max-width: 1240px) {
    .brand :global(.word) {
      display: none;
    }
  }
  @media (max-width: 1000px) {
    .brand :global(.word) {
      display: inline;
    }
  }
  @media (max-width: 640px) {
    .brand :global(.word) {
      display: none;
    }
    /* Телефон (как в Кинопоиске): шапка - только на главной (знак слева, колокольчик справа); у остальных
       экранов свой крупный заголовок и места под шапку нет. */
    header:not(.home) {
      display: none;
    }
    .right .round:not(.bell):not(.srch) {
      display: none;
    }
    .brand :global(.logo svg),
    .brand :global(img) {
      height: 28px;
    }
    /* Страница фильма на телефоне - без шапки: кадр от самого верха, кнопки - на нём. */
    header.detail {
      display: none;
    }
  }
</style>
