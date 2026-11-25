<script lang="ts">
  import { api } from '$lib/api';
  import { cubicOut } from 'svelte/easing';
  import { fade } from 'svelte/transition';
  import { getDetail } from '$lib/discover/details';
  import PosterRow from '$lib/discover/PosterRow.svelte';
  import { isAnime } from '$lib/library';
  import type { Card, ExtCard, ExtDetail } from '$lib/types';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';

  // Подбор «Спросите Nova» (выбор владельца 25.11): сверху большой вопрос по центру на стене постеров (это `h1` страницы),
  // ниже настроения плитками с кадром подходящего фильма, «Фильм дня» - кадр слева, описание справа.
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
    day ? [day.kind === 'series' ? 'Сериал' : 'Фильм', day.year, day.genres.slice(0, 2).join(', '), dayRuntime ? [Math.floor(dayRuntime / 60) ? `${Math.floor(dayRuntime / 60)} ч` : '', dayRuntime % 60 ? `${dayRuntime % 60} мин` : ''].filter(Boolean).join(' ') : null].filter(Boolean).join(' · ') : '',
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
  // Настроения словами (просьба владельца 25.11: «посмеяться и т.п.» должны работать): каждое - набор жанров (любой
  // подходит), подпись «как поняли» и жанр для поиска по IMDb (сервер ищет по одному жанру - `discover::genre_query`).
  const CHIPS = ['Посмеяться', 'Подумать', 'Поплакать', 'Пощекотать нервы', 'Расслабиться', 'Вдохновиться', 'Всей семьёй', 'До 90 минут', 'Аниме'];
  let ask = $state('');
  let idea = $state('');
  // Подборка по идее открывается под карточкой подборок, запрос словами - под строкой запроса.
  const fromIdea = $derived(!!idea && !ask.trim());

  type Mood = { re: RegExp; tag: string; stems: string[]; web: string };
  const MOODS: Mood[] = [
    { re: /посме|смеш|смея|ржа|угар|весел|комед|юмор/, tag: 'смешное', stems: ['комед'], web: 'комедия' },
    { re: /подума|умн|мозг|философ|глубок|заставля|головолом/, tag: 'чтобы подумать', stems: ['фантаст', 'детектив', 'триллер', 'драм'], web: 'фантастика' },
    { re: /поплак|груст|слёз|слез|трогат|тоск|до слёз/, tag: 'трогательное', stems: ['драм', 'мелодрам'], web: 'драма' },
    { re: /пощекот|нерв|страш|ужас|жутк|пуга|хоррор|мурашк/, tag: 'страшное', stems: ['ужас', 'триллер'], web: 'ужасы' },
    { re: /расслаб|отдохн|лёгк|легк|уютн|ненапряж|фоном|добр/, tag: 'лёгкое', stems: ['комед', 'приключ', 'мульт', 'семейн'], web: 'приключения' },
    { re: /вдохнов|мотивир|реальн|настоящ|биограф|по реальным/, tag: 'вдохновляющее', stems: ['биограф', 'спорт', 'истор'], web: 'биография' },
    { re: /динамич|экшен|экшн|боевик|драк|перестрел|взрыв|адреналин|погон/, tag: 'боевик', stems: ['боевик'], web: 'боевик' },
    { re: /романт|любов|мелодрам|свидан|на двоих/, tag: 'про любовь', stems: ['мелодрам', 'романт'], web: 'мелодрама' },
    { re: /космос|фантаст|будущ|робот|инопланет|звёзд|звезд/, tag: 'фантастика', stems: ['фантаст'], web: 'фантастика' },
    { re: /мульт|дет(ям|ск|ей)|семь|семейн|ребён|ребен/, tag: 'для всей семьи', stems: ['мульт', 'семейн'], web: 'семейный' },
    { re: /детектив|расследов|убийств|загадк|тайн/, tag: 'детектив', stems: ['детектив'], web: 'детектив' },
    { re: /триллер|напряж|саспенс/, tag: 'триллер', stems: ['триллер'], web: 'триллер' },
    { re: /фэнтези|фентези|магия|волшеб|дракон/, tag: 'фэнтези', stems: ['фэнтези'], web: 'фэнтези' },
    { re: /приключ|путешеств/, tag: 'приключения', stems: ['приключ'], web: 'приключения' },
    { re: /криминал|мафи|бандит|ограблен|гангстер/, tag: 'криминал', stems: ['криминал'], web: 'криминал' },
    { re: /войн|военн/, tag: 'про войну', stems: ['воен'], web: 'военный' },
    { re: /истори(я|ческ)/, tag: 'историческое', stems: ['истор'], web: 'исторический' },
    { re: /документ/, tag: 'документальное', stems: ['документ'], web: 'документальный' },
    { re: /мюзикл|музык|песн/, tag: 'музыкальное', stems: ['мюзикл', 'музык'], web: 'мюзикл' },
    { re: /спорт|бокс|футбол/, tag: 'про спорт', stems: ['спорт'], web: 'спортивный' },
    { re: /вестерн|ковбо/, tag: 'вестерн', stems: ['вестерн'], web: 'вестерн' },
  ];
  // Служебные слова не ищем в названиях (\b в JS не работает с кириллицей - поэтому набор слов).
  const STOP = new Set(['что', 'нибудь', 'что-нибудь', 'чтобы', 'хочу', 'хочется', 'про', 'без', 'для', 'фильм', 'фильмы', 'фильмов', 'кино', 'посмотреть', 'смотреть', 'какой', 'какое', 'какую', 'интересное', 'интересный', 'хороший', 'хорошее', 'мне', 'нам', 'вечер', 'вечером', 'сегодня', 'можно', 'бы', 'давай', 'найди', 'покажи', 'посоветуй', 'всей', 'вместе', 'минут', 'больше', 'меньше', 'чем', 'или', 'под', 'настроение']);
  type Parsed = { rule: ((e: Entry) => boolean) | null; rest: string; tags: string[]; web: string };
  /** Слова запроса → условия (настроение и жанр, длина, годы, тип) и подписи «как поняли». Что осталось - ищем в названии. */
  function parse(q: string): Parsed {
    const t = q.toLowerCase().replace(/ё/g, 'е').trim();
    if (!t) return { rule: null, rest: '', tags: [], web: '' };
    const rules: ((e: Entry) => boolean)[] = [];
    const tags: string[] = [];
    const webs: string[] = [];
    let rest = t;
    const eat = (re: RegExp) => (rest = rest.replace(new RegExp(re.source, 'g'), ' '));
    const m = /(\d{2,3})\s*мин/.exec(t);
    if (m) { rules.push((e) => !!e.runtime && e.runtime <= Number(m[1])); tags.push(`до ${m[1]} минут`); eat(/(не )?(больше|дольше|длиннее|до)?\s*\d{2,3}\s*мин\S*/); }
    else if (/коротк|быстр|недолг|на вечер/.test(t)) { rules.push((e) => !e.runtime || e.runtime <= 100); tags.push('недлинное'); eat(/коротк\S*|быстр\S*|недолг\S*|на вечер/); }
    if (/длинн|эпичн/.test(t)) { rules.push((e) => !!e.runtime && e.runtime >= 140); tags.push('длинное'); eat(/длинн\S*|эпичн\S*/); }
    const dec = /(\d0)-?(х|е)/.exec(t);
    if (dec) { const d = Number(dec[1]); const from = d >= 30 ? 1900 + d : 2000 + d; rules.push((e) => !!e.year && e.year >= from && e.year < from + 10); tags.push(`${dec[1]}-е`); eat(/\d0-?(х|е)\S*/); }
    const yr = /(19\d\d|20\d\d)/.exec(t);
    if (yr) { rules.push((e) => e.year === Number(yr[1])); tags.push(yr[1]); eat(/(19|20)\d\d/); }
    if (/нов(ое|инк|ый|ые)|свеж/.test(t)) { const y = new Date().getFullYear() - 2; rules.push((e) => !!e.year && e.year >= y); tags.push('новое'); eat(/нов\S*|свеж\S*/); }
    if (/классик|стар(ое|ый|ые)/.test(t)) { rules.push((e) => !!e.year && e.year <= 1995); tags.push('классика'); eat(/классик\S*|стар\S*/); }
    for (const mood of MOODS) {
      if (!mood.re.test(t)) continue;
      rules.push((e) => has(e, ...mood.stems));
      if (!tags.includes(mood.tag)) tags.push(mood.tag);
      webs.push(mood.web);
      rest = rest.replace(/\S+/g, (w) => (mood.re.test(w) ? ' ' : w));
    }
    if (/аниме/.test(t)) { rules.push((e) => (e.card ? isAnime(e.card) : has(e, 'аниме'))); tags.push('аниме'); webs.push('аниме'); eat(/аниме/); }
    const series = /сериал/.test(t);
    if (series) { rules.push((e) => e.kind === 'series'); tags.push('сериалы'); eat(/сериал\S*/); }
    const words = rest.replace(/[^\p{L}\p{N}\s-]/gu, ' ').split(/\s+/).filter((w) => w.length > 2 && !STOP.has(w));
    rest = words.join(' ');
    if (words.length) {
      const name = (e: Entry) => `${e.title} ${e.card?.originalTitle ?? e.ext?.original ?? ''}`.toLowerCase().replace(/ё/g, 'е');
      rules.push((e) => words.every((w) => name(e).includes(w)));
      tags.push(`в названии: «${rest}»`);
    }
    // По IMDb: название, если осталось; иначе первый понятый жанр (с «сериал», если просили сериалы).
    const web = rest || (webs[0] ? `${webs[0]}${series ? ' сериал' : ''}` : '');
    return { rule: rules.length ? (e) => rules.every((r) => r(e)) : null, rest, tags, web };
  }
  const parsed = $derived(parse(ask));
  const found = $derived.by(() => {
    if (parsed.rule) return { title: 'У вас на Nova', list: entries.filter(parsed.rule).sort(best).slice(0, 18) };
    const i = ideas.find((x) => x.id === idea);
    return i ? { title: i.name, list: i.list.slice(0, 18) } : null;
  });

  // Своих мало или в запросе есть название - ищем ещё по всему IMDb (через полсекунды после ввода).
  let web = $state<ExtCard[]>([]);
  let webBusy = $state(false);
  $effect(() => {
    const q = parsed.web.trim();
    const need = ask.trim().length >= 2 && !!q && (!!parsed.rest || (found?.list.length ?? 0) < 12);
    if (!need || q.length < 2) { web = []; return; }
    let live = true;
    webBusy = true;
    const t = setTimeout(() => {
      api<ExtCard[]>(`/api/discover/search?q=${encodeURIComponent(q)}`).then(
        (r) => { if (live) web = r.filter((e) => e.poster); },
        () => { if (live) web = []; },
      ).finally(() => { if (live) webBusy = false; });
    }, 450);
    return () => { live = false; clearTimeout(t); };
  });

  // ---------- плитки настроений и стена постеров ----------
  const TILES: { label: string; hint: string }[] = [
    { label: 'Посмеяться', hint: 'комедии, лёгкое' },
    { label: 'Подумать', hint: 'сюжет с загадкой' },
    { label: 'Поплакать', hint: 'драмы до мурашек' },
    { label: 'Пощекотать нервы', hint: 'триллеры, ужасы' },
    { label: 'Расслабиться', hint: 'приключения, уют' },
    { label: 'Вдохновиться', hint: 'по реальным событиям' },
  ];
  const SMALL = CHIPS.filter((c) => !TILES.some((t) => t.label === c));
  const artOf = (e: Entry) => e.backdrop ?? null;
  const posterOf = (e: Entry) => e.card?.poster ?? e.ext?.poster ?? null;
  // Кадр плитки - лучший по совпадению фильм этого настроения; разные плитки - разные фильмы.
  const tileArt = $derived.by(() => {
    const used = new Set<string>();
    return TILES.map((t) => {
      const mood = MOODS.find((m) => m.re.test(t.label.toLowerCase()));
      const fits = entries.filter((e) => !used.has(e.key) && (!mood || has(e, ...mood.stems))).sort(best);
      const pick = fits.find(artOf) ?? fits.find(posterOf);
      if (pick) used.add(pick.key);
      return pick ? { src: artOf(pick) ?? posterOf(pick), frame: !!artOf(pick) } : null;
    });
  });
  const wall = $derived(entries.map(posterOf).filter((x): x is string => !!x).slice(0, 30));

  // ---------- ряды ----------
  const resume = $derived(entries.filter((e) => e.resume).slice(0, 18));
  const fresh = $derived(entries.filter((e) => e.card).sort((a, b) => b.card!.addedAt - a.card!.addedAt).slice(0, 18));
  const popular = $derived(entries.filter((e) => e.ext).sort(best).slice(0, 18));

  const tiles = (list: Entry[]) => list.map((e) => (e.card ? { card: e.card } : { ext: e.ext! }));
  const collage = (list: Entry[]) => list.slice(0, 3).map((e) => e.card?.poster ?? e.ext?.poster ?? null);
