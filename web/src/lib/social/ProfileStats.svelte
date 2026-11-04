<script lang="ts">
  import { count, plural } from '$lib/fmt';
  import type { ProfileStats } from '$lib/types';

  // Статистика на личной странице: крупные часы просмотра, «билеты» с цифрами, любимые жанры
  // и досмотренное по месяцам - рядами кресел зала.
  let { stats, accent = null }: { stats: ProfileStats; accent?: string | null } = $props();

  const MONTHS = ['янв', 'фев', 'мар', 'апр', 'май', 'июн', 'июл', 'авг', 'сен', 'окт', 'ноя', 'дек'];
  // months[11] - текущий месяц, левее - раньше (по 30 дней).
  const now = new Date().getMonth();
  const label = (i: number) => MONTHS[(now - (11 - i) + 24) % 12];
  const fullLabel = (i: number) => stats.monthLabels?.[i] ? new Date(`${stats.monthLabels[i]}-01T00:00:00Z`).toLocaleDateString('ru-RU', { month: 'long', year: 'numeric', timeZone: 'UTC' }) : label(i);
  const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
  let selected = $state<number | null>(null);
  const total = $derived(stats.months.reduce((a, b) => a + b, 0));
  const active = $derived(stats.months.filter((m) => m > 0).length);
  const peak = $derived(Math.max(1, ...stats.months));
  const hours = $derived(stats.hours >= 10 ? Math.round(stats.hours) : stats.hours);
  const days = $derived(Math.floor(stats.hours / 24));
  const best = $derived(total ? stats.months.indexOf(Math.max(...stats.months)) : -1);

  // Одно кресло - несколько просмотров, чтобы самый плотный месяц не выше восьми кресел.
  const per = $derived(Math.max(1, Math.ceil(peak / 8)));
  const seats = (m: number) => (m ? Math.max(1, Math.round(m / per)) : 0);

  const tickets = $derived([
    { n: String(stats.movies), label: cap(plural(stats.movies, 'фильм', 'фильма', 'фильмов')) },
    { n: String(stats.episodes), label: `${cap(plural(stats.episodes, 'серия', 'серии', 'серий'))} · ${count(stats.series, 'сериал', 'сериала', 'сериалов')}` },
    { n: best >= 0 ? cap(label(best)) : '-', label: best >= 0 ? `Лучший месяц · ${stats.months[best]}` : 'Лучший месяц' },
    { n: String(stats.outside), label: 'Отмечено в Обзоре' },
    { n: String(stats.reviews), label: cap(plural(stats.reviews, 'отзыв', 'отзыва', 'отзывов')) },
    { n: String(stats.friends), label: cap(plural(stats.friends, 'друг', 'друга', 'друзей')) },
  ]);
  // Жанры словами: чем чаще, тем крупнее (первые - три ступени, остальные - мелко).
  const SIZES = ['w1', 'w2', 'w3', 'w3', 'w4', 'w4'];
</script>

