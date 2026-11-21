<script lang="ts">
  import { count, plural } from '$lib/fmt';
  import { countUp } from '$lib/ui/countUp';
  import type { ProfileStats } from '$lib/types';

  // Вкладка «Статистика»: часы, серия дней, средняя оценка, когда и в какие дни смотрит, оценки, год по месяцам и эпохи.
  // Столбики вырастают по очереди, числа набегают. Месяцы - кнопки: нажать - точное число.
  let { stats }: { stats: ProfileStats } = $props();

  const MONTHS = ['янв', 'фев', 'мар', 'апр', 'май', 'июн', 'июл', 'авг', 'сен', 'окт', 'ноя', 'дек'];
  const DAYS = ['пн', 'вт', 'ср', 'чт', 'пт', 'сб', 'вс'];
  const DAYS_FULL = ['понедельник', 'вторник', 'среду', 'четверг', 'пятницу', 'субботу', 'воскресенье'];
  const PARTS = ['Утро', 'День', 'Вечер', 'Ночь'];
  const WHO = ['Утренний зритель', 'Дневной зритель', 'Вечерний зритель', 'Ночная сова'];

  const max = (v: number[]) => v.reduce((a, b) => Math.max(a, b), 0);
  const top = (v: number[]) => (max(v) ? v.indexOf(max(v)) : -1);
  const sum = (v: number[]) => v.reduce((a, b) => a + b, 0);
  const pct = (x: number, m: number) => Math.max(7, Math.round((x / Math.max(1, m)) * 100));

  const hours = $derived(stats.hours >= 10 ? Math.round(stats.hours) : stats.hours);
  const days = $derived(Math.floor(stats.hours / 24));
  const dayparts = $derived(stats.dayparts ?? [0, 0, 0, 0]);
  const weekdays = $derived(stats.weekdays ?? [0, 0, 0, 0, 0, 0, 0]);
  const streak = $derived(stats.streak ?? { best: 0, current: 0 });
  const ratings = $derived(stats.ratings ?? { avg: null, hist: Array(10).fill(0), count: 0 });
  const eras = $derived((stats.decades ?? []).slice(-8));

  // Месяцы: months[11] - текущий; подписи - по `monthLabels`.
  let picked = $state<number | null>(null);
  const label = (i: number) => {
    const m = stats.monthLabels?.[i];
    return m ? MONTHS[Number(m.slice(5, 7)) - 1] : MONTHS[(new Date().getMonth() - (11 - i) + 24) % 12];
  };
  const full = (i: number) => {
    const m = stats.monthLabels?.[i];
    return m ? new Date(`${m}-01T00:00:00Z`).toLocaleDateString('ru-RU', { month: 'long', year: 'numeric', timeZone: 'UTC' }) : label(i);
  };
  const total = $derived(sum(stats.months));
  const bestMonth = $derived(top(stats.months));
</script>

