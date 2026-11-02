<script lang="ts">
  import { goto } from '$app/navigation';
  import { fade, fly } from 'svelte/transition';
  import { motionOn } from '$lib/motion';
  import { session } from '$lib/session.svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  // Экскурсия «вживую»: сама открывает разделы Nova, затемняет страницу и подсвечивает нужное место
  // (баннер, панель разделов, ленту «Что смотреть», поиск, подборки, профиль) с подписью рядом. Шаги без
  // страницы - общие экраны посередине. Запуск: сама после знакомства у нового человека и в любой момент -
  // событие `nova-tour` («Экскурсия по Nova» на странице профиля). Стрелки листают, Esc закрывает.
  type Step = { icon: IconName; title: string; text: string; href?: string; target?: string };
  const ME = '/people/me';
  const STEPS: Step[] = [
    { icon: 'play', title: 'Добро пожаловать в Nova', text: 'Ваш кинозал: фильмы и сериалы с компьютера хозяина, друзья рядом, всё открывается в браузере и на телевизоре. Сейчас мы сами пройдём по разделам и покажем, где что лежит - это минута.' },
    { icon: 'film', title: 'Главная: баннер', text: 'Здесь то, что вы смотрите, новое и рекомендации. «Смотреть» открывает фильм, «+» кладёт в «Хочу», «Не интересно» убирает его из советов.', href: '/', target: '.showcase' },
    { icon: 'sparkle', title: 'Разделы кинотеатра', text: 'Главная, Фильмы, Сериалы, Аниме и Обзор - переключайте полосой. «Фильмы» и «Сериалы» - своё и то, что в планах; «Обзор» - всё кино мира.', href: '/', target: 'nav[aria-label="Разделы кинотеатра"]' },
    { icon: 'home', title: 'Панель разделов', text: 'Главное, Что смотреть, Профиль, Коллекция и Поиск - одни и те же на телефоне (внизу), компьютере (сверху, вместе с поиском и профилем) и телевизоре. Красная точка у профиля - просьбы дружбы и новые сообщения.', href: '/', target: 'nav.tabs, header .pills' },
    { icon: 'squareplay', title: 'Что смотреть', text: 'Лента карточек с кадром из фильма и коротким описанием. Трейлер сам играет у той карточки, что на экране. Закладка - в «Хочу», три точки - «Уже смотрел» или «Не интересно».', href: '/feed', target: '.fc' },
    { icon: 'search', title: 'Поиск', text: 'Любой фильм, сериал или человек кино: русские и международные названия. Ниже - подборки и категории, фильтры по году, рейтингу и жанру.', href: '/search', target: '.big' },
    { icon: 'globe', title: 'Обзор - всё кино мира', text: 'Чарты, новинки и подборки IMDb. Нашли то, чего нет в библиотеке - отметьте «Хочу» или попросите хозяина добавить; фильм появится сам.', href: '/discover', target: '.rw-head' },
    { icon: 'bookmark', title: 'Коллекция', text: 'Всё, что вы отметили: «Буду смотреть», «Смотрю» и «Просмотрено» - в одном месте, с оценкой на постере.', href: '/my', target: '.my h1' },
    { icon: 'list', title: 'Подборки', text: 'Общие подборки с друзьями: «Что посмотреть в пятницу», «Ужасы на Хэллоуин». Добавлять и отмечать просмотренное могут все участники.', href: '/lists', target: '.rw-head' },
    { icon: 'user', title: 'Профиль', text: 'Статистика просмотров, обложка, любимые режиссёры. Здесь же чаты, подборки, комната по ссылке, управление и выход.', href: ME, target: '.quick' },
    { icon: 'party', title: 'Смотрим вместе', text: 'Комната на двоих и больше: видео идёт синхронно, есть чат, реакции и голос. Позовите друга сообщением со ссылкой.' },
    { icon: 'captions', title: 'Плеер', text: 'Качество само подстраивается под интернет. Пробел - пауза, стрелки - перемотка, S - пропустить заставку, F - во весь экран. Субтитры и дорожки - в меню плеера.' },
    { icon: 'tv', title: 'И на телевизоре', text: 'Откройте адрес Nova в браузере телевизора - сервер покажет интерфейс под пульт, с той же панелью разделов. Войти можно тем же логином и паролем.' },
  ];

  let open = $state(false);
  let step = $state(0);
  let dir = $state(1);
  let rect = $state<{ x: number; y: number; w: number; h: number } | null>(null);
  let cardH = $state(260);
  let vw = $state(1280);
  let vh = $state(720);
  let token = 0;
  const last = $derived(step === STEPS.length - 1);
  const s = $derived(STEPS[step]);
  const d = $derived(motionOn() ? 300 : 0);
  const sleep = (ms: number) => new Promise<void>((done) => setTimeout(done, ms));
  const pathOf = (href: string) => (href === ME ? `/people/${session.me?.user.profile?.id ?? ''}` : href);

  $effect(() => {
    const start = () => {
      step = 0;
      dir = 1;
      open = true;
    };
    window.addEventListener('nova-tour', start);
    return () => window.removeEventListener('nova-tour', start);
  });

  // Пришли на шаг: открываем нужную страницу, ждём, пока на ней появится нужное место, подводим к нему
  // и следим за ним (страница может ещё догружаться или прокручиваться).
  $effect(() => {
    if (!open) return;
    const mine = ++token;
    const now = s;
    rect = null;
    void (async () => {
      if (now.href) {
        const want = pathOf(now.href);
        if (location.pathname !== want) await goto(want);
      }
      if (!now.target || mine !== token) return;
      let el: HTMLElement | null = null;
      for (let i = 0; i < 50 && mine === token; i++) {
        // Часть разделов есть в двух видах (шапка на компьютере, полоса и нижняя панель на телефоне) - берём видимый.
        el = [...document.querySelectorAll<HTMLElement>(now.target)].find((e) => e.getBoundingClientRect().width > 0) ?? null;
        if (el) break;
        await sleep(100);
      }
      if (!el || mine !== token) return;
      el.scrollIntoView({ block: 'center', behavior: motionOn() ? 'smooth' : 'auto' });
      const follow = () => {
        if (mine !== token || !open || !el) return;
        const r = el.getBoundingClientRect();
        rect = { x: r.left, y: r.top, w: r.width, h: r.height };
        requestAnimationFrame(follow);
      };
      follow();
    })();
    return () => {
      token++;
    };
  });

  function go(to: number) {
    dir = to > step ? 1 : -1;
    step = Math.max(0, Math.min(STEPS.length - 1, to));
  }
  function close(done = false) {
    open = false;
    rect = null;
    if (done) void goto('/');
  }
  function key(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'ArrowRight') go(step + 1);
    else if (e.key === 'ArrowLeft') go(step - 1);
    else if (e.key === 'Escape') close();
    else return;
    e.preventDefault();
  }

  // Подпись рядом с подсвеченным местом: под ним, а если не помещается - над ним или в свободном углу.
  const cardW = $derived(Math.min(400, vw - 24));
  const place = $derived.by(() => {
    if (!rect) return null;
    const gap = 18;
    const left = Math.max(12, Math.min(rect.x + rect.w / 2 - cardW / 2, vw - cardW - 12));
    const bar = vw > 1000 ? 12 : 84;
    if (rect.y + rect.h + gap + cardH < vh - bar) return { left, top: rect.y + rect.h + gap };
    if (rect.y - gap - cardH > 12) return { left, top: rect.y - gap - cardH };
    return { left: Math.max(12, vw - cardW - 24), top: Math.max(12, vh - cardH - bar - 12) };
  });
