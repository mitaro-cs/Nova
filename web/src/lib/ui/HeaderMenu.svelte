<script lang="ts">
  import { goto } from '$app/navigation';
  import { post } from '$lib/api';
  import { session } from '$lib/session.svelte';
  import { badges } from '$lib/social/badges.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  // Меню аватара (компьютер): своя страница, люди, чаты, списки, оформление, управление, комната по ссылке,
  // экскурсия, выход. Пружинит из угла, пункты - лесенкой. `kind="bell"` - лист колокольчика на телефоне:
  // просьбы дружбы, непрочитанные сообщения и (у хоста) просьбы зрителей. Грузится по нажатию (бюджет JS главной).
  let { kind, path, close }: { kind: 'top' | 'bell'; path: string; close: () => void } = $props();
  const profile = $derived(session.me?.user.profile);
  const news = $derived(badges.requests + badges.unread + (session.me?.manage ? badges.wishes : 0));

  async function logout() {
    await post('/api/auth/logout').catch(() => {});
    session.me = null;
    goto('/login');
  }

  type Item = { href: string; icon: IconName; label: string; count?: number; dot?: boolean };
  const items = $derived<Item[]>([
    ...(profile ? [{ href: `/people/${profile.id}`, icon: 'user' as const, label: 'Моя страница' }] : []),
    { href: '/people', icon: 'users', label: 'Люди', dot: badges.requests > 0 },
    { href: '/chats', icon: 'mail', label: 'Чаты', count: badges.unread },
    { href: '/lists', icon: 'list', label: 'Списки' },
  ]);
  const more = $derived<Item[]>([
    { href: '/profile', icon: 'image', label: 'Профиль и оформление' },
    ...(session.me?.manage ? [{ href: '/admin', icon: 'sliders' as const, label: 'Управление' }] : []),
  ]);
</script>

<div class="menu" role="menu" aria-label={kind === 'bell' ? 'Уведомления' : 'Меню'}>
  {#if kind === 'bell'}
    {#if news}
      {#if badges.requests}<a href="/people" role="menuitem"><span class="ic"><Icon name="users" size={19} /></span><span class="lbl">Просьбы дружбы</span><i class="cnt">{badges.requests}</i></a>{/if}
      {#if badges.unread}<a href="/chats" role="menuitem"><span class="ic"><Icon name="mail" size={19} /></span><span class="lbl">Новые сообщения</span><i class="cnt">{badges.unread}</i></a>{/if}
      {#if session.me?.manage && badges.wishes}<a href="/admin?tab=wishes" role="menuitem"><span class="ic"><Icon name="flag" size={19} /></span><span class="lbl">Просьбы зрителей</span><i class="cnt">{badges.wishes}</i></a>{/if}
    {:else}
      <p class="none">Новых уведомлений нет</p>
    {/if}
  {:else}
  <a class="who" href={profile ? `/people/${profile.id}` : '/profile'} role="menuitem">
    <Avatar avatar={profile?.avatar ?? 0} photo={profile?.photo} size={44} />
    <span>
      <b>{session.me?.user.name}</b>
      <span class="muted">@{session.me?.user.login}</span>
    </span>
  </a>
  {#each items as it, k (it.href)}
    <a href={it.href} role="menuitem" class:on={path.startsWith(it.href)} style:--k={k}>
      <span class="ic"><Icon name={it.icon} size={19} />{#if it.dot}<i class="dot"></i>{/if}</span>
      <span class="lbl">{it.label}</span>
      {#if it.count}<i class="cnt">{it.count}</i>{/if}
    </a>
  {/each}
  <hr />
  {#each more as it, k (it.href)}
    <a href={it.href} role="menuitem" class:on={path.startsWith(it.href)} style:--k={k + items.length}>
      <span class="ic"><Icon name={it.icon} size={19} /></span><span class="lbl">{it.label}</span>
    </a>
  {/each}
  <button role="menuitem" onclick={() => { close(); window.dispatchEvent(new Event('nova-room-link')); }} style:--k={items.length + more.length}>
    <span class="ic"><Icon name="party" size={19} /></span><span class="lbl">Комната по ссылке</span>
  </button>
  <button role="menuitem" onclick={() => { close(); window.dispatchEvent(new Event('nova-tour')); }} style:--k={items.length + more.length + 1}>
    <span class="ic"><Icon name="sparkle" size={19} /></span><span class="lbl">Экскурсия по Nova</span>
  </button>
  {#if !session.me?.window}
    <button role="menuitem" onclick={logout} style:--k={items.length + more.length + 2}>
      <span class="ic"><Icon name="logout" size={19} /></span><span class="lbl">Выйти</span>
    </button>
  {/if}
  {/if}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 71;
    right: max(var(--gutter), env(safe-area-inset-right));
    top: calc(var(--header) + env(safe-area-inset-top) - 6px);
    width: 280px;
    max-width: calc(100vw - 32px);
    padding: 8px;
    border-radius: 18px;
    background: rgba(26, 26, 28, 0.98);
    border: 1px solid var(--line);
    box-shadow: 0 30px 70px -14px rgba(0, 0, 0, 0.85);
    display: grid;
    transform-origin: top right;
    animation: pop 0.32s cubic-bezier(0.2, 0.9, 0.3, 1.25);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.88) translateY(-8px);
    }
  }
  .who {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px 12px;
    margin-bottom: 4px;
    border-bottom: 1px solid var(--line);
    border-radius: 12px 12px 0 0;
    min-width: 0;
  }
  .who span {
    display: grid;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .who .muted {
    font-size: 12.5px;
  }
  .menu a:not(.who),
  .menu button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 12px;
    border: 0;
    background: none;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font-size: 15px;
    font-weight: 500;
    animation: item 0.35s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--k, 0) * 28ms + 40ms);
    transition:
      background 0.15s,
      transform 0.15s;
  }
  @keyframes item {
    from {
      opacity: 0;
      transform: translateX(10px);
    }
  }
  .menu a:hover,
  .menu button:hover {
    background: var(--surface-2);
  }
  .menu a:active,
  .menu button:active {
    transform: scale(0.98);
  }
  .menu a.on {
    background: rgba(255, 255, 255, 0.08);
  }
  .menu .ic {
    position: relative;
    display: grid;
    color: var(--muted);
  }
  .menu .lbl {
    flex: 1;
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
  .cnt {
    min-width: 22px;
    height: 22px;
    padding: 0 7px;
    border-radius: 99px;
    background: #ff5c7a;
    color: #fff;
    font-size: 12px;
    font-style: normal;
    font-weight: 800;
    line-height: 22px;
    text-align: center;
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 4px 0;
  }
  .none {
    margin: 0;
    padding: 14px 12px;
    color: var(--muted);
    font-size: 14px;
  }
</style>
