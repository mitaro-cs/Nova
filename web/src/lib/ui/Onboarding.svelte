<script lang="ts">
  import { fade, fly, scale } from 'svelte/transition';
  import { api, put } from '$lib/api';
  import { motionOn } from '$lib/motion';
  import { session } from '$lib/session.svelte';
  import type { ExtCard } from '$lib/types';
  import Icon from './Icon.svelte';
  import { sound, unlock } from './novaSound';

  // Знакомство после регистрации: три экрана и «кинотитр» в конце. Фильмы для любимого можно выбрать из подборки
  // или найти любой через поиск (тот же каталог, что и в «Поиске»). Экраны плавно сменяют друг друга, на фоне
  // медленно плывёт лента постеров, а в конце сразу стартует экскурсия, которая показывает разделы вживую.
  const choices = ['Боевик', 'Приключения', 'Комедия', 'Драма', 'Фантастика', 'Фэнтези', 'Ужасы', 'Триллер', 'Детектив', 'Анимация'];
  let open = $state(false);
  let step = $state(0);
  let dir = $state(1);
  let busy = $state(false);
  let error = $state('');
  let genres = $state<string[]>([]);
  let films = $state<ExtCard[]>([]);
  let picked = $state<ExtCard[]>([]);
  let query = $state('');
  let results = $state<ExtCard[] | null>(null);
  let searching = $state(false);
  let finale = $state(false);
  let finishTimer: ReturnType<typeof setTimeout> | undefined;
  let tourTimer: ReturnType<typeof setTimeout> | undefined;
  const profile = $derived(session.me?.user.profile?.id);
  const d = $derived(motionOn() && !window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 320 : 0);
  const shown = $derived(query.trim().length > 1 && results ? results : films);
  const chosen = (card: ExtCard) => picked.some((p) => p.id === card.id);

  $effect(() => {
    if (!profile) return;
    try { if (sessionStorage.getItem(`nova-onboarding:${profile}`) !== '1') return; } catch { return; }
    open = true;
    api<{ rows: { items: ExtCard[] }[] }>('/api/discover')
      .then((r) => {
        films = [...new Map(r.rows.flatMap((row) => row.items).filter((f) => f.kind === 'movie').map((f) => [f.id, f])).values()].slice(0, 18);
      })
      .catch(() => {});
  });

  // Поиск любого фильма: через треть секунды после последней буквы.
  $effect(() => {
    const q = query.trim();
    if (q.length < 2) {
      results = null;
      searching = false;
      return;
    }
    searching = true;
    results = null;
    const controller = new AbortController();
    const timer = setTimeout(() => {
      api<ExtCard[]>(`/api/discover/search?q=${encodeURIComponent(q)}`, { signal: controller.signal }).then(
        (r) => {
          results = r.filter((f) => f.kind === 'movie' || f.kind === 'series').slice(0, 24);
          searching = false;
        },
        () => {
          if (!controller.signal.aborted) {
            results = [];
            searching = false;
          }
        },
      );
    }, 350);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  });

  // Звук выбора: каждый следующий жанр - нота выше (мелодия), снять - та же нота ниже и тише.
  const toggleGenre = (g: string) => {
    unlock();
    if (genres.includes(g)) {
      sound('unselect', { k: genres.length - 1 });
      genres = genres.filter((x) => x !== g);
    } else {
      sound('select', { k: genres.length });
      genres = [...genres, g];
    }
  };
  const toggleFilm = (card: ExtCard) => {
    unlock();
    if (chosen(card)) {
      sound('unselect', { k: picked.length - 1 });
      picked = picked.filter((p) => p.id !== card.id);
    } else if (picked.length < 5) {
      sound('fave', { k: picked.length });
      picked = [...picked, card];
    } else error = 'Можно выбрать до пяти любимых произведений.';
  };

  function go(to: number) {
    dir = to > step ? 1 : -1;
    step = to;
  }
  function startTour() {
    clearTimeout(tourTimer);
    tourTimer = setTimeout(() => window.dispatchEvent(new Event('nova-tour')), motionOn() ? 450 : 0);
  }

  $effect(() => () => {
    clearTimeout(finishTimer);
    clearTimeout(tourTimer);
  });

  async function finish() {
    if (!profile || busy || finale) return;
    busy = true;
    error = '';
    try {
      for (const film of picked) {
        const result = await put<{ favorites: string[] }>('/api/me/favorites', { key: `x:${film.id}`, value: true });
        if (session.me) session.me.favorites = result.favorites;
      }
      try { localStorage.setItem(`nova-genres:${profile}`, JSON.stringify(genres)); } catch { /* хранилище недоступно */ }
      try { sessionStorage.removeItem(`nova-onboarding:${profile}`); } catch { /* хранилище недоступно */ }
      window.dispatchEvent(new Event('nova-preferences'));
      // Финальная сцена: галочка прорисовывается, потом окно закрывается и начинается экскурсия.
      finale = true;
      finishTimer = setTimeout(
        () => {
          open = false;
          finale = false;
          startTour();
        },
        motionOn() ? 1700 : 300,
      );
    } catch {
      error = 'Не удалось сохранить выбор. Проверьте соединение и попробуйте снова.';
    } finally {
      busy = false;
    }
  }
  function skip() {
    if (busy) return;
    if (profile) try { sessionStorage.removeItem(`nova-onboarding:${profile}`); } catch { /* хранилище недоступно */ }
    open = false;
    startTour();
  }
