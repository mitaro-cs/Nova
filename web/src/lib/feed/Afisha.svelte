<script lang="ts">
  import PosterRow from '$lib/discover/PosterRow.svelte';
  import { isAnime } from '$lib/library';
  import type { Card } from '$lib/types';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';

  // «Афиша» страницы «Что смотреть»: фильм дня на кадре из фильма, «Спросите Nova» (запрос словами и чипы),
  // подборки по идее и «Вы не досмотрели». Всё считается тут из каталога, без запросов к серверу.
  let { items }: { items: Card[] } = $props();

  const open = (c: Card) => c.status !== 'dropped' && c.status !== 'watched';
  const has = (c: Card, ...stems: string[]) => c.genres.some((g) => stems.some((s) => g.toLowerCase().includes(s)));

  // ---------- фильм дня ----------
  let skipped = $state<number[]>([]);
  const candidates = $derived(
    items
      .filter((c) => open(c) && c.backdrop && !skipped.includes(c.id))
      .sort((a, b) => (b.matchPct ?? 0) - (a.matchPct ?? 0) || b.addedAt - a.addedAt),
  );
  const day = $derived(candidates[0] ?? null);
  const lead = $derived(day ? [day.kind === 'series' ? 'Сериал' : 'Фильм', day.year, day.genres.slice(0, 2).join(', '), day.runtime ? `${Math.floor(day.runtime / 60)} ч ${day.runtime % 60} мин` : null].filter(Boolean).join(' · ') : '');

  // ---------- подборки по идее ----------
  type Idea = { id: string; name: string; hint: string; pick: (c: Card) => boolean };
  const IDEAS: Idea[] = [
    { id: 'short', name: 'Один вечер - один фильм', hint: 'Не длиннее 100 минут', pick: (c) => c.kind === 'movie' && !!c.runtime && c.runtime <= 100 },
    { id: 'think', name: 'Чтобы подумать', hint: 'Фантастика, детективы и триллеры', pick: (c) => has(c, 'фантаст', 'детектив', 'триллер') },
    { id: 'rain', name: 'Дождь за окном', hint: 'Тихие драмы и мелодрамы', pick: (c) => has(c, 'драм', 'мелодрам') },
    { id: 'night', name: 'Тёмное кино после полуночи', hint: 'Ужасы и мрачные триллеры', pick: (c) => has(c, 'ужас', 'триллер') },
    { id: 'family', name: 'Всей семьёй', hint: 'Мультфильмы и семейное кино', pick: (c) => has(c, 'мульт', 'семейн') },
    { id: 'serial', name: 'Сериал на неделю', hint: 'Сериалы, которые вы ещё не начали', pick: (c) => c.kind === 'series' && c.status !== 'watching' },
  ];
  const ideas = $derived(
    IDEAS.map((i) => ({ ...i, cards: items.filter((c) => open(c) && i.pick(c)).sort((a, b) => (b.matchPct ?? 0) - (a.matchPct ?? 0)) })).filter((i) => i.cards.length >= 2),
  );

  // ---------- «Спросите Nova» ----------
  const CHIPS = ['Посмешить', 'Подумать', 'Не больше 90 минут', 'Страшное', 'Аниме'];
  let ask = $state('');
  let idea = $state('');

  /** Слова запроса → условия. Что не поняли, ищем в названии. */
  function parse(q: string): ((c: Card) => boolean) | null {
    const t = q.toLowerCase().trim();
    if (!t) return null;
    const rules: ((c: Card) => boolean)[] = [];
    const m = /(\d{2,3})\s*мин/.exec(t);
    if (m) rules.push((c) => !!c.runtime && c.runtime <= Number(m[1]));
    else if (/коротк|быстр/.test(t)) rules.push((c) => !!c.runtime && c.runtime <= 100);
    if (/длинн|эпичн/.test(t)) rules.push((c) => !!c.runtime && c.runtime >= 140);
    if (/смеш|комед|весел/.test(t)) rules.push((c) => has(c, 'комед'));
    if (/подума|умн|головоломк/.test(t)) rules.push((c) => has(c, 'фантаст', 'детектив', 'триллер'));
    if (/страш|ужас|жутк/.test(t)) rules.push((c) => has(c, 'ужас'));
    if (/космос|фантаст/.test(t)) rules.push((c) => has(c, 'фантаст'));
    if (/драм|груст|трогат/.test(t)) rules.push((c) => has(c, 'драм'));
    if (/боевик|экшен|драк/.test(t)) rules.push((c) => has(c, 'боевик'));
    if (/мульт|дет(ям|ск)/.test(t)) rules.push((c) => has(c, 'мульт', 'семейн'));
    if (/аниме/.test(t)) rules.push((c) => isAnime(c));
    if (/сериал/.test(t)) rules.push((c) => c.kind === 'series');
    if (!rules.length) return (c) => c.title.toLowerCase().includes(t) || (c.originalTitle ?? '').toLowerCase().includes(t);
    return (c) => rules.every((r) => r(c));
  }
  const found = $derived.by(() => {
    const rule = parse(ask);
    const base = items.filter(open);
    if (rule) return { title: `По запросу «${ask.trim()}»`, cards: base.filter(rule).sort((a, b) => (b.matchPct ?? 0) - (a.matchPct ?? 0)).slice(0, 18) };
    const i = ideas.find((x) => x.id === idea);
    return i ? { title: i.name, cards: i.cards.slice(0, 18) } : null;
  });

  // ---------- ряды ----------
  const resume = $derived(items.filter((c) => c.status === 'watching' && c.resume).slice(0, 18));
  const fresh = $derived(items.filter(open).sort((a, b) => b.addedAt - a.addedAt).slice(0, 18));

  const tiles = (cards: Card[]) => cards.map((card) => ({ card }));
  const collage = (cards: Card[]) => cards.slice(0, 3);