</script>

<svelte:window onkeydown={key} bind:innerWidth={vw} bind:innerHeight={vh} />

{#if open}
  <div class="veil" role="dialog" aria-modal="true" aria-label="Экскурсия по Nova" transition:fade={{ duration: d }}>
    {#if rect}
      <div class="spot" style:left="{rect.x - 8}px" style:top="{rect.y - 8}px" style:width="{rect.w + 16}px" style:height="{rect.h + 16}px" aria-hidden="true"></div>
    {:else}
      <div class="dim" aria-hidden="true"></div>
    {/if}
    {#key step}
      <div
        class="card"
        class:floating={!!place}
        style:left={place ? `${place.left}px` : undefined}
        style:top={place ? `${place.top}px` : undefined}
        style:width="{cardW}px"
        bind:clientHeight={cardH}
        in:fly={{ x: 34 * dir, duration: d, delay: d / 4 }}
        out:fade={{ duration: d / 3 }}
      >
        <div class="top">
          <span>Экскурсия · {step + 1} из {STEPS.length}</span>
          <button class="skip" onclick={() => close()}>Закрыть</button>
        </div>
        <div class="bar"><i style:width="{((step + 1) / STEPS.length) * 100}%"></i></div>
        <div class="head">
          <span class="badge"><Icon name={s.icon} size={22} /></span>
          <h2>{s.title}</h2>
        </div>
        <p>{s.text}</p>
        <div class="actions">
          {#if step > 0}<button class="btn" onclick={() => go(step - 1)}>Назад</button>{/if}
          <span class="grow"></span>
          <button class="btn primary" onclick={() => (last ? close(true) : go(step + 1))}>{last ? 'Готово' : 'Дальше'}</button>
        </div>
      </div>
    {/key}
  </div>
{/if}

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 2300;
    display: grid;
    place-items: center;
    padding: 18px;
  }
  /* Без подсвеченного места - просто затемнение, подпись посередине. */
  .dim {
    position: absolute;
    inset: 0;
    background: #000000d9;
  }
  /* С местом: вырез в затемнении (тень вокруг рамки) и пульсирующий белый контур. Рамка плавно переезжает
     с места на место. */
  .spot {
    position: fixed;
    border-radius: 18px;
    box-shadow: 0 0 0 100vmax #000000c8;
    outline: 2px solid #ffffffd9;
    outline-offset: 0;
    pointer-events: none;
    transition: left 0.45s cubic-bezier(0.2, 0.8, 0.2, 1), top 0.45s cubic-bezier(0.2, 0.8, 0.2, 1), width 0.45s cubic-bezier(0.2, 0.8, 0.2, 1), height 0.45s cubic-bezier(0.2, 0.8, 0.2, 1);
    animation: pulse 2.2s ease-in-out infinite;
  }
  .card {
    position: relative;
    z-index: 1;
    padding: clamp(18px, 3vw, 26px);
    border: 1px solid #ffffff24;
    border-radius: 22px;
    background: #17171a;
    color: #fff;
    box-shadow: 0 30px 90px -20px #000;
  }
  .card.floating {
    position: fixed;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: #a1a1aa;
    font-size: 13px;
  }
  .skip {
    border: 0;
    background: none;
    color: #a1a1aa;
    font: inherit;
    cursor: pointer;
  }
  .skip:hover {
    color: #fff;
  }
  .bar {
    height: 3px;
    margin: 12px 0 14px;
    border-radius: 2px;
    background: #ffffff1f;
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: #fff;
    transition: width 0.4s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .badge {
    flex: none;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: #fff;
    color: #111;
    animation: badge-in 0.5s cubic-bezier(0.2, 0.9, 0.3, 1.4) both;
  }
  h2 {
    margin: 0;
    font-size: clamp(18px, 2.4vw, 22px);
    line-height: 1.2;
  }
  p {
    margin: 12px 0 0;
    color: #c9c9cf;
    font-size: 15px;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 18px;
  }
  .grow {
    flex: 1;
  }
  :global(.no-motion) .spot,
  :global(.no-motion) .badge {
    animation: none;
    transition: none;
  }
  @keyframes pulse {
    50% {
      outline-color: #ffffff55;
    }
  }
  @keyframes badge-in {
    from {
      opacity: 0;
      transform: scale(0.6) rotate(-12deg);
    }
  }
</style>