</script>

{#if open}
  <div class="veil" transition:fade={{ duration: d }}>
    <div class="aurora" aria-hidden="true"></div>
    <div class="reel" aria-hidden="true">
      {#each [0, 1, 2] as col (col)}
        <div class="col" style:--c={col}>
          {#each [...films, ...films].filter((_, i) => i % 3 === col) as film, i (film.id + ':' + i)}
            {#if film.poster}<img src={film.poster} alt="" loading="lazy" />{/if}
          {/each}
        </div>
      {/each}
    </div>

    <div class="guide" role="dialog" aria-modal="true" aria-label="Знакомство с Nova" tabindex="-1" in:fly={{ y: 30, duration: d + 80 }}>
      {#if finale}
        <div class="finale" in:scale={{ start: 0.85, duration: d }}>
          <svg viewBox="0 0 80 80" width="96" height="96" aria-hidden="true"><circle cx="40" cy="40" r="34" /><path d="M24 41l11 11 21-24" /></svg>
          <h1>Всё готово</h1>
          <p>Подбираем кино под ваш вкус. Сейчас покажем, где что лежит.</p>
        </div>
      {:else}
        <div class="top">
          <span>Знакомство с Nova · {step + 1}/3</span>
          <button class="skip" disabled={busy} onclick={skip}>Пропустить</button>
        </div>
        <div class="progress" aria-hidden="true"><i style:width="{((step + 1) / 3) * 100}%"></i></div>
        <div class="stage">
          {#key step}
            <div class="slide" in:fly={{ x: 44 * dir, duration: d, delay: d / 3 }} out:fade={{ duration: d / 3 }}>
              {#if step === 0}
                <div class="symbol"><span class="ping"></span><Icon name="play" size={32} /></div>
                <h1>{#each ['Истории', 'становятся', 'ближе'] as word, i (word)}<span class="word" style:--i={i}>{word}</span>{' '}{/each}</h1>
                <p>Ищите фильмы, сохраняйте любимое и смотрите вместе с друзьями. Мы поможем найти то, что понравится именно вам.</p>
              {:else if step === 1}
                <h1>Какие жанры вам нравятся?</h1>
                <p>Выберите несколько - мы покажем подходящие фильмы на главной.</p>
                <div class="choices">
                  {#each choices as genre, i (genre)}
                    <button class:on={genres.includes(genre)} aria-pressed={genres.includes(genre)} style:--i={i} onclick={() => toggleGenre(genre)}>{genre}</button>
                  {/each}
                </div>
              {:else}
                <h1>Выберите любимые фильмы</h1>
                <p>Найдите любой фильм или выберите из подборки. Они появятся в вашем профиле, выбор необязателен.</p>
                <label class="find"><Icon name="search" size={18} /><input type="search" bind:value={query} placeholder="Название любого фильма" aria-label="Найти фильм" autocomplete="off" /></label>
                {#if picked.length}
                  <div class="picked" aria-label="Выбрано">
                    {#each picked as film (film.id)}
                      <button class="chip" onclick={() => toggleFilm(film)} transition:scale={{ duration: d / 2 }} title="Убрать из выбранного">{film.title}<Icon name="close" size={13} /></button>
                    {/each}
                  </div>
                {/if}
                {#if searching}
                  <p class="note">Ищем в каталоге…</p>
                {:else if query.trim().length > 1 && results && !results.length}
                  <p class="note">Ничего не нашлось - попробуйте другое название.</p>
                {/if}
                {#if shown.length}
                  <div class="films">
                    {#each shown as film, i (film.id)}
                      <button class:on={chosen(film)} aria-pressed={chosen(film)} style:--i={i % 12} onclick={() => toggleFilm(film)}>
                        {#if film.poster}<img src={film.poster} alt="" loading="lazy" />{:else}<span class="blank"></span>{/if}
                        <span>{film.title}</span>
                        <i class="tick" aria-hidden="true"><Icon name="check" size={14} /></i>
                      </button>
                    {/each}
                  </div>
                {:else if !searching && !query.trim()}
                  <p class="note">Подборка пока недоступна - найдите фильм поиском или выберите любимые позже.</p>
                {/if}
              {/if}
            </div>
          {/key}
        </div>
        {#if error}<p role="alert" class="err">{error}</p>{/if}
        <div class="actions">
          {#if step > 0}<button class="btn" disabled={busy} onclick={() => go(step - 1)}>Назад</button>{/if}
          <button class="btn primary" disabled={busy} onclick={() => (step < 2 ? go(step + 1) : finish())}>{step < 2 ? 'Продолжить' : busy ? 'Сохраняем…' : 'Начать смотреть'}</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 2200;
    display: grid;
    place-items: center;
    padding: 20px;
    background: #000000e6;
    overflow: hidden;
  }
  /* Фон: тёплое свечение и три колонки постеров, плывущие в разные стороны. Только transform и opacity. */
  .aurora {
    position: absolute;
    width: 90vmax;
    height: 90vmax;
    left: 50%;
    top: 50%;
    margin: -45vmax 0 0 -45vmax;
    background: radial-gradient(closest-side, #ffffff22, transparent 70%);
    animation: breathe 9s ease-in-out infinite;
    pointer-events: none;
  }
  .reel {
    position: absolute;
    inset: -10% -4%;
    display: flex;
    justify-content: space-around;
    gap: clamp(10px, 3vw, 40px);
    opacity: 0.13;
    pointer-events: none;
    transform: rotate(-6deg);
  }
  .col {
    display: grid;
    align-content: start;
    gap: clamp(10px, 2vw, 24px);
    width: clamp(110px, 17vw, 240px);
    animation: drift 46s linear infinite alternate;
  }
  .col:nth-child(2) {
    animation-direction: alternate-reverse;
    animation-duration: 58s;
  }
  .col img {
    width: 100%;
    aspect-ratio: 2 / 3;
    border-radius: 14px;
    object-fit: cover;
  }
  .guide {
    position: relative;
    width: min(700px, 100%);
    max-height: 92dvh;
    overflow: auto;
    padding: clamp(22px, 5vw, 40px);
    border: 1px solid #ffffff24;
    border-radius: 26px;
    background: #1b1b1de8;
    box-shadow: 0 50px 140px -30px #000;
    color: #fff;
  }
  .top {
    display: flex;
    justify-content: space-between;
    color: #aaa;
    font-size: 13px;
  }
  .skip {
    border: 0;
    background: none;
    color: #aaa;
    cursor: pointer;
  }
  .skip:hover {
    color: #fff;
  }
  .progress {
    height: 3px;
    margin: 14px 0 4px;
    border-radius: 2px;
    background: #ffffff1f;
    overflow: hidden;
  }
  .progress i {
    display: block;
    height: 100%;
    background: #fff;
    transform-origin: left;
    transition: width 0.5s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .stage {
    display: grid;
  }
  .slide {
    grid-area: 1 / 1;
  }
  h1 {
    margin: 22px 0 10px;
    font-size: clamp(26px, 5vw, 38px);
    line-height: 1.1;
  }
  p {
    color: #bbb;
    line-height: 1.5;
  }
  .word {
    display: inline-block;
    animation: word 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--i) * 140ms + 250ms);
  }
  .symbol {
    position: relative;
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    margin-top: 24px;
    border-radius: 50%;
    background: #fff;
    color: #111;
  }
  .ping {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 2px solid #fff;
    animation: ping 2.2s ease-out infinite;
  }
  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 24px 0;
  }
  .choices button {
    padding: 10px 16px;
    border: 1px solid #555;
    border-radius: 999px;
    background: #292929;
    color: #fff;
    cursor: pointer;
    animation: pop 0.45s cubic-bezier(0.2, 0.9, 0.3, 1.3) both;
    animation-delay: calc(var(--i) * 45ms);
    transition: background 0.2s, transform 0.15s;
  }
  .choices button:active {
    transform: scale(0.94);
  }
  .choices button.on {
    outline: 2px solid #fff;
    background: #444;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 16px 0 10px;
    padding: 0 14px;
    height: 46px;
    border: 1px solid #ffffff2e;
    border-radius: 14px;
    background: #ffffff10;
    color: #aaa;
  }
  .find:focus-within {
    border-color: #fff;
    color: #fff;
  }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    color: #fff;
    font: inherit;
    font-size: 16px;
  }
  .picked {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 10px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 5px 10px;
    border: 0;
    border-radius: 999px;
    background: #fff;
    color: #111;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .note {
    margin: 8px 0;
    font-size: 14px;
  }
  .films {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 10px;
    max-height: 36dvh;
    overflow: auto;
    padding: 2px;
  }
  .films button {
    position: relative;
    padding: 0 0 7px;
    border: 0;
    border-radius: 10px;
    background: #292929;
    color: #fff;
    overflow: hidden;
    cursor: pointer;
    text-align: left;
    animation: pop 0.45s cubic-bezier(0.2, 0.9, 0.3, 1.2) both;
    animation-delay: calc(var(--i) * 35ms);
    transition: transform 0.15s;
  }
  .films button:active {
    transform: scale(0.96);
  }
  .films button.on {
    outline: 2px solid #fff;
    background: #444;
  }
  .films img,
  .films .blank {
    display: block;
    width: 100%;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    background: #1c1c1e;
  }
  .films span:not(.blank) {
    display: block;
    padding: 5px 6px 0;
    font-size: 12px;
    line-height: 1.25;
  }
  .tick {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: #fff;
    color: #111;
    opacity: 0;
    transform: scale(0.4);
    transition: opacity 0.2s, transform 0.25s cubic-bezier(0.2, 0.9, 0.3, 1.4);
  }
  button.on .tick {
    opacity: 1;
    transform: scale(1);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 26px;
  }
  .err {
    color: #ff8a8a;
  }
  .finale {
    display: grid;
    justify-items: center;
    text-align: center;
    padding: 20px 0 8px;
  }
  .finale svg {
    fill: none;
    stroke: #fff;
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .finale circle {
    stroke-dasharray: 214;
    stroke-dashoffset: 214;
    animation: draw 0.7s ease forwards;
  }
  .finale path {
    stroke-dasharray: 60;
    stroke-dashoffset: 60;
    animation: draw 0.5s 0.55s ease forwards;
  }
  :global(.no-motion) .aurora,
  :global(.no-motion) .col,
  :global(.no-motion) .ping,
  :global(.no-motion) .word,
  :global(.no-motion) .choices button,
  :global(.no-motion) .films button {
    animation: none;
  }
  :global(.no-motion) .finale circle,
  :global(.no-motion) .finale path { animation: none; stroke-dashoffset: 0; }
  @media (prefers-reduced-motion: reduce) {
    .aurora, .col, .ping, .word, .choices button, .films button { animation: none; }
    .finale circle, .finale path { animation: none; stroke-dashoffset: 0; }
    .progress i { transition: none; }
  }
  @keyframes breathe {
    50% {
      transform: scale(1.15);
      opacity: 0.7;
    }
  }
  @keyframes drift {
    from {
      transform: translateY(-12%);
    }
    to {
      transform: translateY(12%);
    }
  }
  @keyframes word {
    from {
      opacity: 0;
      transform: translateY(18px);
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.85);
    }
  }
  @keyframes ping {
    from {
      transform: scale(1);
      opacity: 0.8;
    }
    to {
      transform: scale(2);
      opacity: 0;
    }
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
</style>
