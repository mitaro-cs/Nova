<script lang="ts">
  import type { ExtCard } from '$lib/types';
  import { fadeIn } from '$lib/ui/fade';

  // «Открытия» страницы «Обзор»: подборка дня веером постеров, две плитки главных списков и календарь премьер
  // (ближайшие даты из «Скоро в кино»). Всё из уже загруженной ленты IMDb, без новых запросов.
  type Row = { key: string; title: string; items: ExtCard[] };
  let { rows }: { rows: Row[] } = $props();

  const row = (key: string) => rows.find((r) => r.key === key);
  const hero = $derived(row('popular_movies') ?? row('new_movies') ?? rows[0]);
  const fan = $derived((hero?.items ?? []).filter((c) => c.poster).slice(0, 4));
  const tiles = $derived(
    [
      { key: 'top_movies', kicker: 'Список', name: 'Лучшие фильмы всех времён' },
      { key: 'top_series', kicker: 'Сериалы', name: 'Лучшие сериалы' },
    ].filter((t) => row(t.key)),
  );

  const today = new Date().toISOString().slice(0, 10);
  const soon = $derived(
    (row('soon_movies')?.items ?? [])
      .filter((c) => c.release && /^\d{4}-\d{2}-\d{2}$/.test(c.release) && c.release >= today && c.poster)
      .sort((a, b) => (a.release ?? '').localeCompare(b.release ?? ''))
      .slice(0, 6),
  );
  const day = (r: string) => new Date(`${r}T12:00:00`);
  const month = $derived(soon.length ? day(soon[0].release!).toLocaleDateString('ru-RU', { month: 'long' }) : '');
</script>

{#if hero && fan.length >= 2}
  <section class="spot" aria-label="Открытия">
    <a class="hero" href="/discover?chart={hero.key}">
      <span class="fan" aria-hidden="true">{#each fan as c, k (c.id)}<img use:fadeIn src={c.poster} alt="" style:--k={k} loading="lazy" decoding="async" />{/each}</span>
      <span class="tx"><small>Подборка дня</small><b>{hero.title}</b><em>Открыть · {hero.items.length}</em></span>
    </a>
    <div class="side">
      {#each tiles as t, i (t.key)}
        <a class="tile t{i}" href="/discover?chart={t.key}"><small>{t.kicker}</small><b>{t.name}</b></a>
      {/each}
    </div>
  </section>
{/if}

{#if soon.length >= 3}
  <section class="cal" aria-label="Календарь премьер">
    <div class="hd"><h2>Календарь премьер</h2><span>{month}</span></div>
    <ul>
      {#each soon as c (c.id)}
        <li>
          <a href="/ext/{c.id}">
            <span class="d"><b>{day(c.release!).getDate()}</b><small>{day(c.release!).toLocaleDateString('ru-RU', { weekday: 'short' })}</small></span>
            <img use:fadeIn src={c.poster} alt="" loading="lazy" decoding="async" />
            <span class="n">{c.title}</span>
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .spot { display: grid; grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr); gap: 16px; margin-bottom: 28px; }
  .hero { position: relative; display: block; min-height: 340px; overflow: hidden; border: 1px solid #ffffff1a; border-radius: 36px; background: linear-gradient(135deg, #25254a, #0b0b14); transition: transform 0.25s; }
  .hero:hover { transform: translate3d(0, -2px, 0); }
  .fan { position: absolute; right: 26px; top: 26px; display: flex; }
  .fan img { width: clamp(70px, 8vw, 120px); aspect-ratio: 2 / 3; margin-left: -30px; border: 3px solid #0b0b14; border-radius: 14px; object-fit: cover; background: #26262a; transform: translate3d(0, calc(var(--k) * 6px), 0); }
  .tx { position: absolute; left: clamp(20px, 3vw, 36px); bottom: clamp(20px, 3vw, 34px); display: grid; gap: 8px; max-width: 420px; }
  .tx small, .tile small { font-size: 11px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; opacity: 0.7; }
  .tx b { font-size: clamp(26px, 3vw, 40px); font-weight: 800; letter-spacing: -0.04em; line-height: 1.05; }
  .tx em { justify-self: start; padding: 11px 24px; border-radius: 999px; background: #fff; color: #0a0a0b; font-size: 14px; font-style: normal; font-weight: 700; }
  .side { display: grid; grid-template-rows: 1fr 1fr; gap: 16px; }
  .tile { display: grid; align-content: end; gap: 6px; min-height: 150px; padding: 22px; border: 1px solid #ffffff1a; border-radius: 30px; transition: transform 0.25s; }
  .tile:hover { transform: translate3d(0, -2px, 0); }
  .tile b { font-size: 24px; font-weight: 800; letter-spacing: -0.03em; line-height: 1.1; }
  .t0 { background: linear-gradient(135deg, #3a3a22, #101012); }
  .t1 { background: linear-gradient(135deg, #1f3a35, #101012); }
  .cal { margin-bottom: 28px; padding: 24px; border: 1px solid #ffffff1a; border-radius: 32px; background: #101012; }
  .hd { display: flex; justify-content: space-between; align-items: baseline; }
  .hd h2 { margin: 0; font-size: 22px; font-weight: 700; }
  .hd span { color: var(--muted); font-size: 13px; }
  .cal ul { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 12px; margin: 16px 0 0; padding: 0; list-style: none; }
  .cal a { display: grid; gap: 10px; padding: 14px; border-radius: 22px; background: #17171a; transition: transform 0.25s, background 0.2s; }
  .cal a:hover { transform: translate3d(0, -2px, 0); background: #1f1f23; }
  .d { display: flex; align-items: baseline; gap: 6px; }
  .d b { font-size: 28px; font-weight: 800; letter-spacing: -0.04em; }
  .d small { color: var(--muted); font-size: 12px; }
  .cal img { width: 100%; aspect-ratio: 3 / 4; border-radius: 12px; object-fit: cover; background: #26262a; }
  .n { font-size: 13px; font-weight: 600; overflow-wrap: anywhere; }
  @media (max-width: 900px) { .spot { grid-template-columns: minmax(0, 1fr); } .side { grid-template-rows: none; grid-template-columns: repeat(2, minmax(0, 1fr)); } .hero { min-height: 280px; } }
</style>
