<script lang="ts">
  import { api, post } from '$lib/api';
  import { toast } from '$lib/ui/toast.svelte';
  import { clock, size } from '$lib/fmt';
  import { percent } from '$lib/library';
  import Icon from '$lib/ui/Icon.svelte';
  import Metrics from './Metrics.svelte';
  import ServerMode from './ServerMode.svelte';
  import Scheme from './Scheme.svelte';
  import type { IconName } from '$lib/ui/icons';
  import type { Health, Status } from './types';

  // Обзор для хоста: сводка «всё ли в порядке», чек-лист с пояснениями и ссылками, куда идти,
  // цифры с объяснением и кто сейчас смотрит. Показатели обновляются раз в 5 секунд без
  // перестройки страницы: меняются только числа (размеры плиток и строк заданы заранее).
  let { status, reload }: { status: Status; reload: () => void } = $props();
  let copied = $state(false);

  $effect(() => {
    const t = setInterval(() => document.visibilityState === 'visible' && reload(), 5000);
    return () => clearInterval(t);
  });

  let health = $state<Health | null>(null);
  let copiedErrors = $state(false);
  $effect(() => {
    const load = () => api<Health>('/api/admin/health').then((h) => (health = h), () => {});
    load();
    const t = setInterval(() => document.visibilityState === 'visible' && load(), 30000);
    return () => clearInterval(t);
  });
  const when = (ms: number) => new Date(ms).toLocaleString('ru-RU', { dateStyle: 'short', timeStyle: 'medium' });
  const upFor = (s: number) => (s >= 86400 ? `${Math.floor(s / 86400)} д ${Math.floor((s % 86400) / 3600)} ч` : s >= 3600 ? `${Math.floor(s / 3600)} ч ${Math.floor((s % 3600) / 60)} мин` : `${Math.floor(s / 60)} мин`);
  async function copyErrors() {
    if (!health) return;
    const text = [`Nova ${health.version}, работает ${upFor(health.uptimeSec)}`, ...health.errors.map((e) => `${when(e.at)} ${e.level} ${e.target}: ${e.message}`)].join('\n');
    await navigator.clipboard?.writeText(text).catch(() => {});
    copiedErrors = true;
    setTimeout(() => (copiedErrors = false), 1500);
  }

  async function copy() {
    if (!status.publicUrl) return;
    await navigator.clipboard?.writeText(status.publicUrl).catch(() => {});
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  type Check = { ok: boolean | null; icon: IconName; title: string; text: string; href?: string; action?: string };
  const GB = 1024 ** 3;
  const diskLow = $derived(status.system?.diskFree != null && status.system.diskFree < 10 * GB);
  const checks = $derived<Check[]>([
    {
      ok: true,
      icon: 'pulse',
      title: 'Сервер работает',
      text: `Nova ${status.version} раздаёт фильмы с этого компьютера. Пока он включён, зрители могут смотреть.`,
    },
    {
      ok: !!status.publicUrl,
      icon: 'globe',
      title: status.publicUrl ? 'Доступ из интернета включён' : 'Доступ из интернета выключен',
      text: status.publicUrl
        ? 'Друзья заходят по постоянному адресу ниже - его можно отправить один раз.'
        : 'Сейчас Nova видно только в вашей домашней сети. Включите доступ, чтобы смотрели друзья из других мест.',
      href: '?tab=access',
      action: status.publicUrl ? 'Настроить' : 'Включить',
    },
    {
      ok: status.ffmpeg && status.ffprobe,
      icon: 'film',
      title: status.ffmpeg && status.ffprobe ? 'ffmpeg на месте' : 'Нет ffmpeg',
      text:
        status.ffmpeg && status.ffprobe
          ? 'MKV, AVI, несколько озвучек и субтитры играют в любом браузере, качество подстраивается под интернет.'
          : 'Без ffmpeg не играют MKV/AVI и фильмы с несколькими озвучками. Укажите путь в «Настройках».',
      href: status.ffmpeg && status.ffprobe ? undefined : '?tab=settings',
      action: 'Открыть настройки',
    },
    {
      ok: status.scan.error ? false : status.titles > 0 ? true : null,
      icon: 'folder',
      title: status.scan.error ? 'Ошибка в папках' : status.titles ? 'Папки с фильмами подключены' : 'Фильмов пока нет',
      text: status.scan.error
        ? status.scan.error
        : status.scan.running && status.scan.progress
          ? `Идёт проверка папок: проверено файлов ${status.scan.progress}.`
          : status.titles
          ? `${status.titles} фильмов и сериалов, ${status.files} файлов (${size(status.size)}). Новое в папках появляется само.`
          : 'Добавьте папку с фильмами или перетащите файлы в окно Nova - они появятся на сайте сами.',
      href: '?tab=folders',
      action: status.titles ? 'Папки' : 'Добавить папку',
    },
    {
      ok: status.meta.error ? false : status.metaPending || status.unmatched ? null : true,
      icon: 'image',
      title: status.meta.error
        ? 'Описания не загружаются'
        : status.metaPending
          ? 'Загружаем описания'
          : status.unmatched
            ? 'Есть фильмы без описания'
            : 'Описания загружены',
      text: status.meta.error
        ? status.meta.error
        : status.metaPending
          ? `Осталось ${status.metaPending}: постеры, кадры, актёры и сюжет по-русски подтягиваются в фоне.`
          : status.unmatched
            ? `У ${status.unmatched} фильмов описание не нашлось: откройте «Каталог» и выберите верное.`
            : 'У всех фильмов есть постер, описание и оценки.',
      href: status.unmatched && !status.metaPending ? '?tab=catalog&filter=none' : '?tab=catalog',
      action: 'Каталог',
    },
    {
      ok: status.system?.supported ? !diskLow : null,
      icon: 'download',
      title: diskLow ? 'Мало места на диске' : 'Место на диске',
      text:
        status.system?.diskFree != null
          ? `Свободно ${size(status.system.diskFree)} из ${size(status.system.diskTotal ?? 0)}.${diskLow ? ' Нужно хотя бы 10 ГБ - на копии и кэш видео.' : ''}`
          : 'Свободное место покажется после первого замера.',
    },
    {
      ok: status.update?.state === 'available' ? null : true,
      icon: 'refresh',
      title: status.update?.state === 'available' ? `Есть новая версия ${status.update.latest ?? ''}` : 'Версия',
      text: status.update?.state === 'available' ? 'Установщик обновит Nova, фильмы и люди сохранятся.' : `Установлена ${status.version}.`,
      href: '?tab=version',
      action: 'Подробнее',
    },
  ]);
  const cpuPct = $derived(Math.round(status.system?.cpu ?? 0));
  const memPct = $derived(status.system?.memoryUsed && status.system.memoryTotal ? Math.round((status.system.memoryUsed / status.system.memoryTotal) * 100) : 0);
  const diskPct = $derived(status.system?.diskFree != null && status.system.diskTotal ? Math.round(100 - (status.system.diskFree / status.system.diskTotal) * 100) : 0);
  const problems = $derived(checks.filter((c) => c.ok === false).length);
  const waiting = $derived(checks.filter((c) => c.ok === null).length);
  async function scanNow() {
    try {
      await post('/api/admin/scan');
      toast('Папки проверяются');
      reload?.();
    } catch {
      toast('Не удалось запустить проверку папок');
    }
  }
</script>

<ServerMode {status} {reload} />

<section class="summary" class:bad={problems > 0} aria-label="Сводка">
  <span class="dot" aria-hidden="true"><Icon name={problems ? 'info' : 'check'} size={30} /></span>
  <div class="msg">
    <h2>{problems ? `Нужно внимание: ${problems}` : waiting ? 'Всё работает, кое-что ещё в процессе' : 'Всё в порядке'}</h2>
    <p class="muted">
      {problems
        ? 'Ниже отмечено, что не так и куда нажать, чтобы исправить.'
        : 'Nova раздаёт фильмы, зрители могут смотреть. Ниже - подробности по каждой части.'}
    </p>
  </div>
  <button class="btn primary scan" onclick={scanNow} disabled={status.scan.running}><Icon name="refresh" size={16} /> {status.scan.running ? 'Сканируем…' : 'Сканировать папки'}</button>
</section>

<Scheme {status} />

<Metrics {status} />

<section class="card pane" aria-label="Состояние">
  <h3><Icon name="check" size={18} /> Состояние</h3>
  <ul class="checks">
    {#each checks as c (c.title + c.icon)}
      <li class:ok={c.ok === true} class:bad={c.ok === false} class:wait={c.ok === null}>
        <span class="ic"><Icon name={c.icon} size={18} /></span>
        <div>
          <b>{c.title}</b>
          <p>{c.text}</p>
        </div>
        {#if c.href}<a class="btn sm" href={c.href} data-sveltekit-replacestate data-sveltekit-noscroll>{c.action}</a>{/if}
      </li>
    {/each}
  </ul>
</section>

<section class="card pane" aria-label="Адрес для зрителей">
  <h3><Icon name="link" size={18} /> Адрес для зрителей</h3>
  {#if status.publicUrl}
    <p class="muted small">Отправьте этот адрес друзьям. Он не меняется; работает, пока компьютер включён.</p>
    <div class="url">
      <code>{status.publicUrl}</code>
      <button class="btn sm" onclick={copy}><Icon name={copied ? 'check' : 'copy'} size={15} /> {copied ? 'Скопировано' : 'Копировать'}</button>
    </div>
  {:else}
    <p class="muted">Доступ из интернета выключен. Вкладка «Доступ» - включить в пару нажатий.</p>
  {/if}
</section>

{#if status.system?.supported}
  <section class="card pane" aria-label="Компьютер хоста">
    <h3><Icon name="pulse" size={18} /> Компьютер хоста</h3>
    <p class="muted small">Насколько занят компьютер, который раздаёт фильмы. Высокая загрузка процессора - обычно перекодирование видео для слабого интернета у зрителя.</p>
    <div class="meters">
      {@render ring('Процессор', cpuPct, status.system.cpu == null ? '…' : `${cpuPct}%`, cpuPct >= 90)}
      {@render ring('Память', memPct, status.system.memoryUsed == null ? '-' : size(status.system.memoryUsed), memPct >= 90)}
      {@render ring('Диск занят', diskPct, status.system.diskFree == null || !status.system.diskTotal ? '-' : `${diskPct}%`, diskLow)}
    </div>
    <p class="faint small">Обновляется каждые 5 секунд, пока вкладка открыта.</p>
  </section>
{/if}

<section class="card pane" aria-label="Сейчас смотрят">
  <h3><Icon name="play" size={18} /> Сейчас смотрят</h3>
  {#if status.watching.length}
    <ul class="watching">
      {#each status.watching as w, i (i)}
        <li>
          <b>{w.user}</b>
          <span class="what">{w.title}{w.episode ? ` · ${w.episode}` : ''}</span>
          <span class="time">{clock(w.position)} / {clock(w.duration)}</span>
          <span class="bar"><i style:transform="scaleX({percent(w.position, w.duration) / 100})"></i></span>
          {#if w.net}
            <span class="net" class:slow={w.net.stalls > 0}>
              {w.net.mbit !== null ? `${w.net.mbit} Мбит/с` : 'скорость не замерена'} · {w.net.quality === 'orig' ? 'оригинал' : w.net.quality === 'auto' ? 'автокачество' : w.net.quality}
              {#if w.net.stalls > 0} · остановок: {w.net.stalls}{/if}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted">Никто не смотрит. Когда кто-то включит фильм, он появится здесь с местом просмотра, скоростью интернета и качеством.</p>
  {/if}
</section>

{#snippet ring(label: string, pct: number, text: string, bad: boolean)}
  <div class="ring" class:low={bad}>
    <svg viewBox="0 0 44 44" aria-hidden="true">
      <circle class="track" cx="22" cy="22" r="18" />
      <circle class="arc" cx="22" cy="22" r="18" pathLength="100" stroke-dasharray="{Math.min(100, pct)} 100" />
    </svg>
    <b>{text}</b>
    <span>{label}</span>
  </div>
{/snippet}

{#if health}
  <section class="card pane" aria-label="Недавние ошибки">
    <h3><Icon name="flag" size={18} /> Недавние ошибки</h3>
    <p class="muted small">Версия {health.version}, работает {upFor(health.uptimeSec)}, база {size(health.dbBytes)}. Подробный журнал лежит в папке данных, в «logs».</p>
    <p class="small">
      Источники видео:
      {#each health.sources as s, i (s.site)}{i ? ', ' : ' '}{s.label} - {s.state === 'ok' ? 'отвечает' : s.state === 'down' ? 'выключен на 10 минут' : 'ещё не проверялся'}{/each}.
    </p>
    {#if health.errors.length}
      <ul class="errors">
        {#each health.errors as e, i (i)}
          <li class:err={e.level === 'ERROR'}><time>{when(e.at)}</time><b>{e.level === 'ERROR' ? 'Ошибка' : 'Предупреждение'}</b><span>{e.target}: {e.message}</span></li>
        {/each}
      </ul>
      <button class="btn sm" onclick={copyErrors}><Icon name={copiedErrors ? 'check' : 'copy'} size={15} /> {copiedErrors ? 'Скопировано' : 'Скопировать для отчёта'}</button>
    {:else}
      <p class="muted">Ошибок нет.</p>
    {/if}
  </section>
{/if}

<style>
  .errors {
    display: grid;
    gap: 8px;
    margin: 12px 0 14px;
    padding: 0;
    list-style: none;
    font-size: 13px;
  }
  .errors li {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    gap: 4px 12px;
    align-items: baseline;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--line);
  }
  .errors time {
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .errors b {
    color: var(--muted);
  }
  .errors li.err b {
    color: var(--danger);
  }
  .errors span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .summary {
    display: flex;
    align-items: center;
    gap: 22px;
    margin-bottom: 18px;
    padding: 26px 30px;
    border-radius: 30px;
    background: linear-gradient(100deg, #0f2a1a, #0a140e);
    border: 1px solid #1f6b3d;
  }
  .summary.bad {
    background: linear-gradient(100deg, #2a1015, #140a0c);
    border-color: rgba(255, 92, 108, 0.45);
  }
  .msg { flex: 1; min-width: 0; }
  .summary h2 {
    margin: 0 0 4px;
    font-size: clamp(20px, 2.2vw, 28px);
    letter-spacing: -0.02em;
  }
  .summary p {
    margin: 0;
  }
  .dot {
    flex: none;
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: #16a34a;
    color: #fff;
  }
  .summary.bad .dot {
    background: var(--danger);
  }
  .scan { flex: none; }
  @media (max-width: 640px) {
    .summary { flex-wrap: wrap; padding: 20px; gap: 14px; }
    .dot { width: 48px; height: 48px; }
    .scan { width: 100%; justify-content: center; }
  }
  .pane {
    margin-bottom: 16px;
    padding: 18px 20px;
  }
  .pane h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 12px;
    font-size: 16px;
  }
  .small {
    font-size: 13px;
    margin: 0 0 12px;
  }
  .checks {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .checks li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    padding: 12px 4px;
    border-top: 1px solid var(--line);
  }
  .checks li:first-child {
    border-top: 0;
  }
  .checks .ic {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
  }
  .checks .ok .ic {
    background: rgba(48, 209, 88, 0.14);
    color: #30d158;
  }
  .checks .bad .ic {
    background: rgba(255, 92, 108, 0.14);
    color: var(--danger);
  }
  .checks .wait .ic {
    background: rgba(255, 255, 255, 0.08);
    color: var(--muted);
  }
  .checks b {
    display: block;
    font-size: 14px;
  }
  .checks p {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .url {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .url code {
    min-width: 0;
    padding: 8px 12px;
    border-radius: 10px;
    background: #000;
    overflow-wrap: anywhere;
  }
  .meters {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 180px), 1fr));
    gap: 12px;
    margin-bottom: 10px;
  }
  /* Кольца загрузки: белое - обычно, красное - когда пора вмешаться (диск почти полон, процессор или память на пределе). */
  .ring {
    position: relative;
    display: grid;
    grid-template-columns: 76px minmax(0, 1fr);
    grid-template-rows: auto auto;
    align-content: center;
    column-gap: 14px;
    padding: 14px 16px;
    border-radius: 18px;
    background: #ffffff0a;
  }
  .ring svg {
    grid-row: 1 / 3;
    width: 76px;
    height: 76px;
    transform: rotate(-90deg);
    fill: none;
    stroke-width: 4.4;
  }
  .ring .track {
    stroke: #ffffff1c;
  }
  .ring .arc {
    stroke: #fff;
    stroke-linecap: round;
    transition: stroke-dasharray 0.6s ease;
  }
  .ring.low .arc {
    stroke: var(--danger);
  }
  .ring b {
    align-self: end;
    font-size: 22px;
    font-variant-numeric: tabular-nums;
  }
  .ring.low b {
    color: var(--danger);
  }
  .ring span {
    align-self: start;
    font-size: 12px;
    color: var(--muted);
  }
  .watching {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .watching li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: baseline;
    gap: 4px 12px;
  }
  .watching .what {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--muted);
  }
  .watching .time {
    font-size: 12px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  /* Скорость и качество у зрителя: остановки - заметнее. */
  .watching .net {
    grid-column: 1 / -1;
    font-size: 12px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .watching .net.slow {
    color: var(--danger);
  }
  .watching .bar {
    grid-column: 1 / -1;
    height: 3px;
    overflow: hidden;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.12);
  }
  .watching .bar i {
    display: block;
    height: 100%;
    background: #fff;
    transform-origin: left;
    transition: transform 0.6s ease;
  }
  @media (max-width: 640px) {
    .checks li {
      grid-template-columns: auto minmax(0, 1fr);
    }
    .checks li .btn {
      grid-column: 2;
      justify-self: start;
    }
  }
</style>
