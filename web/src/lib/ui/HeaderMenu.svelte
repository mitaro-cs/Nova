<script lang="ts">
  import { badges, readyHref, seeReady } from '$lib/social/badges.svelte';
  import { session } from '$lib/session.svelte';
  import Icon from './Icon.svelte';

  // Лист колокольчика: добавленное по просьбе, просьбы дружбы, непрочитанные сообщения и (у хоста) просьбы зрителей.
  // Пружинит из угла. Грузится по нажатию (бюджет JS главной).
  let { close }: { close: () => void } = $props();
  const news = $derived(badges.requests + badges.unread + badges.ready.length + (session.me?.manage ? badges.wishes : 0));
</script>

<div class="menu" role="menu" aria-label="Уведомления">
  {#if news}
    {#each badges.ready as r (r.id)}
      <a href={readyHref(r)} role="menuitem" onclick={() => (seeReady(r), close())}><span class="ic"><Icon name="check" size={19} /></span><span class="lbl">{r.kind === 'series' ? 'Сериал' : 'Фильм'} добавлен: {r.title}</span></a>
    {/each}
    {#if badges.requests}<a href="/people" role="menuitem" onclick={close}><span class="ic"><Icon name="users" size={19} /></span><span class="lbl">Просьбы дружбы</span><i class="cnt">{badges.requests}</i></a>{/if}
    {#if badges.unread}<a href="/chats" role="menuitem" onclick={close}><span class="ic"><Icon name="mail" size={19} /></span><span class="lbl">Новые сообщения</span><i class="cnt">{badges.unread}</i></a>{/if}
    {#if session.me?.manage && badges.wishes}<a href="/admin?tab=wishes" role="menuitem" onclick={close}><span class="ic"><Icon name="flag" size={19} /></span><span class="lbl">Просьбы зрителей</span><i class="cnt">{badges.wishes}</i></a>{/if}
  {:else}
    <p class="none">Новых уведомлений нет</p>
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
  .menu a {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 12px;
    color: var(--text);
    font-size: 15px;
    font-weight: 500;
    transition:
      background 0.15s,
      transform 0.15s;
  }
  .menu a:hover {
    background: var(--surface-2);
  }
  .menu a:active {
    transform: scale(0.98);
  }
  .ic {
    display: grid;
    color: var(--muted);
  }
  .lbl {
    flex: 1;
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
  .none {
    margin: 0;
    padding: 14px 12px;
    color: var(--muted);
    font-size: 14px;
  }
</style>
