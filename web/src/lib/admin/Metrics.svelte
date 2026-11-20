<script lang="ts">
  import { api, errorText, post } from '$lib/api';
  import { count, plural, size } from '$lib/fmt';
  import { motionOn } from '$lib/motion';
  import { ask } from '$lib/ui/dialogs.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';
  import { toast } from '$lib/ui/toast.svelte';
  import type { Metrics, Status } from './types';

  // Показатели «Обзора» хоста. Цветов-«украшений» нет (решение владельца, 2 ноября): всё белое и серое, а цвет
  // значит только состояние - зелёный «работает», красный «ошибка». Место на диске - кольцо, качество файлов -
  // горизонтальные полосы, активность - столбики и сетка по часам. Данные считаются на сервере из базы и папок
  // (`GET /api/admin/metrics`) и обновляются раз в 30 секунд.
  let { status }: { status: Status } = $props();
  let m = $state<Metrics | null>(null);
  let failed = $state('');
  let busy = $state('');

  const load = () =>
    api<Metrics>(`/api/admin/metrics?tz=${-new Date().getTimezoneOffset()}`).then(
      (r) => ((m = r), (failed = '')),
      (e) => (failed = errorText(e)),
    );
  $effect(() => {
    void load();
    const t = setInterval(() => document.visibilityState === 'visible' && void load(), 30000);
    return () => clearInterval(t);
  });

  /** Число, которое плавно «набегает» до нового значения (если анимации включены). */
  function tick(node: HTMLElement, value: number) {
    let shown = 0;
    let raf = 0;
    const show = (v: number) => (node.textContent = Math.round(v).toLocaleString('ru-RU'));
    const run = (to: number) => {
      cancelAnimationFrame(raf);
      if (!motionOn()) return show((shown = to));
      const from = shown;
      const start = performance.now();
      const step = (now: number) => {
        const k = Math.min(1, (now - start) / 700);
        shown = from + (to - from) * (1 - Math.pow(1 - k, 3));
        show(shown);
        if (k < 1) raf = requestAnimationFrame(step);
      };
      raf = requestAnimationFrame(step);
    };
    run(value);
    return { update: run, destroy: () => cancelAnimationFrame(raf) };
  }

  const hours = $derived(m ? Math.round(m.library.seconds / 3600) : 0);
  const fmtHours = (h: number) => (h >= 24 ? `${Math.floor(h / 24)} д ${h % 24} ч` : `${h} ч`);
  const days = $derived(m?.activity.days ?? []);
  const peak = $derived(Math.max(1, ...days.map((d) => d.views)));
  const hourPeak = $derived(Math.max(1, ...(m?.activity.hours ?? [0])));
  const weekViews = $derived(days.slice(-7).reduce((a, d) => a + d.views, 0));
  const dayName = (ms: number) => new Date(ms).toLocaleDateString('ru-RU', { day: 'numeric', month: 'long' });
  const dd = (ms: number) => new Date(ms).getDate();
  const topMax = $derived(Math.max(1, ...(m?.activity.top ?? []).map((t) => t.viewers)));
  const busyHour = $derived.by(() => {
    const h = m?.activity.hours ?? [];
    const max = Math.max(...h, 0);
    return max ? h.indexOf(max) : -1;
  });

  type Part = { key: string; label: string; bytes: number; tone: string; note: string };
  const parts = $derived<Part[]>(
    m
      ? [
          { key: 'hls', label: 'Кэш видео', bytes: m.disk.hls, tone: '#ffffff', note: 'перекодированное для слабого интернета' },
          { key: 'copy', label: 'Копии «Оригинал»', bytes: m.disk.copy, tone: '#d4d4d8', note: 'быстрая перепаковка без потерь' },
          { key: 'images', label: 'Постеры и кадры', bytes: m.disk.images, tone: '#a1a1aa', note: 'картинки фильмов и аватарки' },
          { key: 'db', label: 'База', bytes: m.disk.db, tone: '#83838c', note: 'люди, отметки, отзывы, настройки' },
          { key: 'backups', label: 'Копии базы', bytes: m.disk.backups, tone: '#63636b', note: 'перед обновлением и ручные' },
          { key: 'other', label: 'Прочее', bytes: m.disk.cacheOther + m.disk.logs, tone: '#46464d', note: 'кадры перемотки, журнал' },
        ]
      : [],
  );
  const partsTotal = $derived(parts.reduce((a, p) => a + p.bytes, 0));
  const cacheBytes = $derived((m?.disk.hls ?? 0) + (m?.disk.copy ?? 0));

  const QUALITY: Record<string, { label: string }> = {
    '4k': { label: '4K' },
    '1440': { label: '1440p' },
    '1080': { label: '1080p' },
    '720': { label: '720p' },
    sd: { label: 'до 720p' },
  };
  const resolutions = $derived(
    (m?.library.resolutions ?? [])
      .map((r) => ({ ...r, ...(QUALITY[r.q] ?? { label: r.q }) }))
      .sort((a, b) => Object.keys(QUALITY).indexOf(a.q) - Object.keys(QUALITY).indexOf(b.q)),
  );
  const resTotal = $derived(resolutions.reduce((a, r) => a + r.files, 0));
  const resMax = $derived(Math.max(1, ...resolutions.map((r) => r.files)));

  // Кольцо «куда уходит место»: доли данных Nova по видам, от самой большой к самой малой.
  const RING = 2 * Math.PI * 40;
  const slices = $derived.by(() => {
    let from = 0;
    return parts
      .filter((p) => p.bytes > 0)
      .map((p) => {
        const share = partsTotal ? p.bytes / partsTotal : 0;
        const slice = { key: p.key, tone: p.tone, len: Math.max(0, share * RING - 2), from };
        from += share * RING;
        return slice;
      });
  });

  async function clearCache() {
    if (!(await ask(`Удалить кэш видео (${size(cacheBytes)})? Ничего нужного не пропадёт: при следующем просмотре видео соберётся заново.`, 'Очистить', true))) return;
    busy = 'cache';
    try {
      const r = await post<{ freed: number }>('/api/admin/cache/clear');
      toast(`Освобождено ${size(r.freed)}`);
      await load();
    } catch (e) {
      toast(errorText(e), 6000);
    } finally {
      busy = '';
    }
  }
  async function backup() {
    busy = 'backup';
    try {
      const r = await post<{ name: string; bytes: number; folder: string }>('/api/admin/backup');
      toast(`Копия базы готова (${size(r.bytes)}): ${r.folder}`, 7000);
      await load();
    } catch (e) {
      toast(errorText(e), 6000);
    } finally {
      busy = '';
    }
  }

  const NEUTRAL = '#e4e4e7';
  type Kpi = { icon: IconName; tone: string; value: number; label: string; note: string; text?: string };
  const kpis = $derived<Kpi[]>(
    m
      ? [
          { icon: 'film', tone: NEUTRAL, value: m.library.movies, label: 'Фильмов', note: m.library.addedWeek ? `+${m.library.addedWeek} за неделю` : 'новых за неделю нет' },
          { icon: 'tv', tone: NEUTRAL, value: m.library.series, label: 'Сериалов', note: `${m.library.episodes} ${plural(m.library.episodes, 'серия', 'серии', 'серий')}` },
          { icon: 'clock', tone: NEUTRAL, value: hours, label: 'Часов кино', note: fmtHours(hours), text: '' },
          { icon: 'download', tone: NEUTRAL, value: Math.round(status.size / 1024 ** 3), label: 'ГБ на дисках', note: size(status.size) },
          { icon: 'users', tone: NEUTRAL, value: m.people.users, label: 'Зрителей', note: m.people.newWeek ? `+${m.people.newWeek} за неделю` : `${m.people.activeWeek} смотрели на этой неделе` },
          { icon: 'play', tone: NEUTRAL, value: status.watching.length, label: 'Смотрят сейчас', note: status.watching.length ? 'человек у экрана' : 'никого' },
        ]
      : [],
  );

  // «За неделю»: сумма за последние 7 суток против предыдущих 7, и линия по дням (две недели).
  type Trend = { label: string; value: number; delta: number | null; line: string; note: string };
  const spark = (a: number[]) => {
    const hi = Math.max(1, ...a);
    return a.map((v, i) => `${(i / Math.max(1, a.length - 1)) * 100},${22 - (v / hi) * 20}`).join(' ');
  };
  const trends = $derived<Trend[]>(
    m
      ? (() => {
          const d = m.activity.days;
          const sum = (a: number[]) => a.reduce((x, y) => x + y, 0);
          const views = d.map((x) => x.views);
          const people = d.map((x) => x.people);
          const part = (a: number[]) => [sum(a.slice(-7)), sum(a.slice(-14, -7))];
          const pct = (now: number, was: number) => (was > 0 ? Math.round(((now - was) / was) * 100) : null);
          const [vNow, vWas] = part(views);
          const [pNow, pWas] = part(people);
          return [
            { label: 'Просмотров за неделю', value: vNow, delta: pct(vNow, vWas), line: spark(views), note: `${vWas} неделей раньше` },
            { label: 'Заходов зрителей', value: pNow, delta: pct(pNow, pWas), line: spark(people), note: `${m.people.activeWeek} разных людей` },
            { label: 'Новых фильмов', value: m.library.addedWeek, delta: null, line: spark(views.map((_, i) => (i === views.length - 1 ? m!.library.addedWeek : 0))), note: `${m.library.addedMonth} за месяц` },
            { label: 'Входов за сутки', value: m.people.loginsDay, delta: null, line: spark(people), note: m.people.failedWeek ? `${m.people.failedWeek} неудачных за неделю` : 'неудачных нет' },
          ];
        })()
      : [],
  );
