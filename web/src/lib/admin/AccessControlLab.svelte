<script lang="ts">
  import Icon from '$lib/ui/Icon.svelte';

  type Demo = 'baseline' | 'spoofed-forwarded' | 'spoofed-agent' | 'retry' | 'trusted-proxy';
  type Result = { title: string; status: number; decision: string; explanation: string; unsafe: boolean };
  let result = $state<Result | null>(null);
  const sourceExamples = ['LordFilm', 'Lordserials', 'YummyAnime'];

  // Учебная модель целиком в браузере: никакие сетевые запросы не выполняются.
  function run(demo: Demo) {
    const examples: Record<Demo, Result> = {
      baseline: {
        title: 'Обычный запрос к закрытому ресурсу', status: 403,
        decision: 'Отказано по политике', explanation: 'Запрос понятен серверу, но клиент не имеет выданного разрешения.', unsafe: false,
      },
      'spoofed-forwarded': {
        title: 'Модель ошибки: сервер доверяет клиентскому X-Forwarded-For', status: 200,
        decision: 'Ложный доступ в уязвимой модели', explanation: 'Только в этой локальной симуляции сервер ошибочно принимает переданный клиентом адрес за доверенный. Исправленная модель ниже игнорирует такой заголовок и отвечает 403.', unsafe: true,
      },
      'spoofed-agent': {
        title: 'Подмена User-Agent', status: 403,
        decision: 'Доступ не изменился', explanation: 'Название клиента не является учётными данными и не выдаёт разрешение.', unsafe: false,
      },
      retry: {
        title: 'Повтор запроса', status: 403,
        decision: 'Доступ не изменился', explanation: 'Повтор не меняет политику доступа и может лишь создать лишнюю нагрузку.', unsafe: false,
      },
      'trusted-proxy': {
        title: 'Исправленная модель: доверенный прокси', status: 403,
        decision: 'Отказано корректно', explanation: 'Приложение использует адрес соединения, полученный от настроенного доверенного прокси, а не произвольное значение от клиента.', unsafe: false,
      },
    };
    result = examples[demo];
  }

  function runSourceExample(name: string) {
    result = {
      title: `${name}: учебный ответ источника`,
      status: 403,
      decision: 'Имитация отказа доступа',
      explanation: 'Это заранее заданный пример для занятия. Nova не отправляла запрос на сайт и не проверяла его реальный статус.',
      unsafe: false,
    };
  }
</script>

<section class="lab" aria-labelledby="access-lab-title">
  <div class="head">
    <span class="badge"><Icon name="pulse" size={18} /></span>
    <div><p class="eyebrow">УЧЕБНАЯ СИМУЛЯЦИЯ · БЕЗ СЕТЕВЫХ ЗАПРОСОВ</p><h3 id="access-lab-title">Почему нельзя «обходить» 403</h3><p class="muted">Посмотри, как ошибка в собственной политике доступа создаёт ложный пропуск и как сервер должен принимать решение.</p></div>
  </div>

  <div class="actions" aria-label="Сценарии симуляции">
    <button class="btn sm" onclick={() => run('baseline')}>Обычный запрос</button>
    <button class="btn sm warning" onclick={() => run('spoofed-forwarded')}>Уязвимая модель заголовка</button>
    <button class="btn sm" onclick={() => run('spoofed-agent')}>Подмена User-Agent</button>
    <button class="btn sm" onclick={() => run('retry')}>Повтор запроса</button>
    <button class="btn sm primary" onclick={() => run('trusted-proxy')}>Исправленная модель</button>
  </div>

  <div class="sources" aria-label="Учебные примеры источников">
    <span class="muted">Фиктивные кейсы источников:</span>
    {#each sourceExamples as name (name)}<button class="btn sm" onclick={() => runSourceExample(name)}>{name} · 403</button>{/each}
  </div>

  {#if result}
    <div class="result" class:unsafe={result.unsafe} role="status" aria-live="polite">
      <span class="status">HTTP {result.status}</span>
      <div><b>{result.title}</b><strong>{result.decision}</strong><p>{result.explanation}</p></div>
    </div>
  {:else}
    <p class="placeholder">Выбери сценарий — Nova покажет результат искусственной модели.</p>
  {/if}

  <p class="footnote"><Icon name="info" size={15} />Это объяснение для защиты своего сервера. Результат «200» существует только в симуляции и не отправляется ни на один внешний адрес.</p>
</section>

<style>
  .lab{display:grid;gap:15px;padding:20px;border:1px solid var(--line);border-radius:20px;background:var(--surface-2)}
  .head{display:flex;align-items:flex-start;gap:12px}.badge{display:grid;place-items:center;width:38px;height:38px;flex:none;border-radius:12px;background:var(--surface-3)}
  .eyebrow{margin:0 0 5px;font-size:9px;letter-spacing:.12em;font-weight:700;color:var(--muted)}h3{font-size:16px;margin:0}.head .muted{margin:5px 0 0;max-width:720px;font-size:12px;line-height:1.5}
  .actions{display:flex;flex-wrap:wrap;gap:8px}.warning{border-color:#9b7654;color:#ffd6ad}
  .sources{display:flex;align-items:center;flex-wrap:wrap;gap:8px}.sources>span{font-size:11px}
  .result{display:grid;grid-template-columns:auto 1fr;align-items:start;gap:12px;padding:14px;border:1px solid var(--line);border-radius:14px;background:var(--surface)}
  .result.unsafe{border-color:#a96d48;background:color-mix(in srgb,#8b451f 12%,var(--surface))}.status{font-weight:800;font-variant-numeric:tabular-nums;color:#f4bd68}.result:not(.unsafe) .status{color:var(--muted)}
  .result div{display:grid;gap:4px}.result strong{font-size:12px}.result p,.placeholder{margin:0;color:var(--muted);font-size:12px;line-height:1.5}.placeholder{padding:4px 0}
  .footnote{display:flex;gap:8px;align-items:flex-start;margin:0;color:var(--muted);font-size:11px;line-height:1.5}
</style>
