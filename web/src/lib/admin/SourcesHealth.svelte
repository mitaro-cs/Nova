<script lang="ts">
  import type { Health } from './types';

  // Источники видео по ссылке: работает, выключен ненадолго (три ошибки подряд - на 10 минут) или ещё не проверялся.
  let { health }: { health: Health | null } = $props();
  const STATE = { ok: 'Работает', down: 'Выключен на 10 минут после ошибок', unknown: 'Ещё не проверялся' } as const;
</script>

<section class="card pane" aria-labelledby="sh-h">
  <h3 id="sh-h">Состояние источников</h3>
  <p class="muted small">Где Nova ищет фильмы, которых нет в ваших папках. Сломавшийся источник выключается сам и включается снова через 10 минут.</p>
  {#if health?.sources.length}
    <ul class="list">
      {#each health.sources as s (s.site)}
        <li>
          <i class="dot {s.state}" aria-hidden="true"></i>
          <span class="nm"><b>{s.label}</b><small>{STATE[s.state]}{s.fails && s.state !== 'down' ? ` · ошибок подряд: ${s.fails}` : ''}</small></span>
          <span class="ms">{s.ok_ms != null ? `${(s.ok_ms / 1000).toLocaleString('ru-RU', { maximumFractionDigits: 1 })} с` : ''}</span>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted small">{health ? 'Источники ещё не проверялись' : 'Проверяем…'}</p>
  {/if}
</section>

<style>
  .list { margin: 0; padding: 0; list-style: none; }
  li { display: grid; grid-template-columns: 10px minmax(0, 1fr) auto; gap: 14px; align-items: center; padding: 11px 0; border-top: 1px solid var(--line); }
  li:first-child { border-top: 0; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--ok); }
  .dot.down { background: var(--danger); }
  .dot.unknown { background: #636366; }
  .nm { display: grid; min-width: 0; }
  .nm small { color: var(--muted); font-size: 12.5px; }
  .ms { color: var(--muted); font-size: 13px; font-variant-numeric: tabular-nums; }
</style>