</script>

{#if failed}
  <p class="error" role="alert">Не удалось посчитать показатели: {failed} <button class="btn sm" onclick={load}>Повторить</button></p>
{:else if !m}
  <div class="kpis" aria-busy="true">{#each Array(6) as _, i (i)}<div class="kpi skeleton"></div>{/each}</div>
{:else}
  <div class="trends" aria-label="За неделю">
    {#each trends as t, i (t.label)}
      <div class="trend rise" style:--i={i}>
        <span class="tl">{t.label}</span>
        <div class="tv"><b>{t.value}</b>{#if t.delta !== null}<em class:down={t.delta < 0}>{t.delta > 0 ? '+' : ''}{t.delta}%</em>{/if}</div>
        <svg viewBox="0 0 100 24" preserveAspectRatio="none" aria-hidden="true"><polyline points={t.line} /></svg>
        <small>{t.note}</small>
      </div>
    {/each}
  </div>

  <div class="kpis" aria-label="Главные цифры">
    {#each kpis as k, i (k.label)}
      <div class="kpi rise" style:--tone={k.tone} style:--i={i}>
        <span class="mark"><Icon name={k.icon} size={18} /></span>
        <b use:tick={k.value}>{k.value}</b>
        <span class="lbl">{k.label}</span>
        <small>{k.note}</small>
      </div>
    {/each}
  </div>

  <div class="two">
    <section class="panel rise" style:--tone={NEUTRAL} style:--i={6} aria-label="Активность">
      <header>
        <span class="mark"><Icon name="pulse" size={17} /></span>
        <div><h3>Активность за две недели</h3><p>Сколько разных фильмов и серий смотрели каждый день. Помогает понять, когда Nova нужна сильнее всего.</p></div>
        <strong>{weekViews}<small>за 7 дней</small></strong>
      </header>
      <div class="bars" role="img" aria-label="Просмотры по дням за две недели">
        {#each days as d, i (d.day)}
          <div class="col" class:today={i === days.length - 1} title="{dayName(d.day)}: {count(d.views, 'просмотр', 'просмотра', 'просмотров')}, {count(d.people, 'человек', 'человека', 'человек')}">
            <i style:height="{Math.max(d.views ? 6 : 2, (d.views / peak) * 100)}%"></i>
            <span>{i % 2 === 0 || i === days.length - 1 ? dd(d.day) : ''}</span>
          </div>
        {/each}
      </div>
      <div class="hours" role="img" aria-label="Когда смотрят: просмотры по часам суток за месяц">
        {#each m.activity.hours as v, h (h)}
          <span style:--a={v / hourPeak} title="{String(h).padStart(2, '0')}:00 - {count(v, 'просмотр', 'просмотра', 'просмотров')}"></span>
        {/each}
      </div>
      <p class="axis"><span>0:00</span><span>6:00</span><span>12:00</span><span>18:00</span><span>24:00</span></p>
      <p class="hint">{busyHour >= 0 ? `Чаще всего смотрят около ${String(busyHour).padStart(2, '0')}:00.` : 'Когда начнут смотреть, здесь появится любимое время зрителей.'}</p>
    </section>

    <section class="panel rise" style:--tone={NEUTRAL} style:--i={7} aria-label="Самое смотримое">
      <header>
        <span class="mark"><Icon name="star" size={17} /></span>
        <div><h3>Самое смотримое</h3><p>Фильмы и сериалы, которые открывало больше всего разных людей.</p></div>
      </header>
      {#if m.activity.top.length}
        <ol class="top">
          {#each m.activity.top as t, i (t.id)}
            <li>
              <span class="rank">{i + 1}</span>
              <a href="/title/{t.id}"><b>{t.title}</b><small>{t.year ?? ''}{t.year ? ' · ' : ''}{t.kind === 'series' ? 'сериал' : 'фильм'}</small></a>
              <span class="meter"><i style:transform="scaleX({t.viewers / topMax})"></i></span>
              <span class="who"><Icon name="users" size={14} /> {t.viewers}</span>
            </li>
          {/each}
        </ol>
      {:else}
        <p class="empty-note">Пока никто не смотрел. Когда зрители включат фильмы, самые любимые будут тут.</p>
      {/if}
    </section>
  </div>

  <div class="two">
    <section class="panel rise" style:--tone={NEUTRAL} style:--i={8} aria-label="Место на диске">
      <header>
        <span class="mark"><Icon name="download" size={17} /></span>
        <div><h3>Куда уходит место</h3><p>Что занимает диск с данными Nova. Сами фильмы лежат в ваших папках и сюда не считаются.</p></div>
        <strong>{size(partsTotal)}<small>данные Nova</small></strong>
      </header>
      <div class="donut-row">
        <svg class="donut" viewBox="0 0 100 100" role="img" aria-label="Доли места по видам">
          <circle class="track" cx="50" cy="50" r="40" />
          {#each slices as s (s.key)}
            <circle class="slice" cx="50" cy="50" r="40" stroke={s.tone} stroke-dasharray="{s.len} {RING}" stroke-dashoffset={-s.from} />
          {/each}
        </svg>
        <ul class="legend">
          {#each parts as p (p.key)}
            <li><i style:background={p.tone}></i><span><b>{p.label}</b><small>{p.note}</small></span><em>{size(p.bytes)}</em></li>
          {/each}
        </ul>
      </div>
      {#if status.system?.diskFree != null && status.system.diskTotal}
        <p class="free"><Icon name="info" size={14} /> Свободно на диске {size(status.system.diskFree)} из {size(status.system.diskTotal)}.</p>
      {/if}
      <div class="actions">
        <button class="btn sm" disabled={busy !== '' || cacheBytes === 0} onclick={clearCache}><Icon name="trash" size={15} /> Очистить кэш видео{cacheBytes ? ` (${size(cacheBytes)})` : ''}</button>
        <button class="btn sm" disabled={busy !== ''} onclick={backup}><Icon name="download" size={15} /> Копия базы сейчас{m.backups ? ` · есть ${m.backups}` : ''}</button>
      </div>
    </section>

    <section class="panel rise" style:--tone={NEUTRAL} style:--i={9} aria-label="Качество файлов">
      <header>
        <span class="mark"><Icon name="film" size={17} /></span>
        <div><h3>Качество видео</h3><p>В каком разрешении лежат ваши файлы. Выше качество - больше места и тяжелее перекодирование для слабого интернета.</p></div>
      </header>
      {#if resTotal}
        <ul class="bars-h" aria-label="Файлы по разрешению">
          {#each resolutions as r (r.q)}
            <li>
              <b>{r.label}</b>
              <span class="track"><i style:transform="scaleX({r.files / resMax})"></i></span>
              <em>{count(r.files, 'файл', 'файла', 'файлов')} · {Math.round((r.files / resTotal) * 100)}%</em>
            </li>
          {/each}
        </ul>
        {#if m.library.codecs.length}
          <p class="codecs">Кодеки: {#each m.library.codecs as c (c.codec)}<span>{c.codec.toUpperCase()} <b>{c.files}</b></span>{/each}</p>
        {/if}
      {:else}
        <p class="empty-note">Файлы ещё проверяются - разрешения появятся после проверки.</p>
      {/if}
    </section>
  </div>

  <section class="panel wide rise" style:--tone={NEUTRAL} style:--i={10} aria-label="Зрители и общение">
    <header>
      <span class="mark"><Icon name="users" size={17} /></span>
      <div><h3>Зрители и общение</h3><p>Кто заходит в Nova и как люди общаются между собой.</p></div>
    </header>
    <div class="mini">
      <div><b>{m.people.activeWeek}</b><span>смотрели за неделю</span></div>
      <div><b>{m.people.loginsDay}</b><span>входов за сутки</span></div>
      <div class:warn={m.people.failedWeek > 5}><b>{m.people.failedWeek}</b><span>неудачных входов за неделю</span></div>
      <div><b>{m.social.friends}</b><span>дружб</span></div>
      <div><b>{m.social.reviews}</b><span>отзывов</span></div>
      <div><b>{m.social.messages}</b><span>сообщений</span></div>
      <div><b>{m.social.lists}</b><span>подборок</span></div>
      <div><b>{m.social.wishes}</b><span>просьб добавить фильм</span></div>
    </div>
  </section>
{/if}

<style>
  .kpis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(136px, 1fr));
    gap: 12px;
    margin-bottom: 18px;
  }
  .kpi {
    position: relative;
    display: grid;
    gap: 2px;
    padding: 16px;
    min-height: 132px;
    border: 1px solid color-mix(in srgb, var(--tone) 28%, transparent);
    border-radius: 26px;
    background: linear-gradient(160deg, color-mix(in srgb, var(--tone) 16%, transparent), color-mix(in srgb, var(--tone) 4%, transparent) 70%);
    overflow: hidden;
  }
  .kpi.skeleton {
    border-color: transparent;
    background: var(--surface);
  }
  /* Не `.chip`: у общего класса из app.css есть отступы и подъём при наведении - значок уезжал из центра круга. */
  .mark {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border-radius: 50%;
    background: color-mix(in srgb, var(--tone) 24%, transparent);
    color: var(--tone);
    flex: none;
  }
  .kpi b {
    margin-top: 8px;
    font-size: clamp(26px, 2.4vw, 34px);
    font-weight: 800;
    line-height: 1.05;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }
  .kpi .lbl {
    font-size: 13px;
    font-weight: 700;
  }
  .kpi small {
    color: var(--muted);
    font-size: 12px;
  }
  .two {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
    margin-bottom: 14px;
  }
  .panel {
    min-width: 0;
    padding: 18px;
    border: 1px solid var(--line);
    border-radius: 28px;
    background: var(--surface);
    margin-bottom: 14px;
  }
  .two .panel {
    margin-bottom: 0;
  }
  .panel header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 16px;
  }
  .panel header > div {
    flex: 1;
    min-width: 0;
  }
  .panel h3 {
    margin: 0;
    font-size: 16px;
  }
  .panel header p {
    margin: 3px 0 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.4;
  }
  .panel strong {
    display: grid;
    justify-items: end;
    font-size: 22px;
    font-variant-numeric: tabular-nums;
  }
  .panel strong small {
    color: var(--muted);
    font-size: 11px;
    font-weight: 500;
  }
  /* Столбики активности: высота - число просмотров, сегодняшний выделен. */
  .bars {
    display: grid;
    grid-template-columns: repeat(14, minmax(0, 1fr));
    gap: 6px;
    align-items: end;
    height: 120px;
  }
  .col {
    display: grid;
    grid-template-rows: 1fr auto;
    height: 100%;
    gap: 4px;
    align-items: end;
    text-align: center;
  }
  .col i {
    display: block;
    border-radius: 6px 6px 3px 3px;
    background: color-mix(in srgb, var(--tone) 55%, transparent);
    transform-origin: bottom;
    animation: grow 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    transition: background 0.2s;
  }
  .col:hover i,
  .col.today i {
    background: var(--tone);
  }
  .col span {
    min-height: 14px;
    color: var(--faint);
    font-size: 11px;
  }
  .hours {
    display: grid;
    grid-template-columns: repeat(24, minmax(0, 1fr));
    gap: 3px;
    margin-top: 14px;
  }
  .hours span {
    height: 22px;
    border-radius: 5px;
    background: color-mix(in srgb, var(--tone) calc(10% + var(--a) * 90%), transparent);
  }
  .axis {
    display: flex;
    justify-content: space-between;
    margin: 4px 0 0;
    color: var(--faint);
    font-size: 11px;
  }
  .hint {
    margin: 10px 0 0;
    color: var(--muted);
    font-size: 13px;
  }
  .top {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .top li {
    display: grid;
    grid-template-columns: 26px minmax(0, 1.4fr) minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
  }
  .rank {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--tone) 22%, transparent);
    color: var(--tone);
    font-size: 12px;
    font-weight: 800;
  }
  .top a {
    display: grid;
    min-width: 0;
  }
  .top b {
    overflow: hidden;
    font-size: 14px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .top small {
    color: var(--faint);
    font-size: 12px;
  }
  .meter {
    display: block;
    height: 8px;
    border-radius: 4px;
    background: var(--line);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    background: var(--tone);
    transform-origin: left;
    animation: sweep 0.8s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  .who {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  /* Кольцо и подписи рядом; на узком экране - друг под другом. */
  .donut-row {
    display: grid;
    grid-template-columns: clamp(120px, 14vw, 168px) minmax(0, 1fr);
    gap: 20px;
    align-items: center;
  }
  .donut {
    width: 100%;
    height: auto;
    transform: rotate(-90deg);
    fill: none;
    stroke-width: 14;
  }
  .donut .track {
    stroke: #ffffff10;
  }
  .donut .slice {
    transition: stroke-dasharray 0.6s;
  }
  .bars-h {
    display: grid;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .bars-h li {
    display: grid;
    grid-template-columns: 62px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
  }
  .bars-h b {
    font-size: 13.5px;
  }
  .bars-h .track {
    display: block;
    height: 10px;
    overflow: hidden;
    border-radius: 5px;
    background: #ffffff12;
  }
  .bars-h .track i {
    display: block;
    height: 100%;
    border-radius: 5px;
    background: #fff;
    transform-origin: left;
    animation: sweep 0.8s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  .bars-h em {
    color: var(--muted);
    font-size: 12.5px;
    font-style: normal;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .legend {
    display: grid;
    gap: 9px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .legend li {
    display: grid;
    grid-template-columns: 12px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
  }
  .legend li > i {
    width: 12px;
    height: 12px;
    border-radius: 4px;
  }
  .legend span {
    display: grid;
    min-width: 0;
  }
  .legend b {
    font-size: 13.5px;
  }
  .legend small {
    color: var(--faint);
    font-size: 11.5px;
  }
  .legend em {
    color: var(--muted);
    font-size: 13px;
    font-style: normal;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .free {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 14px 0 0;
    color: var(--muted);
    font-size: 13px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
  }
  .codecs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 14px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .codecs span {
    padding: 3px 10px;
    border-radius: 99px;
    background: var(--line);
    color: var(--text);
  }
  .codecs b {
    color: var(--tone);
  }
  .mini {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 10px;
  }
  .mini div {
    display: grid;
    gap: 2px;
    padding: 12px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--tone) 8%, transparent);
  }
  .mini b {
    font-size: 24px;
    font-variant-numeric: tabular-nums;
  }
  .mini span {
    color: var(--muted);
    font-size: 12px;
  }
  .mini .warn {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
  }
  .mini .warn b {
    color: var(--danger);
  }
  .empty-note {
    margin: 4px 0;
    color: var(--muted);
    font-size: 14px;
  }
  @media (max-width: 1100px) {
    .two {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (max-width: 640px) {
    .donut-row {
      grid-template-columns: minmax(0, 1fr);
      justify-items: center;
    }
    .donut-row .legend {
      justify-self: stretch;
    }
    .bars-h li {
      grid-template-columns: 54px minmax(0, 1fr);
    }
    .bars-h em {
      grid-column: 1 / -1;
    }
    .top li {
      grid-template-columns: 26px minmax(0, 1fr) auto;
    }
    .top .meter {
      display: none;
    }
  }
  :global(.no-motion) .col i,
  :global(.no-motion) .meter i,
  :global(.no-motion) .bars-h .track i {
    animation: none;
  }
  @keyframes grow {
    from {
      transform: scaleY(0);
    }
  }
  @keyframes sweep {
    from {
      transform: scaleX(0);
    }
  }
  .trends { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin-bottom: 14px; }
  .trend { min-width: 0; padding: 18px 20px; border: 1px solid #ffffff1a; border-radius: 26px; background: #101012; }
  .tl { color: var(--muted); font-size: 12px; font-weight: 600; }
  .tv { display: flex; align-items: baseline; gap: 8px; margin-top: 8px; }
  .tv b { font-size: 34px; font-weight: 800; letter-spacing: -0.04em; line-height: 1; font-variant-numeric: tabular-nums; }
  .tv em { color: var(--ok); font-size: 12px; font-style: normal; font-weight: 700; }
  .tv em.down { color: #fca5a5; }
  .trend svg { display: block; width: 100%; height: 28px; margin-top: 10px; }
  .trend polyline { fill: none; stroke: #ffffffb0; stroke-width: 1.6; stroke-linejoin: round; stroke-linecap: round; vector-effect: non-scaling-stroke; }
  .trend small { display: block; margin-top: 6px; color: var(--muted); font-size: 12px; }
  @media (max-width: 900px) { .trends { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>
