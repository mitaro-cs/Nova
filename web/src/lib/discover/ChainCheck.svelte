<script lang="ts">
  // Живая схема цепочки воспроизведения: от найденной ссылки до первого байта видео.
  // Шаги приходят от сервера, здесь только выбор адреса и раскрытие объяснений.
  import { api, errorText, post } from '$lib/api';
  import Icon from '$lib/ui/Icon.svelte';

  type Step = {
    key: string;
    title: string;
    note: string;
    status: 'ok' | 'fail' | 'skip';
    ms: number;
    detail: string;
  };
  type Chain = { ok: boolean; verdict: string; host: string | null; steps: Step[] };
  type Found = { url: string; label?: string; site?: string; title?: string };

  let mode = $state<'find' | 'manual'>('find');
  let query = $state('');
  let manual = $state('');
  let season = $state('');
  let episode = $state('');
  let voice = $state('');
  let results = $state<Found[]>([]);
  let searching = $state(false);
  let running = $state(false);
  let chain = $state<Chain | null>(null);
  let error = $state('');
  let picked = $state('');
  let active = $state<string | null>(null);

  async function find(e: SubmitEvent) {
    e.preventDefault();
    const q = query.trim();
    if (!q || searching) return;
    searching = true;
    error = '';
    results = [];
    chain = null;
    try {
      results = await api<Found[]>(`/api/links/search?q=${encodeURIComponent(q)}`);
    } catch (x) {
      error = errorText(x);
    } finally {
      searching = false;
    }
  }

  function manualUrl(): string {
    let u = manual.trim();
    if (!u) return '';
    const parts: string[] = [];
    // type="number" в Svelte связывает число, а не строку - приводим к строке перед trim.
    const s = String(season ?? '').trim();
    const e = String(episode ?? '').trim();
    const v = String(voice ?? '').trim();
    if (s) parts.push(`nova-season=${encodeURIComponent(s)}`);
    if (e) parts.push(`episode=${encodeURIComponent(e)}`);
    if (v) parts.push(`nova-voice=${encodeURIComponent(v)}`);
    // Дописываем к существующему фрагменту, чтобы не потерять nova-origin (нужен каталогам).
    if (parts.length) u += (u.includes('#') ? '&' : '#') + parts.join('&');
    return u;
  }

  async function checkManual(e: SubmitEvent) {
    e.preventDefault();
    const u = manualUrl();
    if (u) await run(u);
  }

  async function run(url: string) {
    if (running) return;
    picked = url;
    running = true;
    error = '';
    chain = null;
    active = null;
    try {
      chain = await post<Chain>('/api/stream/diagnose', { url });
      const failed = chain.steps.find((s) => s.status === 'fail');
      active = failed?.key ?? null;
    } catch (x) {
      error = errorText(x);
    } finally {
      running = false;
    }
  }

  function switchMode(next: 'find' | 'manual') {
    mode = next;
    error = '';
    chain = null;
    active = null;
  }

  const stepIcon = (s: Step) => (s.status === 'ok' ? 'check' : s.status === 'fail' ? 'ban' : 'dots');
  const nameOf = (f: Found) => f.label || f.title || f.site || 'Видео';
</script>