</script>

{#snippet results()}
  {#key found?.title}
    <section class="found" aria-label="Подобрано" in:fade={{ duration: 320, easing: cubicOut }}>
      {#if found?.list.length}
        <PosterRow title={found.title} items={tiles(found.list)} />
      {/if}
      {#if web.length}
        <PosterRow title={found?.list.length ? 'Ещё на IMDb' : 'Нашли на IMDb'} items={web.slice(0, 18)} />
      {/if}
      {#if !found?.list.length && !web.length}
        <p class="none">{webBusy ? 'Ищем по IMDb…' : `По запросу «${ask.trim()}» ничего не нашли. Попробуйте жанр, год или название: «комедия 90-х», «до 90 минут».`}</p>
      {/if}
    </section>
  {/key}
{/snippet}

<div class="af">
  <section class="top" aria-label="Спросите Nova">
    {#if wall.length >= 10}
      <div class="wall" aria-hidden="true">{#each wall as src, i (i)}<img use:fadeIn {src} alt="" loading="lazy" decoding="async" />{/each}</div>
    {/if}
    <div class="center">
      <h1>Что посмотрим сегодня?</h1>
      <p class="sub">Напишите как другу - Nova поймёт настроение, время и эпоху.</p>
      <form class="ask" role="search" aria-label="Спросите Nova" onsubmit={(e) => (e.preventDefault(), (idea = ''))}>
        <div class="field">
          <span class="spark"><Icon name="sparkle" size={20} /></span>
          <input bind:value={ask} oninput={() => (idea = '')} placeholder="посмеяться, но недолго · космос 90-х" aria-label="Что хотите посмотреть" maxlength="80" />
          {#if ask}<button class="clear" type="button" aria-label="Очистить" onclick={() => (ask = '')}><Icon name="close" size={16} /></button>{/if}
        </div>
        <div class="moods" role="group" aria-label="Настроение">
          {#each SMALL as c (c)}<button type="button" aria-pressed={ask === c} onclick={() => ((ask = ask === c ? '' : c), (idea = ''))}>{c}</button>{/each}
        </div>
        {#if parsed.tags.length}
          <p class="got" aria-live="polite"><span>Поняли так:</span>{#each parsed.tags as t (t)}<b>{t}</b>{/each}</p>
        {/if}
      </form>
    </div>
  </section>

  <ul class="tiles" role="group" aria-label="Настроения">
    {#each TILES as t, i (t.label)}
      {@const a = tileArt[i]}
      <li>
        <button type="button" aria-pressed={ask === t.label} onclick={() => ((ask = ask === t.label ? '' : t.label), (idea = ''))}>
          {#if a?.src}<img use:fadeIn class:po={!a.frame} src={a.src} alt="" loading="lazy" decoding="async" />{/if}
          <b>{t.label}</b><small>{t.hint}</small>
        </button>
      </li>
    {/each}
  </ul>

  {#if found && !fromIdea}
    {@render results()}
  {/if}

  {#if day}
    <section class="day" aria-label="Фильм дня">
      <div class="pic"><img use:fadeIn src={day.backdrop} alt="" decoding="async" /></div>
      <div class="txt">
        <span class="tag">Фильм дня · для вас</span>
        <h2>{day.title}</h2>
        <p>{lead}</p>
        {#if day.match >= 50}<p class="why"><b>{day.match}% совпадение</b>{day.card?.matchPct ? ' по вашим оценкам' : ' по оценке IMDb'}</p>{/if}
        {#if dayText}<p class="ov">{dayText}</p>{/if}
        <div class="btns">
          <a class="btn primary big" href={day.href}>Смотреть</a>
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
    {#if found && fromIdea}{@render results()}{/if}
  {/if}

  {#if resume.length}<PosterRow title="Вы не досмотрели" items={tiles(resume)} />{/if}
  {#if fresh.length}<PosterRow title="Новое в библиотеке" items={tiles(fresh)} />{/if}
  {#if popular.length}<PosterRow title="Популярное на IMDb" items={tiles(popular)} />{/if}
</div>

<style>
  .af { display: grid; grid-template-columns: minmax(0, 1fr); gap: 26px; padding: 0 var(--gutter); }
  /* Верх - во всю ширину окна: стена постеров (только transform), по центру вопрос. */
  .top { position: relative; isolation: isolate; display: grid; place-items: center; min-height: clamp(400px, 56vh, 560px); margin: 0 calc(-1 * var(--gutter)); padding: calc(var(--header) + 24px) var(--gutter) 70px; overflow: hidden; text-align: center; }
  .wall { position: absolute; inset: -40px; z-index: -2; display: grid; grid-template-columns: repeat(10, minmax(0, 1fr)); gap: 12px; opacity: 0.32; transform: rotate(-6deg) scale(1.1); }
  .wall img { width: 100%; max-width: none; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 12px; }
  .top::after { content: ''; position: absolute; inset: 0; z-index: -1; background: radial-gradient(60% 70% at 50% 50%, #000000eb 0%, #000000ad 55%, #00000066 100%), linear-gradient(0deg, var(--bg, #000) 0%, #00000000 34%); }
  .center { display: grid; justify-items: center; width: min(900px, 100%); min-width: 0; }
  h1 { margin: 0; font-size: clamp(38px, 5.6vw, 76px); font-weight: 800; letter-spacing: -0.05em; line-height: 1.02; text-wrap: balance; }
  .sub { margin: 14px 0 26px; color: #ffffffb3; font-size: clamp(15px, 1.4vw, 18px); }
  .ask { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; width: 100%; min-width: 0; justify-items: center; }
  .field { display: flex; align-items: center; gap: 14px; width: 100%; padding: 9px 10px 9px 9px; border: 1px solid #ffffff38; border-radius: 999px; background: #121214; box-shadow: 0 20px 60px #000; transition: border-color 0.2s; text-align: left; }
  .field:focus-within { border-color: #fff; }
  .spark { display: grid; place-items: center; flex: none; width: 48px; height: 48px; border-radius: 50%; background: #fff; color: #0a0a0b; }
  .field input { flex: 1; min-width: 0; border: 0; outline: 0; background: none; color: #fff; font: inherit; font-size: clamp(16px, 1.5vw, 19px); }
  .field input::placeholder { color: #a1a1aa; }
  .clear { display: grid; place-items: center; flex: none; width: 36px; height: 36px; border: 0; border-radius: 50%; background: #ffffff14; color: #fff; cursor: pointer; }
  .moods { display: flex; justify-content: center; gap: 8px; max-width: 100%; overflow-x: auto; scrollbar-width: none; padding: 2px; margin: -2px; }
  .moods::-webkit-scrollbar { display: none; }
  .moods button { flex: none; height: 38px; padding: 0 16px; border: 1px solid #ffffff24; border-radius: 999px; background: #17171ab3; color: #fff; font: inherit; font-size: 14px; font-weight: 600; white-space: nowrap; cursor: pointer; transition: background 0.2s, color 0.2s; }
  .moods button:hover { background: #26262a; }
  .moods button[aria-pressed='true'] { background: #fff; border-color: #fff; color: #0a0a0b; }
  .got { display: flex; flex-wrap: wrap; justify-content: center; align-items: center; gap: 6px; margin: 0; color: #a1a1aa; font-size: 13px; }
  .got b { max-width: 100%; overflow-wrap: anywhere; padding: 3px 10px; border-radius: 999px; background: #ffffff14; color: #fff; font-weight: 600; }
  /* Настроения плитками с кадром: заходят на низ верхнего блока. */
  .tiles { position: relative; z-index: 1; display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 14px; margin: -54px 0 0; padding: 0; list-style: none; }
  .tiles button { position: relative; isolation: isolate; display: block; width: 100%; height: clamp(120px, 11vw, 170px); padding: 0; border: 0; border-radius: 20px; overflow: hidden; background: #17171a; color: #fff; font: inherit; text-align: left; cursor: pointer; }
  .tiles button::after { content: ''; position: absolute; inset: 0; z-index: -1; background: linear-gradient(0deg, #000000e6 8%, #00000026 72%); }
  .tiles button::before { content: ''; position: absolute; inset: 0; z-index: 2; border-radius: inherit; box-shadow: inset 0 0 0 2px #fff; opacity: 0; transition: opacity 0.2s; }
  .tiles button:hover::before, .tiles button:focus-visible::before, .tiles button[aria-pressed='true']::before { opacity: 1; }
  .tiles img { position: absolute; inset: 0; z-index: -2; width: 100%; max-width: none; height: 100%; object-fit: cover; transition: transform 0.5s var(--ease, ease); }
  .tiles img.po { object-position: 50% 20%; }
  .tiles button:hover img { transform: scale(1.05); }
  .tiles b { position: absolute; left: 16px; right: 12px; bottom: 32px; font-size: clamp(16px, 1.4vw, 20px); font-weight: 800; letter-spacing: -0.02em; line-height: 1.1; }
  .tiles small { position: absolute; left: 16px; right: 12px; bottom: 13px; color: #ffffffad; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .none { margin: 0; color: var(--muted); }
  /* Фильм дня: кадр слева, о фильме справа (на узком - друг под другом). */
  .day { display: grid; grid-template-columns: minmax(0, 1.5fr) minmax(0, 1fr); overflow: hidden; border-radius: 24px; background: #111113; }
  .pic { position: relative; min-height: clamp(260px, 26vw, 400px); }
  .pic img { position: absolute; inset: 0; width: 100%; max-width: none; height: 100%; object-fit: cover; object-position: 60% 22%; }
  .txt { display: grid; align-content: center; justify-items: start; gap: 10px; min-width: 0; padding: clamp(22px, 3vw, 40px); }
  .tag { display: inline-block; color: #8c8c96; font-size: 12px; font-weight: 700; letter-spacing: 0.16em; text-transform: uppercase; }
  .day h2 { margin: 0; font-size: clamp(28px, 3.2vw, 44px); font-weight: 800; letter-spacing: -0.045em; line-height: 1.02; overflow-wrap: anywhere; }
  .day p { margin: 0; color: #d4d4d8; font-size: 15px; }
  .day .why b { display: inline-block; margin-right: 6px; padding: 3px 10px; border-radius: 999px; background: #fff; color: #000; font-size: 12px; font-weight: 800; }
  .day .why { color: #a1a1aa; font-size: 13px; }
  .day .ov { display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; max-width: 58ch; color: #c4c4cc; }
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
  @media (max-width: 1000px) { .tiles { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
  @media (max-width: 640px) {
    .af { gap: 20px; }
    .ideas { padding: 16px; border-radius: 26px; }
    .top { min-height: 0; padding: calc(env(safe-area-inset-top) + 72px) var(--gutter) 70px; }
    .wall { grid-template-columns: repeat(5, minmax(0, 1fr)); }
    .moods { justify-content: flex-start; }
    .tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
    .day { grid-template-columns: minmax(0, 1fr); border-radius: 22px; }
    .pic { min-height: 210px; }
  }
</style>