<div class="grid">
  <section class="w rise" style:--i={0} aria-label="Время просмотра">
    <small>Часы просмотра</small>
    <div class="big"><span use:countUp={{ value: hours, decimals: hours < 10 && hours % 1 ? 1 : 0 }}>0</span> <i>{plural(Math.floor(stats.hours), 'час', 'часа', 'часов')}</i></div>
    <p class="cap">
      {#if days >= 1}Это дольше {count(days, 'день', 'дня', 'дней')} подряд.{:else}Пока меньше суток подряд.{/if}
      {#if stats.better != null && stats.better >= 50}Смотрит больше, чем <mark>{stats.better}%</mark> зрителей Nova.{/if}
    </p>
  </section>

  <section class="w rise" style:--i={1} aria-label="Серия дней подряд">
    <small>Серия дней подряд</small>
    <div class="big"><span use:countUp={{ value: streak.best }}>0</span></div>
    <p class="cap">{streak.best ? `Рекорд. Сейчас - ${count(streak.current, 'день', 'дня', 'дней')}` : 'Появится, когда досмотрите что-нибудь'}</p>
  </section>

  <section class="w rise" style:--i={2} aria-label="Средняя оценка">
    <small>Средняя оценка</small>
    <div class="big">{#if ratings.avg != null}<span use:countUp={{ value: ratings.avg, decimals: 1 }}>0</span>{:else}-{/if}</div>
    <p class="cap">{ratings.count ? `По ${count(ratings.count, 'оценке', 'оценкам', 'оценкам')}` : 'Оценок пока нет'}</p>
  </section>

  <section class="w rise" style:--i={3} aria-label="Когда смотрит">
    <small>Когда смотрит</small>
    {#if sum(dayparts)}
      <div class="bars">{#each dayparts as v, i (i)}<b class:h={i === top(dayparts)} style:--i={i} style:height="{pct(v, max(dayparts))}%" title="{PARTS[i]}: {v}"></b>{/each}</div>
      <div class="lab">{#each PARTS as p (p)}<span>{p}</span>{/each}</div>
      <p class="cap">{WHO[top(dayparts)]}</p>
    {:else}<p class="cap muted">Появится, когда досмотрите что-нибудь</p>{/if}
  </section>

  <section class="w rise" style:--i={4} aria-label="Дни недели">
    <small>Дни недели</small>
    {#if sum(weekdays)}
      <div class="bars">{#each weekdays as v, i (i)}<b class:h={i === top(weekdays)} style:--i={i} style:height="{pct(v, max(weekdays))}%" title="{DAYS[i]}: {v}"></b>{/each}</div>
      <div class="lab">{#each DAYS as d (d)}<span>{d}</span>{/each}</div>
      <p class="cap">Любимый день - {DAYS_FULL[top(weekdays)]}</p>
    {:else}<p class="cap muted">Появится, когда досмотрите что-нибудь</p>{/if}
  </section>

  <section class="w rise" style:--i={5} aria-label="Оценки">
    <small>Оценки</small>
    {#if ratings.count}
      <div class="bars">{#each ratings.hist as v, i (i)}<b class:h={v === max(ratings.hist) && v > 0} style:--i={i} style:height="{v ? pct(v, max(ratings.hist)) : 4}%" title="{(i + 1) / 2} ★: {v}"></b>{/each}</div>
      <div class="lab">{#each ratings.hist as _, i (i)}<span>{i === 0 ? '0,5' : i === 4 ? '2,5' : i === 9 ? '5' : ''}</span>{/each}</div>
    {:else}<p class="cap muted">Оценок пока нет</p>{/if}
  </section>

  <section class="w wide rise" style:--i={6} aria-label="По месяцам">
    <small>Досмотрено за год</small>
    <div class="bars tall">
      {#each stats.months as m, i (i)}
        <button type="button" class="col" class:h={i === bestMonth} class:selected={picked === i} title="{full(i)}: {m}" aria-label="{full(i)}: досмотрено {m}" aria-pressed={picked === i} onclick={() => (picked = i)}>
          <b style:--i={i} style:height="{m ? pct(m, max(stats.months)) : 4}%"></b>
          <span>{label(i)}</span>
        </button>
      {/each}
    </div>
    <p class="cap chart-detail" aria-live="polite">
      {picked === null ? (total ? `${count(total, 'фильм', 'фильма', 'фильмов')} и серий за год. Выберите месяц - увидите точное число.` : 'Пока нет завершённых просмотров. Начатые фильмы учитываются в часах.') : `${full(picked)} · досмотрено ${stats.months[picked]}`}
    </p>
  </section>

  <section class="w rise" style:--i={7} aria-label="Эпохи">
    <small>Эпохи</small>
    {#if eras.length}
      <div class="bars">{#each eras as e, i (e.decade)}<b class:h={e.count === max(eras.map((x) => x.count))} style:--i={i} style:height="{pct(e.count, max(eras.map((x) => x.count)))}%" title="{e.decade}-е: {e.count}"></b>{/each}</div>
      <div class="lab">{#each eras as e (e.decade)}<span>{String(e.decade % 100).padStart(2, '0')}</span>{/each}</div>
      <p class="cap">Больше всего - {eras.reduce((a, b) => (b.count > a.count ? b : a)).decade}-е</p>
    {:else}<p class="cap muted">Появится, когда досмотрите что-нибудь</p>{/if}
  </section>
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0;
    border-block: 1px solid var(--line);
  }
  .w {
    min-width: 0;
    padding: 20px 18px;
    background: transparent;
  }
  .w:not(:nth-child(3n)) { border-right: 1px solid var(--line); }
  .w:nth-child(-n+6) { border-bottom: 1px solid var(--line); }
  .w.wide {
    grid-column: span 2;
  }
  small {
    display: block;
    margin-bottom: 8px;
    color: var(--muted);
    font-size: 12px;
  }
  .big {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 40px;
    font-weight: 800;
    line-height: 1;
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
  }
  .big i {
    font-size: 16px;
    font-style: normal;
    font-weight: 600;
    letter-spacing: 0;
    color: var(--muted);
  }
  .cap {
    margin: 8px 0 0;
    color: #dcdcdf;
    font-size: 13px;
    line-height: 1.4;
  }
  .cap.muted {
    color: var(--muted);
  }
  mark {
    padding: 0 6px;
    border-radius: 6px;
    background: #fff;
    color: #000;
    font-weight: 700;
  }
  .bars {
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 6px;
    height: 74px;
  }
  .bars.tall {
    height: 112px;
    gap: 5px;
  }
  .bars b {
    flex: 1 1 0;
    min-width: 0;
    max-width: 56px;
    border-radius: 4px 4px 0 0;
    background: #ffffff40;
    transform-origin: bottom;
    animation: grow 0.8s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--i) * 55ms + 150ms);
  }
  .bars b.h {
    background: #fff;
  }
  .col {
    flex: 1 1 0;
    min-width: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: stretch;
    gap: 6px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }
  .col b {
    flex: none;
    transition: background 0.2s;
  }
  .col:hover b {
    background: #ffffff80;
  }
  .col.h b,
  .col.selected b {
    background: #fff;
  }
  .col span {
    font-size: 11px;
    text-align: center;
    overflow: hidden;
  }
  .col.selected span {
    color: var(--text);
  }
  .col:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }
  .lab {
    display: flex;
    justify-content: center;
    gap: 6px;
    margin-top: 6px;
    color: var(--muted);
    font-size: 11px;
  }
  .lab span {
    flex: 1 1 0;
    min-width: 0;
    max-width: 56px;
    overflow: hidden;
    text-align: center;
  }
  @keyframes grow {
    from {
      transform: scaleY(0);
    }
  }
  .rise {
    animation: rise 0.8s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--i) * 90ms);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate3d(0, 26px, 0);
    }
  }
  @media (max-width: 900px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .w:not(:nth-child(3n)) { border-right: 0; }
    .w:nth-child(odd) { border-right: 1px solid var(--line); }
    .w:nth-child(-n+6) { border-bottom: 0; }
    .w:not(:nth-last-child(-n+2)) { border-bottom: 1px solid var(--line); }
    .w.wide { grid-column: span 2; }
    .big {
      font-size: 32px;
    }
  }
  @media (max-width: 420px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
    .w.wide {
      grid-column: auto;
    }
    .w,.w:nth-child(odd) { padding:18px 8px;border-right:0;border-bottom:1px solid var(--line); }
    .w:last-child { border-bottom:0; }
    .bars { justify-content:flex-start; }
  }
  /* Карточки по макету Figma 21.11: каждый показатель - своя плитка, ведущий столбик белый, остальные приглушены. */
  .grid { gap: 12px; border: 0; }
  .w, .w:nth-child(odd), .w:not(:nth-child(3n)), .w:nth-child(-n+6), .w:not(:nth-last-child(-n+2)), .w:last-child {
    padding: 20px 22px; border: 1px solid #ffffff14; border-radius: 22px; background: #101012;
  }
  .bars b { border-radius: 8px 8px 3px 3px; background: #ffffff2e; }
  .bars b.h { background: #fff; }
  @media (max-width: 420px) { .w, .w:nth-child(odd) { padding: 18px; } }
</style>
