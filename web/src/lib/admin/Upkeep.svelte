<script lang="ts">
  import { errorText, post } from '$lib/api';
  import { ago, size } from '$lib/fmt';
  import { ask } from '$lib/ui/dialogs.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import type { Status } from './types';

  // Обслуживание сервера: копия базы и кэш видео - здесь, в «Системе → Сервер» (раньше - внизу «Обзора»).
  let { status, reload }: { status: Status; reload: () => void } = $props();
  let busy = $state('');
  const sys = $derived(status.system);
  const pct = (a: number | null | undefined, b: number | null | undefined) => (a != null && b ? Math.round((a / b) * 100) : null);
  const meters = $derived([
    { label: 'Процессор', p: sys?.cpu != null ? Math.round(sys.cpu) : null, note: sys?.cpu != null ? `${Math.round(sys.cpu)}%` : 'нет данных' },
    { label: 'Память', p: pct(sys?.memoryUsed, sys?.memoryTotal), note: sys?.memoryTotal ? `${size(sys.memoryUsed ?? 0)} из ${size(sys.memoryTotal)}` : 'нет данных' },
    { label: 'Диск', p: sys?.diskTotal && sys.diskFree != null ? pct(sys.diskTotal - sys.diskFree, sys.diskTotal) : null, note: sys?.diskFree != null ? `${size(sys.diskFree)} свободно` : 'нет данных' },
  ]);

  async function backup() {
    busy = 'backup';
    try {
      const r = await post<{ bytes: number; folder: string }>('/api/admin/backup');
      toast(`Копия базы готова (${size(r.bytes)}): ${r.folder}`, 7000);
      reload();
    } catch (e) {
      toast(errorText(e), 6000);
    } finally {
      busy = '';
    }
  }
  async function clearCache() {
    if (!(await ask('Удалить кэш видео? Ничего нужного не пропадёт: при следующем просмотре видео соберётся заново.', 'Очистить', true))) return;
    busy = 'cache';
    try {
      const r = await post<{ freed: number }>('/api/admin/cache/clear');
      toast(`Освобождено ${size(r.freed)}`);
    } catch (e) {
      toast(errorText(e), 6000);
    } finally {
      busy = '';
    }
  }
</script>

{#if sys?.supported}
  <div class="meters">
    {#each meters as m (m.label)}
      <div class="meter">
        <div class="top"><b>{m.label}</b><span>{m.p != null ? `${m.p}%` : ''}</span></div>
        <div class="bar" aria-hidden="true"><i style:transform="scaleX({(m.p ?? 0) / 100})"></i></div>
        <small>{m.note}</small>
      </div>
    {/each}
  </div>
{/if}

<section class="card pane" aria-labelledby="up-h">
  <h3 id="up-h">Обслуживание</h3>
  <div class="row">
    <span><b>Копия базы</b><small>{status.lastBackup ? `Последняя - ${ago(status.lastBackup)}` : 'Копий ещё нет'}. Перед обновлением делается сама, ручных хранится пять</small></span>
    <button class="btn sm" type="button" disabled={busy === 'backup'} onclick={backup}><Icon name="download" size={15} /> {busy === 'backup' ? 'Делаем…' : 'Сделать копию'}</button>
  </div>
  <div class="row">
    <span><b>Кэш видео</b><small>Перекодированное и «Оригинал». Не очищается во время просмотра</small></span>
    <button class="btn sm" type="button" disabled={busy === 'cache'} onclick={clearCache}><Icon name="trash" size={15} /> Очистить</button>
  </div>
</section>

<style>
  .meters { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px; margin-bottom: 16px; }
  .meter { display: grid; gap: 8px; padding: 16px; border: 1px solid var(--line); border-radius: 20px; background: #141416; min-width: 0; }
  .top { display: flex; justify-content: space-between; gap: 8px; }
  .top span { color: var(--muted); font-variant-numeric: tabular-nums; }
  .bar { height: 6px; border-radius: 99px; background: #ffffff1f; overflow: hidden; }
  .bar i { display: block; height: 100%; background: #fff; transform-origin: 0 50%; transition: transform 0.6s var(--ease); }
  .meter small { color: var(--muted); font-size: 12.5px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 16px; padding: 12px 0; border-top: 1px solid var(--line); }
  .row:first-of-type { border-top: 0; }
  .row > span { display: grid; gap: 2px; min-width: 0; }
  .row small { color: var(--muted); font-size: 12.5px; }
  @media (max-width: 640px) {
    .meters { grid-template-columns: minmax(0, 1fr); }
    .row { flex-wrap: wrap; }
  }
</style>
