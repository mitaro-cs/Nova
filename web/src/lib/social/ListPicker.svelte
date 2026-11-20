<script lang="ts">
  import { api, put } from '$lib/api';
  import Icon from '$lib/ui/Icon.svelte';
  import Modal from '$lib/ui/Modal.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import NewList from './NewList.svelte';

  // Со страницы фильма: в какие совместные подборки положить (галочки), или новая подборка.
  let { open = $bindable(false), titleId }: { open: boolean; titleId: number } = $props();

  let lists = $state<{ id: number; name: string; has: boolean }[] | null>(null);
  let creating = $state(false);

  function load() {
    api<{ id: number; name: string; has: boolean }[]>(`/api/titles/${titleId}/lists`).then(
      (r) => (lists = r),
      () => (lists = []),
    );
  }
  $effect(() => {
    if (open) load();
  });

  async function toggle(l: { id: number; name: string; has: boolean }) {
    l.has = !l.has;
    try {
      await put(`/api/lists/${l.id}/items/${titleId}`, { value: l.has });
      toast(l.has ? `Добавили в «${l.name}»` : `Убрали из «${l.name}»`);
    } catch {
      l.has = !l.has;
    }
  }

  async function created(id: number) {
    await put(`/api/lists/${id}/items/${titleId}`, { value: true }).catch(() => {});
    open = true;
    load();
  }
</script>

<Modal bind:open title="Подборки с друзьями">
  <div class="box">
    <p class="lead">Куда положить этот фильм? Подборку видят и дополняют все участники.</p>
    {#if lists === null}
      <div class="skeleton ph"></div>
    {:else if lists.length === 0}
      <div class="none">
        <span class="big"><Icon name="users" size={26} /></span>
        <b>Подборок пока нет</b>
        <p class="muted">Создайте первую - и смотрите вместе с друзьями.</p>
      </div>
    {:else}
      <ul>
        {#each lists as l (l.id)}
          <li>
            <button class:on={l.has} role="menuitemcheckbox" aria-checked={l.has} onclick={() => toggle(l)}>
              <span class="av" aria-hidden="true">{l.name.trim().charAt(0).toUpperCase() || '?'}</span>
              <span class="txt"><span class="name">{l.name}</span><small>{l.has ? 'Фильм уже здесь' : 'Нажмите, чтобы добавить'}</small></span>
              <span class="sw" aria-hidden="true"><i></i></span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <div class="row">
      <a class="btn ghost" href="/lists" onclick={() => (open = false)}>Все подборки</a>
      <button class="btn primary" onclick={() => ((open = false), (creating = true))}><Icon name="plus" size={16} /> Новая подборка</button>
    </div>
  </div>
</Modal>
<NewList bind:open={creating} oncreated={created} />

<style>
  .box { display: grid; gap: 16px; min-width: 0; }
  .lead { margin: 0; color: var(--muted); font-size: 14px; line-height: 1.45; }
  .ph { height: 96px; border-radius: 16px; }
  .none { display: grid; justify-items: center; gap: 6px; padding: 26px 12px; border: 1px dashed #ffffff2e; border-radius: 22px; text-align: center; }
  .none p { margin: 0; font-size: 14px; }
  .big { display: grid; place-items: center; width: 56px; height: 56px; margin-bottom: 6px; border-radius: 50%; background: #ffffff14; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; max-height: min(52vh, 420px); overflow-y: auto; }
  li button {
    width: 100%; min-width: 0; display: flex; align-items: center; gap: 14px; padding: 12px 14px;
    border: 1px solid #ffffff1a; border-radius: 20px; background: #141416; color: inherit; text-align: left; cursor: pointer;
    transition: background 0.2s, border-color 0.2s, transform 0.2s var(--ease);
  }
  li button:hover { background: #1a1a1d; border-color: #ffffff33; transform: translateY(-1px); }
  li button:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }
  li button.on { border-color: #ffffff59; background: #1c1c1f; }
  .av { display: grid; place-items: center; flex: none; width: 44px; height: 44px; border-radius: 14px; background: #ffffff1a; font-size: 18px; font-weight: 700; }
  .txt { display: grid; min-width: 0; flex: 1; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; font-size: 16px; }
  .txt small { color: var(--muted); font-size: 12.5px; }
  .sw { position: relative; flex: none; width: 46px; height: 28px; border-radius: 999px; background: #ffffff26; transition: background 0.2s; }
  .sw i { position: absolute; top: 3px; left: 3px; width: 22px; height: 22px; border-radius: 50%; background: #fff; transition: transform 0.25s var(--ease); }
  .on .sw { background: var(--ok); }
  .on .sw i { transform: translateX(18px); }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
  .row .btn { min-height: 44px; }
</style>
