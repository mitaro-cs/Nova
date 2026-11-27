<script lang="ts">
  import { post, errorText } from '$lib/api';
  import { toast } from '$lib/ui/toast.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import { attention } from './attention';
  import type { Health, Status } from './types';

  // «Требует внимания»: всё, что ждёт решения, важное сверху; кнопка ведёт туда, где это решается.
  let { status, health, wishes, reload }: { status: Status; health: Health | null; wishes: number; reload: () => void } = $props();
  const items = $derived(attention(status, health, wishes));
  let busy = $state('');

  async function run(id: string, act: 'backup') {
    if (busy) return;
    busy = id;
    try {
      if (act === 'backup') {
        await post('/api/admin/backup');
        toast('Копия базы готова');
      }
      reload();
    } catch (e) {
      toast(errorText(e));
    } finally {
      busy = '';
    }
  }
</script>

<section class="att" aria-labelledby="att-h">
  <h3 id="att-h">Требует внимания {#if items.length}<span class="n">{items.length}</span>{/if}</h3>
  {#if items.length}
    <ul>
      {#each items as a (a.id)}
        <li>
          <i class="dot {a.level}" aria-hidden="true"></i>
          <span class="txt"><b>{a.title}</b><small>{a.text}</small></span>
          {#if a.act}
            <button class="btn sm" type="button" disabled={busy === a.id} onclick={() => run(a.id, a.act!)}>{busy === a.id ? 'Делаем…' : a.label}</button>
          {:else if a.href}
            <a class="btn sm" href={a.href} data-sveltekit-replacestate data-sveltekit-noscroll>{a.label}</a>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="calm"><Icon name="check" size={18} /> Всё в порядке - ничего не ждёт вашего решения</p>
  {/if}
</section>

<style>
  .att { min-width: 0; padding: 20px 22px; border: 1px solid var(--line); border-radius: 24px; background: #141416; }
  h3 { display: flex; align-items: center; gap: 10px; margin: 0 0 6px; font-size: 16px; letter-spacing: -0.01em; }
  .n { min-width: 22px; padding: 1px 8px; border-radius: 99px; background: #ffffff1a; font-size: 12px; text-align: center; }
  ul { margin: 0; padding: 0; list-style: none; }
  li { display: grid; grid-template-columns: 10px minmax(0, 1fr) auto; gap: 14px; align-items: center; padding: 12px 0; border-top: 1px solid var(--line); }
  li:first-child { border-top: 0; }
  .dot { width: 8px; height: 8px; border-radius: 50%; }
  .dot.bad { background: var(--danger); box-shadow: 0 0 0 4px color-mix(in srgb, var(--danger) 22%, transparent); }
  .dot.warn { background: #ffd60a; }
  .dot.info { background: #8e8e93; }
  .txt { display: grid; gap: 2px; min-width: 0; }
  .txt b { font-size: 14px; font-weight: 600; }
  .txt small { color: var(--muted); font-size: 12.5px; line-height: 1.4; }
  .calm { display: flex; align-items: center; gap: 10px; margin: 8px 0 0; color: var(--muted); }
  .calm :global(svg) { color: var(--ok); }
  @media (max-width: 520px) {
    li { grid-template-columns: 10px minmax(0, 1fr); }
    li > :last-child { grid-column: 2; justify-self: start; }
  }
</style>
