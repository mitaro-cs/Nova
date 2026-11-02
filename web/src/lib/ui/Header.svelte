<script lang="ts">
  import type { Component } from 'svelte';
  import { page, navigating } from '$app/state';
  import { session } from '$lib/session.svelte';
  import { badges, watchBadges } from '$lib/social/badges.svelte';
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
    const on = () => (scrolled = window.scrollY > 8);
    on();
    window.addEventListener('scroll', on, { passive: true });
    return () => window.removeEventListener('scroll', on);
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

  const inHome = $derived(path === '/' || path.startsWith('/discover'));
  const inFeed = $derived(path.startsWith('/feed'));
  const inCollection = $derived(path.startsWith('/my') || path.startsWith('/lists'));
  // Вкладка «Профиль»: своя страница, оформление, люди, чаты и управление.
  const inProfile = $derived(['/people', '/chats', '/profile', '/admin'].some((p) => path.startsWith(p)));
  const news = $derived(badges.requests + badges.unread + (session.me?.manage ? badges.wishes : 0));
</script>

<header class:scrolled class:home={path === "/"} class:detail={path.startsWith("/title/") || path.startsWith("/ext/")}>
  <a href="/" class="brand" aria-label="На главную"><Logo /></a>

  <div class="right">
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
    .right .round:not(.bell) {
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