<section class="chain" aria-labelledby="chain-title">
  <div class="intro">
    <div>
      <h2 id="chain-title">Как работает видео</h2>
      <p>Найдите фильм или вставьте ссылку - и посмотрите, где именно цепочка ломается.</p>
    </div>
    <Icon name="pulse" size={30} />
  </div>

  <div class="tabs" role="tablist" aria-label="Способ проверки">
    <button class:on={mode === 'find'} role="tab" aria-selected={mode === 'find'} onclick={() => switchMode('find')}>По названию</button>
    <button class:on={mode === 'manual'} role="tab" aria-selected={mode === 'manual'} onclick={() => switchMode('manual')}>По ссылке</button>
  </div>

  {#if mode === 'find'}
    <form class="ask" onsubmit={find} role="search">
      <Icon name="search" size={20} />
      <input type="search" bind:value={query} placeholder="Название фильма или сериала" aria-label="Найти видео для проверки" />
      <button class="btn" type="submit" disabled={searching || !query.trim()}>{searching ? 'Ищем...' : 'Найти'}</button>
    </form>
  {:else}
    <form class="ask" onsubmit={checkManual}>
      <Icon name="link" size={20} />
      <input type="url" bind:value={manual} placeholder="https://сайт/фильм или плеер" aria-label="Ссылка на видео" />
      <button class="btn" type="submit" disabled={running || !manual.trim()}>Проверить</button>
    </form>
    <div class="tune" aria-label="Выбор серии и озвучки">
      <label>Сезон <input type="number" min="1" bind:value={season} placeholder="1" /></label>
      <label>Серия <input type="number" min="1" bind:value={episode} placeholder="1" /></label>
      <label>Озвучка (id) <input type="text" bind:value={voice} placeholder="89" /></label>
    </div>
  {/if}

  {#if error}
    <p class="err"><Icon name="offline" size={18} /> {error}</p>
  {/if}

  {#if mode === 'find' && results.length}
    <div class="found" aria-label="Найденные видео">
      {#each results as f, i (f.url + i)}
        <button class="pick" class:on={picked === f.url} onclick={() => run(f.url)}>
          <span class="pick-name">{nameOf(f)}</span>
          <span class="pick-site">{f.site ?? 'источник'}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if running}
    <p class="progress"><span class="spin" aria-hidden="true"></span> Проверяем цепочку...</p>
  {/if}

  {#if chain}
    <div class="verdict" class:bad={!chain.ok}>
      <Icon name={chain.ok ? 'check' : 'ban'} size={20} />
      <div>
        <strong>{chain.verdict}</strong>
        {#if chain.host}<span class="host">Сайт потока: {chain.host}</span>{/if}
      </div>
    </div>
    <ol class="steps">
      {#each chain.steps as s, i (s.key)}
        <li class={s.status}>
          <button
            class="step"
            onclick={() => (active = active === s.key ? null : s.key)}
            aria-expanded={active === s.key || s.status === 'fail'}
          >
            <span class="dot"><Icon name={stepIcon(s)} size={15} /></span>
            <span class="line">
              <span class="top">
                <b>{i + 1}. {s.title}</b>
                {#if s.ms}<span class="ms">{s.ms} мс</span>{/if}
              </span>
              {#if active === s.key || s.status === 'fail'}
                <span class="more">
                  <span class="note">{s.note}</span>
                  <span class="detail">{s.detail}</span>
                </span>
              {/if}
            </span>
            <span class="caret"><Icon name="down" size={16} /></span>
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .chain {
    margin: 24px 0 8px;
    padding: 24px;
    border: 1px solid var(--line);
    border-radius: 22px;
    background: var(--surface-2);
  }
  .intro {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    color: var(--muted);
  }
  .intro h2 {
    margin: 0;
    color: var(--text);
    font-size: 21px;
  }
  .intro p {
    margin: 5px 0 0;
    font-size: 14px;
  }
  .tabs {
    display: flex;
    gap: 6px;
    margin-top: 18px;
  }
  .tabs button {
    padding: 7px 14px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .tabs button.on {
    border-color: var(--text);
    background: var(--surface-3);
    color: var(--text);
  }
  .ask {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    padding: 6px 6px 6px 14px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--surface-3);
  }
  .ask input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--text);
    font: inherit;
  }
  .ask input::placeholder {
    color: var(--faint, var(--muted));
  }
  .ask .btn {
    min-height: 40px;
    padding: 0 20px;
  }
  .ask .btn:disabled {
    opacity: 0.5;
  }
  .tune {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 12px;
  }
  .tune label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 13px;
  }
  .tune input {
    width: 78px;
    padding: 7px 10px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface-3);
    color: var(--text);
    font: inherit;
  }
  .err,
  .progress {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 14px 0 0;
    color: var(--muted);
    font-size: 14px;
  }
  .err {
    color: #ff8a8a;
  }
  .spin {
    width: 14px;
    height: 14px;
    border: 2px solid var(--line-strong);
    border-top-color: var(--text);
    border-radius: 50%;
    animation: turn 0.8s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  .found {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
  }
  .pick {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    max-width: 280px;
    padding: 9px 14px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--surface-3);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .pick.on {
    border-color: var(--text);
  }
  .pick-name {
    overflow: hidden;
    max-width: 100%;
    font-size: 14px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pick-site {
    color: var(--muted);
    font-size: 12px;
  }
  .verdict {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-top: 18px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--surface-3);
    font-size: 14px;
  }
  .verdict.bad {
    border-color: rgba(255, 110, 110, 0.4);
  }
  .verdict strong {
    font-weight: 600;
  }
  .host {
    display: block;
    margin-top: 3px;
    color: var(--muted);
    font-size: 12px;
  }
  .steps {
    margin: 14px 0 0;
    padding: 0;
    list-style: none;
  }
  .steps li {
    position: relative;
    padding-left: 18px;
  }
  .steps li:not(:last-child)::before {
    position: absolute;
    top: 34px;
    bottom: -4px;
    left: 11px;
    width: 1px;
    background: var(--line);
    content: '';
  }
  .step {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    padding: 8px 4px;
    border: 0;
    background: none;
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .dot {
    display: grid;
    flex: 0 0 26px;
    width: 26px;
    height: 26px;
    place-items: center;
    border: 1px solid var(--line-strong);
    border-radius: 50%;
    color: var(--muted);
  }
  .ok .dot {
    border-color: rgba(120, 220, 150, 0.6);
    color: #8ce0a0;
  }
  .fail .dot {
    border-color: rgba(255, 110, 110, 0.6);
    color: #ff8a8a;
  }
  .line {
    flex: 1;
    min-width: 0;
  }
  .top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  .top b {
    font-size: 14px;
    font-weight: 600;
  }
  .ms {
    color: var(--muted);
    font-size: 12px;
  }
  .more {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 6px;
    font-size: 13px;
  }
  .note {
    color: var(--muted);
  }
  .detail {
    overflow-wrap: anywhere;
    color: var(--text);
  }
  .caret {
    margin-top: 5px;
    color: var(--muted);
  }
  @media (max-width: 640px) {
    .chain {
      padding: 18px 16px;
    }
    .intro h2 {
      font-size: 18px;
    }
    .ask {
      padding-left: 12px;
    }
    .ask .btn {
      padding: 0 14px;
    }
  }
</style>
