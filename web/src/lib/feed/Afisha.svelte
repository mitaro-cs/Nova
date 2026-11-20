<script lang="ts">
  import { getDetail } from '$lib/discover/details';
  import PosterRow from '$lib/discover/PosterRow.svelte';
  import { isAnime } from '$lib/library';
  import type { Card, ExtCard, ExtDetail } from '$lib/types';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';

  // «Афиша» страницы «Что смотреть»: фильм дня на кадре из фильма, «Спросите Nova» (запрос словами и чипы),
  // подборки по идее и «Вы не досмотрели». Берём свои фильмы и подборки IMDb вместе (всё описано по IMDb);
  // кадр у фильма из ленты подгружаем отдельно. Считается тут, без новых запросов к серверу, кроме кадров.
  let { items, ext = [] }: { items: Card[]; ext?: ExtCard[] } = $props();

  type Entry = {
    key: string;
    title: string;
    year: number | null;
    genres: string[];
    runtime: number | null;
    kind: 'movie' | 'series';
    match: number;
    backdrop: string | null;
    href: string;
    resume: boolean;
    card?: Card;
    ext?: ExtCard;
  };

  // Свои фильмы - только описанные по IMDb (с описанием или оценкой): остальные без картинок и с чужими годами.
  const open = (c: Card) => c.status !== 'dropped' && c.status !== 'watched' && (!!c.overview || c.rating != null);
  const openExt = (e: ExtCard) => e.mark !== 'dropped' && e.mark !== 'watched' && !!e.poster;
  const has = (e: Entry, ...stems: string[]) => e.genres.some((g) => stems.some((s) => g.toLowerCase().includes(s)));

  const entries = $derived.by<Entry[]>(() => {
    const own = items.filter(open).map<Entry>((card) => ({
      key: `t${card.id}`, title: card.title, year: card.year, genres: card.genres, runtime: card.runtime, kind: card.kind,
      match: card.matchPct ?? 0, backdrop: card.backdrop, href: `/title/${card.id}`, resume: card.status === 'watching' && !!card.resume, card,
    }));
    const seen = new Set<string>();
    const outer = ext.filter((e) => !e.local && openExt(e) && !seen.has(e.id) && !!seen.add(e.id)).map<Entry>((e) => ({
      key: `x${e.id}`, title: e.title, year: e.year, genres: e.genres, runtime: null, kind: e.kind,
      match: Math.round((e.rating ?? 0) * 10), backdrop: backs[e.id]?.backdrop ?? null, href: `/ext/${e.id}`, resume: false, ext: e,
    }));
    return [...own, ...outer];
  });

  // Кадры и описания фильмов из ленты: берём у первых кандидатов, пока не найдётся достаточно с кадром.
  let backs = $state<Record<string, Pick<ExtDetail, 'backdrop' | 'overview' | 'runtime'>>>({});
  $effect(() => {
    const want = ext.filter((e) => !e.local && openExt(e)).sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0)).slice(0, 8);
    let live = true;
    (async () => {
      for (const e of want) {
        if (!live) return;
        if (backs[e.id]) continue;
        try {
          const d = await getDetail(e.id);
          if (live && d.backdrop) backs[e.id] = { backdrop: d.backdrop, overview: d.overview, runtime: d.runtime };
        } catch {
          /* кадра нет - этот фильм героем не станет */
        }
      }
    })();
    return () => (live = false);
  });

  // ---------- фильм дня ----------
  let skipped = $state<string[]>([]);
  const candidates = $derived(entries.filter((e) => e.backdrop && !skipped.includes(e.key)).sort((a, b) => b.match - a.match));
  const day = $derived(candidates[0] ?? null);
  const dayText = $derived(day?.ext ? backs[day.ext.id]?.overview ?? null : day?.card?.overview ?? null);
  const dayRuntime = $derived(day ? day.runtime ?? (day.ext ? backs[day.ext.id]?.runtime ?? null : null) : null);
  const lead = $derived(
    day ? [day.kind === 'series' ? 'Сериал' : 'Фильм', day.year, day.genres.slice(0, 2).join(', '), dayRuntime ? `${Math.floor(dayRuntime / 60)} ч ${dayRuntime % 60} мин` : null].filter(Boolean).join(' · ') : '',
  );

  // ---------- подборки по идее ----------
  type Idea = { id: string; name: string; hint: string; pick: (e: Entry) => boolean };
  const IDEAS: Idea[] = [
    { id: 'short', name: 'Один вечер - один фильм', hint: 'Не длиннее 100 минут', pick: (e) => e.kind === 'movie' && !!e.runtime && e.runtime <= 100 },
    { id: 'think', name: 'Чтобы подумать', hint: 'Фантастика, детективы и триллеры', pick: (e) => has(e, 'фантаст', 'детектив', 'триллер') },
    { id: 'rain', name: 'Дождь за окном', hint: 'Тихие драмы и мелодрамы', pick: (e) => has(e, 'драм', 'мелодрам') },
    { id: 'night', name: 'Тёмное кино после полуночи', hint: 'Ужасы и мрачные триллеры', pick: (e) => has(e, 'ужас', 'триллер') },
    { id: 'family', name: 'Всей семьёй', hint: 'Мультфильмы и семейное кино', pick: (e) => has(e, 'мульт', 'семейн') },
    { id: 'serial', name: 'Сериал на неделю', hint: 'Сериалы, которые вы ещё не начали', pick: (e) => e.kind === 'series' },
  ];
  const best = (a: Entry, b: Entry) => b.match - a.match;
  const ideas = $derived(IDEAS.map((i) => ({ ...i, list: entries.filter(i.pick).sort(best) })).filter((i) => i.list.length >= 2));

  // ---------- «Спросите Nova» ----------
  const CHIPS = ['Посмешить', 'Подумать', 'Не больше 90 минут', 'Страшное', 'Аниме'];
  let ask = $state('');
  let idea = $state('');

  /** Слова запроса → условия. Что не поняли, ищем в названии. */
  function parse(q: string): ((e: Entry) => boolean) | null {
    const t = q.toLowerCase().trim();
    if (!t) return null;
    const rules: ((e: Entry) => boolean)[] = [];
    const m = /(\d{2,3})\s*мин/.exec(t);
    if (m) rules.push((e) => !!e.runtime && e.runtime <= Number(m[1]));
    else if (/коротк|быстр/.test(t)) rules.push((e) => !!e.runtime && e.runtime <= 100);
    if (/длинн|эпичн/.test(t)) rules.push((e) => !!e.runtime && e.runtime >= 140);
    if (/смеш|комед|весел/.test(t)) rules.push((e) => has(e, 'комед'));
    if (/подума|умн|головоломк/.test(t)) rules.push((e) => has(e, 'фантаст', 'детектив', 'триллер'));
    if (/страш|ужас|жутк/.test(t)) rules.push((e) => has(e, 'ужас'));
    if (/космос|фантаст/.test(t)) rules.push((e) => has(e, 'фантаст'));
    if (/драм|груст|трогат/.test(t)) rules.push((e) => has(e, 'драм'));
    if (/боевик|экшен|драк/.test(t)) rules.push((e) => has(e, 'боевик'));
    if (/мульт|дет(ям|ск)/.test(t)) rules.push((e) => has(e, 'мульт', 'семейн'));
    if (/аниме/.test(t)) rules.push((e) => (e.card ? isAnime(e.card) : has(e, 'аниме')));
    if (/сериал/.test(t)) rules.push((e) => e.kind === 'series');
    if (!rules.length) return (e) => e.title.toLowerCase().includes(t) || (e.card?.originalTitle ?? e.ext?.original ?? '').toLowerCase().includes(t);
    return (e) => rules.every((r) => r(e));
  }
  const found = $derived.by(() => {
    const rule = parse(ask);
    if (rule) return { title: `По запросу «${ask.trim()}»`, list: entries.filter(rule).sort(best).slice(0, 18) };
    const i = ideas.find((x) => x.id === idea);
    return i ? { title: i.name, list: i.list.slice(0, 18) } : null;
  });

  // ---------- ряды ----------
  const resume = $derived(entries.filter((e) => e.resume).slice(0, 18));
  const fresh = $derived(entries.filter((e) => e.card).sort((a, b) => b.card!.addedAt - a.card!.addedAt).slice(0, 18));
  const popular = $derived(entries.filter((e) => e.ext).sort(best).slice(0, 18));

  const tiles = (list: Entry[]) => list.map((e) => (e.card ? { card: e.card } : { ext: e.ext! }));
  const collage = (list: Entry[]) => list.slice(0, 3).map((e) => e.card?.poster ?? e.ext?.poster ?? null);