</script>

<div class="af">
  <form class="ask" role="search" aria-label="Спросите Nova" onsubmit={(e) => (e.preventDefault(), (idea = ''))}>
    <span class="spark"><Icon name="sparkle" size={20} /></span>
    <input bind:value={ask} oninput={() => (idea = '')} placeholder="Опишите словами: «что-нибудь короткое про космос»" aria-label="Что хотите посмотреть" maxlength="80" />
    <span class="chips">{#each CHIPS as c (c)}<button type="button" onclick={() => ((ask = c), (idea = ''))}>{c}</button>{/each}</span>
  </form>

  {#if found}
    <section class="found" aria-label="Подобрано">
      {#if found.cards.length}
        <PosterRow title={found.title} items={tiles(found.cards)} />
      {:else}
        <p class="none">По запросу «{ask.trim()}» ничего не нашли. Попробуйте жанр или длину: «комедия», «до 90 минут».</p>
      {/if}
    </section>
  {/if}

  {#if day}
    <section class="day" aria-label="Фильм дня">
      <img use:fadeIn src={day.backdrop} alt="" decoding="async" />
      <i class="shade" aria-hidden="true"></i>
      <div class="txt">
        <span class="tag">{day.matchPct ? `Фильм дня · совпадение ${day.matchPct}%` : 'Фильм дня'}</span>
        <h2>{day.title}</h2>
        <p>{lead}</p>
        <div class="btns">
          <a class="btn primary big" href="/title/{day.id}">Смотреть</a>
          <a class="btn big" href="/title/{day.id}#trailer">Трейлер</a>
          <button class="btn big" type="button" onclick={() => (skipped = [...skipped, day.id])}>Не сегодня</button>
        </div>
      </div>
    </section>
  {/if}

  {#if ideas.length}
    <section class="ideas" aria-label="Подборки по идее">
      <div class="hd"><h2>Подборки по идее</h2><span>собраны из вашей библиотеки</span></div>
      <ul>
        {#each ideas as i (i.id)}
          <li>
            <button type="button" class:on={idea === i.id && !ask} onclick={() => ((idea = idea === i.id ? '' : i.id), (ask = ''))}>
              <span class="fan" aria-hidden="true">
                {#each collage(i.cards) as c, k (c.id)}<img use:fadeIn src={c.poster} alt="" style:--k={k} loading="lazy" decoding="async" />{/each}
              </span>
              <span class="nm"><b>{i.name}</b><small>{i.hint}</small></span>
              <em>{i.cards.length}</em>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if resume.length}<PosterRow title="Вы не досмотрели" items={tiles(resume)} />{/if}
  {#if fresh.length}<PosterRow title="Новое в библиотеке" items={tiles(fresh)} />{/if}
</div>

<style>
  .af { display: grid; gap: 26px; padding: 0 var(--gutter); }
  .ask { display: flex; align-items: center; gap: 14px; padding: 10px 14px 10px 10px; border: 1px solid #ffffff33; border-radius: 999px; background: #101012; }
  .spark { display: grid; place-items: center; flex: none; width: 44px; height: 44px; border-radius: 50%; background: #fff; color: #0a0a0b; }
  .ask input { flex: 1; min-width: 0; border: 0; outline: 0; background: none; color: #fff; font: inherit; font-size: 17px; }
  .ask input::placeholder { color: #a1a1aa; }
  .ask:focus-within { border-color: #fff; }
  .chips { display: flex; gap: 8px; }
  .chips button { padding: 9px 16px; border: 1px solid #ffffff24; border-radius: 999px; background: #17171a; color: #fff; font: inherit; font-size: 13px; font-weight: 600; white-space: nowrap; cursor: pointer; transition: background 0.2s; }
  .chips button:hover { background: #26262a; }
  .none { margin: 0; color: var(--muted); }
  .day { position: relative; min-height: clamp(340px, 34vw, 460px); overflow: hidden; border: 1px solid #ffffff1a; border-radius: 40px; background: #05070d; }
  .day img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .shade { position: absolute; inset: 0; background: linear-gradient(90deg, #05070dee 0%, #05070db3 38%, #05070d33 70%, transparent 100%), linear-gradient(180deg, transparent 55%, #05070d 100%); }
  .txt { position: absolute; left: clamp(22px, 4vw, 56px); right: 22px; bottom: clamp(24px, 3.4vw, 46px); max-width: 660px; }
  .tag { display: inline-block; padding: 8px 16px; border-radius: 999px; background: #ffffff1f; font-size: 13px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
  .day h2 { margin: 14px 0 0; font-size: clamp(32px, 4.6vw, 60px); font-weight: 800; letter-spacing: -0.045em; line-height: 1; overflow-wrap: anywhere; }
  .day p { margin: 12px 0 0; color: #d4d4d8; font-size: 16px; }
  .btns { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 18px; }
  .big { min-height: 48px; padding-inline: 26px; border-radius: 999px; font-size: 16px; }
  .ideas { padding: 24px; border: 1px solid #ffffff1a; border-radius: 32px; background: #101012; }
  .hd { display: flex; justify-content: space-between; align-items: baseline; gap: 12px; }
  .hd h2 { margin: 0; font-size: 22px; font-weight: 700; }
  .hd span { color: var(--muted); font-size: 13px; }
  .ideas ul { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 330px), 1fr)); gap: 10px; margin: 14px 0 0; padding: 0; list-style: none; }
  .ideas button { display: flex; align-items: center; gap: 14px; width: 100%; padding: 10px 14px 10px 12px; border: 1px solid #ffffff14; border-radius: 22px; background: #17171a; color: inherit; font: inherit; text-align: left; cursor: pointer; transition: transform 0.25s, background 0.2s; }
  .ideas button:hover { transform: translate3d(0, -2px, 0); background: #1f1f23; }
  .ideas button.on { border-color: #fff; }
  .fan { position: relative; flex: none; width: 92px; height: 62px; }
  .fan img { position: absolute; top: 2px; left: calc(var(--k) * 22px); width: 42px; height: 58px; border: 2px solid #17171a; border-radius: 9px; object-fit: cover; background: #26262a; }
  .nm { display: grid; flex: 1; min-width: 0; }
  .nm b { font-size: 15px; }
  .nm small { color: var(--muted); font-size: 12px; }
  .ideas em { color: #c4c4cc; font-size: 12px; font-style: normal; font-weight: 700; }
  @media (max-width: 900px) { .chips { display: none; } }
  @media (max-width: 640px) { .day { border-radius: 28px; } .af { gap: 20px; } .ideas { padding: 16px; border-radius: 26px; } }
</style>