<div class="stats" style:--acc={accent ?? '#f5f5f7'}>
  <section class="hero" aria-label="Время просмотра">
    <small>Просмотр в Nova</small>
    <div class="big"><b>{hours}</b><span>{plural(Math.floor(stats.hours), 'час', 'часа', 'часов')}</span></div>
    <p>
      {#if days >= 1}Это дольше {count(days, 'день', 'дня', 'дней')} подряд.{:else}Пока меньше суток подряд.{/if}
      {#if stats.better != null && stats.better >= 50}Смотрит больше, чем <mark>{stats.better}%</mark> зрителей Nova.{/if}
    </p>
  </section>

  <ul class="tickets">
    {#each tickets as t, i (t.label)}
      <li class="ticket">
        <div class="main"><small aria-hidden="true">Зал {Math.floor(i / 2) + 1} · ряд {(i % 2) + 1}</small><b>{t.n}</b><span>{t.label}</span></div>
        <div class="stub" aria-hidden="true"><i>№ {String(i + 1).padStart(4, '0')}</i></div>
      </li>
    {/each}
  </ul>

  <div class="charts">
    <section class="card" aria-label="Любимые жанры">
      <h3>Любимые жанры</h3>
      {#if stats.genres.length}
        <ul class="fav">
          {#each stats.genres as g, i (g.genre)}
            <li class={SIZES[i] ?? 'w4'} title="{g.genre}: {g.count}">{g.genre}<sup>{g.count}</sup></li>
          {/each}
        </ul>
      {:else}
        <p class="muted">Появятся, когда что-нибудь посмотрите.</p>
      {/if}
    </section>
    <section class="card" aria-label="По месяцам">
      <div class="chart-title"><h3>Досмотрено за год</h3><b>{total} <small>фильмов и серий</small></b></div>
      <div class="chart-summary"><span>{count(active, 'активный месяц', 'активных месяца', 'активных месяцев')}</span><span>В среднем {(total / 12).toLocaleString('ru-RU', { maximumFractionDigits: 1 })} в месяц</span></div>
      <div class="hall">
        {#each stats.months as m, i (i)}
          <button class="col" class:selected={selected === i} title="{fullLabel(i)}: {m}" aria-label="{fullLabel(i)}: досмотрено {m}" aria-pressed={selected === i} onclick={() => (selected = i)}>
            <span class="rows">{#each { length: seats(m) } as _, k (k)}<i></i>{/each}{#if !m}<i class="empty"></i>{/if}</span>
            <small>{label(i)}</small>
          </button>
        {/each}
      </div>
      <p class="chart-detail" aria-live="polite">{selected === null ? `Одно кресло - ${count(per, 'просмотр', 'просмотра', 'просмотров')}. Выберите месяц, чтобы увидеть точное число.` : `${fullLabel(selected)} · досмотрено ${stats.months[selected]}`}</p>
      {#if !total}<p class="muted">Пока нет завершённых просмотров. Начатые фильмы учитываются в часах просмотра.</p>{/if}
      <p class="chart-note">Часы рассчитаны по сохранённым позициям локального видео и внешних источников. Отметка «Просмотрел» учитывается в количестве, но не добавляет выдуманное время.</p>
    </section>
  </div>
</div>

<style>
  .stats {
    display: grid;
    gap: 16px;
  }
  /* Крупные часы. */
  .hero {
    display: grid;
    gap: 6px;
    padding: 26px 24px;
    border-radius: var(--radius);
    background: linear-gradient(160deg, var(--surface-2), var(--surface));
  }
  .hero small {
    font-size: 12px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .big {
    display: flex;
    align-items: baseline;
    gap: 10px;
    color: var(--acc);
  }
  .big b {
    font-size: clamp(56px, 12vw, 96px);
    line-height: 0.95;
    font-weight: 800;
    letter-spacing: -0.045em;
    font-variant-numeric: tabular-nums;
  }
  .big span {
    font-size: clamp(18px, 3.4vw, 30px);
    font-weight: 600;
    color: var(--muted);
  }
  .hero p {
    margin: 4px 0 0;
    font-size: 17px;
    line-height: 1.5;
    max-width: 40ch;
  }
  .hero mark {
    padding: 1px 8px;
    border-radius: 8px;
    background: #fff;
    color: #000;
    font-weight: 700;
  }
  /* Билеты: бежевая бумага, вырезы по бокам, пунктир у корешка. */
  .tickets {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }
  .ticket {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 48px;
    min-height: 104px;
    overflow: hidden;
    border-radius: 14px;
    background: #f1ece1;
    color: #17120a;
    -webkit-mask:
      radial-gradient(circle 9px at 0 50%, #0000 98%, #000) left / 51% 100% no-repeat,
      radial-gradient(circle 9px at 100% 50%, #0000 98%, #000) right / 51% 100% no-repeat;
    mask:
      radial-gradient(circle 9px at 0 50%, #0000 98%, #000) left / 51% 100% no-repeat,
      radial-gradient(circle 9px at 100% 50%, #0000 98%, #000) right / 51% 100% no-repeat;
  }
  .ticket .main {
    display: grid;
    align-content: space-between;
    gap: 4px;
    padding: 13px 4px 13px 22px;
    min-width: 0;
  }
  .ticket small {
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6a604b;
  }
  .ticket b {
    font-size: 36px;
    line-height: 1;
    font-weight: 700;
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
  }
  .ticket .main span {
    font-size: 13px;
    font-weight: 500;
    color: #3b3326;
    overflow-wrap: break-word;
    hyphens: none;
  }
  .stub {
    display: grid;
    place-items: center;
    padding: 8px 0;
    border-left: 2px dashed #17120a33;
  }
  .stub i {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    font-style: normal;
    font-size: 11px;
    letter-spacing: 0.22em;
    color: #6a604b;
  }
  @media (max-width: 760px) {
    .tickets {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 400px) {
    .ticket {
      grid-template-columns: minmax(0, 1fr) 36px;
    }
    .ticket b {
      font-size: 30px;
    }
    .ticket .main {
      padding-left: 18px;
    }
  }

  .charts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 300px), 1fr));
    gap: 16px;
  }
  .charts .card {
    padding: 18px;
    background: var(--surface);
    border-radius: var(--radius);
    min-width: 0;
  }
  h3 {
    font-size: 15px;
    font-weight: 600;
    margin: 0 0 14px;
  }
  /* Жанры словами: размер - по месту в списке. */
  .fav {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px 14px;
  }
  .fav li {
    letter-spacing: -0.03em;
    text-transform: capitalize;
    overflow-wrap: anywhere;
  }
  .fav sup {
    margin-left: 3px;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0;
    color: var(--faint);
  }
  .w1 {
    font-size: clamp(32px, 8vw, 50px);
    font-weight: 800;
    line-height: 1.05;
    color: var(--acc);
  }
  .w2 {
    font-size: clamp(24px, 5vw, 32px);
    font-weight: 700;
    color: #cfcfd4;
  }
  .w3 {
    font-size: clamp(18px, 3.4vw, 22px);
    font-weight: 600;
    color: var(--muted);
  }
  .w4 {
    font-size: 15px;
    color: var(--faint);
  }
  /* Месяцы - ряды кресел. */
  .hall {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 5px;
    height: 170px;
  }
  .col {
    display: grid;
    grid-template-rows: minmax(0, 1fr) 18px;
    justify-items: center;
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    cursor: pointer;
  }
  .rows {
    display: flex;
    flex-direction: column-reverse;
    gap: 3px;
    align-items: center;
    justify-content: flex-start;
    width: 100%;
    padding-bottom: 6px;
  }
  .rows i {
    display: block;
    width: 100%;
    max-width: 20px;
    height: 9px;
    border-radius: 4px 4px 2px 2px;
    background: var(--acc);
  }
  .rows i.empty {
    height: 3px;
    background: var(--surface-2);
  }
  .col.selected .rows i {
    background: #f5c518;
  }
  .col small {
    font-size: 10.5px;
    color: var(--faint);
  }
  .chart-title {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: baseline;
  }
  .chart-title b {
    font-size: 24px;
    white-space: nowrap;
  }
  .chart-title b small {
    font-size: 11px;
    color: var(--muted);
    font-weight: 400;
  }
  .chart-summary {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: var(--muted);
    font-size: 11px;
    margin-bottom: 18px;
  }
  .chart-detail {
    font-size: 12px;
    margin: 16px 0 10px;
  }
  .chart-note {
    font-size: 11px;
    color: var(--muted);
    line-height: 1.6;
    margin: 0;
  }
  /* Узкий телефон: двенадцать подписей не помещаются - через одну (выбранный месяц - всегда). */
  @media (max-width: 480px) {
    .hall {
      gap: 3px;
      height: 150px;
    }
    .rows i {
      height: 7px;
    }
    .col:nth-child(odd):not(.selected) small {
      visibility: hidden;
    }
    .col small {
      font-size: 9.5px;
    }
  }
  /* Совсем узкий экран (плита профиля уже): подпись - у каждого третьего месяца. */
  @media (max-width: 380px) {
    .col:not(:nth-child(3n + 2)):not(.selected) small {
      visibility: hidden;
    }
    .col small {
      font-size: 8px;
      letter-spacing: -0.02em;
    }
  }
</style>
