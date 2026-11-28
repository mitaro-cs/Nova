<script lang="ts">
  import { api, errorText } from '$lib/api';
  import { count } from '$lib/fmt';
  import FriendButton from '$lib/social/FriendButton.svelte';
  import type { Member } from '$lib/types';
  import Avatar from '$lib/ui/Avatar.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Sheet from '$lib/ui/Sheet.svelte';

  // Люди сайта: найти друзей, принять заявки. Сверху - кто просит дружбы, потом друзья.
  let list = $state<Member[] | null>(null);
  let error = $state('');
  let q = $state('');

  $effect(() => {
    api<Member[]>('/api/members').then((r) => (list = r), (e) => (error = errorText(e)));
  });

  const norm = (s: string) => s.toLowerCase().replace(/ё/g, 'е');
  const shown = $derived((list ?? []).filter((m) => !q.trim() || norm(m.name).includes(norm(q.trim()))));
  const groups = $derived([
    { id: 'incoming', title: 'Хотят дружить', items: shown.filter((m) => m.relation === 'incoming') },
    { id: 'friend', title: 'Друзья', items: shown.filter((m) => m.relation === 'friend') },
    { id: 'rest', title: 'Все на сайте', items: shown.filter((m) => m.relation === 'none' || m.relation === 'outgoing') },
  ]);
</script>

<Sheet title="Люди" subtitle="Друзья, заявки и все, кто есть на сайте">
  {#snippet actions()}
    <a class="btn sm" href="/my"><Icon name="users" size={15} /> Смотрим вместе</a>
  {/snippet}
  <div class="rw-block">
    <label class="find">
      <Icon name="search" size={18} />
      <input type="search" bind:value={q} placeholder="Найти по имени" aria-label="Найти человека" />
    </label>
  </div>

  {#if error}
    <div class="empty"><Icon name="info" size={36} /> {error}</div>
  {:else if list === null}
    <div class="sk skeleton"></div>
  {:else if list.length === 0}
    <div class="empty"><Icon name="users" size={40} /> Пока здесь только вы. Позовите друзей - регистрация открыта.</div>
  {:else}
    {#each groups as g (g.id)}
      {#if g.items.length}
        <section class="rw-block" aria-label={g.title}>
          <h2 class="rw-h2">{g.title} <span class="faint">{g.items.length}</span></h2>
          <ul>
            {#each g.items as m, i (m.id)}
              <li style:--i={i}>
                <a class="who" href="/people/{m.id}">
                  <Avatar avatar={m.avatar} photo={m.photo} size={48} />
                  <span>
                    <b>{m.name}</b>
                    <small class="muted">{count(m.watched, 'фильм просмотрен', 'фильма просмотрено', 'фильмов просмотрено')} · {count(m.reviews, 'отзыв', 'отзыва', 'отзывов')}</small>
                  </span>
                </a>
                <FriendButton id={m.id} bind:relation={m.relation} small />
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/each}
  {/if}
</Sheet>
<style>

  .find {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 46px;
    padding: 0 16px;
    border-radius: 999px;
    border: 1.5px solid var(--line-strong);
    color: var(--muted);
  }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--text);
  }

  section h2 {
    display: flex;
    gap: 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px 14px;
    padding: 12px 14px;
    border-radius: 16px;
    background: var(--surface-2);
    animation: rise 0.35s calc(min(var(--i), 10) * 30ms) cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    flex: 1 1 220px;
  }
  .who span {
    display: grid;
    min-width: 0;
  }
  .who b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who small {
    font-size: 12px;
  }
  .sk {
    height: 240px;
    border-radius: 16px;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  :global(.no-motion) li {
    animation: none;
  }
</style>
