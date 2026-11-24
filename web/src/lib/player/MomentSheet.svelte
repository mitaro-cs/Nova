<script lang="ts">
  import { api, errorText, post } from '$lib/api';
  import { clock } from '$lib/fmt';
  import type { Who } from '$lib/types';
  import Avatar from '$lib/ui/Avatar.svelte';
  import Icon from '$lib/ui/Icon.svelte';

  // «Отметить момент» (выбор владельца, 23.11): ссылка на эту секунду фильма (`?t=`) - скопировать себе или сразу
  // отправить другу сообщением с подписью. Открывший ссылку попадает ровно на этот момент.
  let { at, title, fileId = 0, linkId = null, titleId = 0, onclose }: { at: number; title: string; fileId?: number; linkId?: number | null; titleId?: number; onclose: () => void } = $props();

  const link = $derived(`${location.origin}${linkId ? `/watch/link/${linkId}` : `/watch/${fileId}`}?t=${at}`);
  let note = $state('');
  let copied = $state(false);
  let friends = $state<Who[] | null>(null);
  let sent = $state<number[]>([]);
  let error = $state('');

  $effect(() => {
    api<{ items: { person: Who }[] }>('/api/chats').then((r) => (friends = r.items.map((i) => i.person)), () => (friends = []));
  });
  async function copy() {
    try {
      await navigator.clipboard.writeText(link);
      copied = true;
    } catch {
      copied = false;
    }
  }
  async function send(p: Who) {
    error = '';
    const body = `${note.trim() ? `${note.trim()} - ` : ''}«${title}», момент ${clock(at)}: ${link}`;
    try {
      await post(`/api/chats/${p.id}`, titleId ? { body, titleId } : { body });
      sent = [...sent, p.id];
    } catch (x) {
      error = errorText(x);
    }
  }
</script>

<div class="ms" role="dialog" aria-label="Отметить момент">
  <div class="box">
    <header><b>Момент {clock(at)}</b><button class="x" type="button" aria-label="Закрыть" onclick={onclose}><Icon name="close" size={16} /></button></header>
    <p class="t">«{title}»</p>
    <input class="input" bind:value={note} maxlength="200" placeholder="Подпись, например «вот эта сцена»" aria-label="Подпись к моменту" />
    <button class="btn primary" type="button" onclick={copy}><Icon name={copied ? 'check' : 'link'} size={16} /> {copied ? 'Ссылка скопирована' : 'Скопировать ссылку'}</button>
    <small class="lab">Отправить другу</small>
    {#if friends === null}
      <p class="muted">Загружаем друзей…</p>
    {:else if !friends.length}
      <p class="muted">Друзей пока нет - отправьте ссылку.</p>
    {:else}
      <ul>
        {#each friends.slice(0, 8) as f (f.id)}
          <li>
            <Avatar avatar={f.avatar} photo={f.photo} size={28} /><span>{f.name}</span>
            <button class="btn sm" type="button" disabled={sent.includes(f.id)} onclick={() => send(f)}>{sent.includes(f.id) ? 'Отправлено' : 'Отправить'}</button>
          </li>
        {/each}
      </ul>
    {/if}
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </div>
</div>

<style>
  .ms { position: absolute; inset: 0; z-index: 9; display: grid; place-items: center; padding: 20px; background: #0009; }
  .box { display: grid; gap: 12px; width: min(400px, 100%); max-height: calc(100% - 40px); overflow-y: auto; padding: 18px; border-radius: 20px; background: #161619; border: 1px solid #ffffff1f; color: #fff; }
  header { display: flex; align-items: center; justify-content: space-between; font-size: 18px; }
  .x { display: grid; place-items: center; width: 32px; height: 32px; border: 0; border-radius: 50%; background: #ffffff14; color: #fff; cursor: pointer; }
  .t { margin: -6px 0 0; color: #a1a1aa; font-size: 14px; }
  .lab { color: #a1a1aa; font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; }
  ul { display: grid; gap: 6px; margin: 0; padding: 0; list-style: none; }
  li { display: flex; align-items: center; gap: 10px; }
  li span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  li :global(img), li :global(.avatar) { border-radius: 50%; }
  .muted { margin: 0; color: #a1a1aa; font-size: 13px; }
  .err { margin: 0; color: #ef6a6a; font-size: 13px; }
</style>