</script>

<div class="af">
  <form class="ask" role="search" aria-label="Спросите Nova" onsubmit={(e) => (e.preventDefault(), (idea = ''))}>
    <span class="spark"><Icon name="sparkle" size={20} /></span>
    <input bind:value={ask} oninput={() => (idea = '')} placeholder="Опишите словами: «что-нибудь короткое про космос»" aria-label="Что хотите посмотреть" maxlength="80" />
    <span class="chips">{#each CHIPS as c (c)}<button type="button" onclick={() => ((ask = c), (idea = ''))}>{c}</button>{/each}</span>
  </form>

  {#if found}
    <section class="found" aria-label="Подобрано">
      {#if found.list.length}
        <PosterRow title={found.title} items={tiles(found.list)} />
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
        <span class="tag">{day.card?.matchPct ? `Фильм дня · совпадение ${day.card.matchPct}%` : 'Фильм дня'}</span>
        <h2>{day.title}</h2>
        <p>{lead}</p>
        {#if dayText}<p class="ov">{dayText}</p>{/if}
        <div class="btns">
          <a class="btn primary big" href={day.href}>Смотреть</a>
          <a class="btn big" href="{day.href}#trailer">Трейлер</a>
          <button class="btn big" type="button" onclick={() => (skipped = [...skipped, day.key])}>Не сегодня</button>
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
                {#each collage(i.list) as src, k (k)}{#if src}<img use:fadeIn {src} alt="" style:--k={k} loading="lazy" decoding="async" />{/if}{/each}
              </span>
              <span class="nm"><b>{i.name}</b><small>{i.hint}</small></span>
              <em>{i.list.length}</em>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if resume.length}<PosterRow title="Вы не досмотрели" items={tiles(resume)} />{/if}
  {#if fresh.length}<PosterRow title="Новое в библиотеке" items={tiles(fresh)} />{/if}
  {#if popular.length}<PosterRow title="Популярное на IMDb" items={tiles(popular)} />{/if}
</div>

<style>
  .af { display: grid; grid-template-columns: minmax(0, 1fr); gap: 26px; padding: 0 var(--gutter); }
  .ask { display: flex; align-items: center; gap: 14px; padding: 10px 14px 10px 10px; border: 1px solid #ffffff33; border-radius: 999px; background: #101012; }
  .spark { display: grid; place-items: center; flex: none; width: 44px; height: 44px; border-radius: 50%; background: #fff; color: #0a0a0b; }
  .ask input { flex: 1; min-width: 0; border: 0; outline: 0; background: none; color: #fff; font: inherit; font-size: 17px; }
  .ask input::placeholder { color: #a1a1aa; }
  .ask:focus-within { border-color: #fff; }
  .chips { display: flex; flex: none; gap: 8px; }
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
  .day .ov { display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; max-width: 58ch; font-size: 15px; color: #c4c4cc; }
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
  @media (max-width: 1280px) { .chips { display: none; } }
  @media (max-width: 640px) { .day { border-radius: 28px; } .af { gap: 20px; } .ideas { padding: 16px; border-radius: 26px; } }
</style>
