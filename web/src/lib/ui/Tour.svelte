<script lang="ts">
  import { goto } from '$app/navigation';
  import { fade, fly } from 'svelte/transition';
  import { motionOn } from '$lib/motion';
  import { session } from '$lib/session.svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  // Экскурсия A «Попробуйте сами» (выбор владельца, 23.11): семь коротких шагов прямо по приложению. Подсвечено одно
  // место, рядом подсказка. На шагах «нажмите» подсвеченное можно нажать по-настоящему (оценка, «Смотреть») - и экскурсия
  // сама переходит дальше, уже на открывшейся странице; на остальных шагах нажатия мимо подписи не проходят. «Дальше»
  // работает всегда: если нужной страницы ещё нет, экскурсия сама откроет первую подходящую ссылку (`via`).
  // Запуск: после знакомства и событием `nova-tour` («Экскурсия по Nova» в профиле). Стрелки листают, Esc закрывает.
  type Step = {
    icon: IconName;
    title: string;
    text: string;
    /** Открыть страницу перед шагом. */
    href?: string;
    /** Подсветить первое видимое место. */
    target?: string;
    /** Если места нет на текущей странице - перейти по первой видимой ссылке. */
    via?: string;
    /** click - нажатие на подсвеченное и есть «Дальше»; free - с подсвеченным можно работать, дальше - кнопкой. */
    try?: 'click' | 'free';
    hint?: string;
    /** Подсказку показываем, только если подсвечено именно это (на телефоне звёзды спрятаны в меню «…»). */
    hintFor?: string;
    /** На шаге плеера ставим видео на паузу: на паузе управление не прячется. */
    pause?: boolean;
  };
  const ME = '/people/me';
  const FILM = '.showcase a[href^="/title/"], .showcase a[href^="/ext/"], a[href^="/title/"], a[href^="/ext/"]';
  const STEPS: Step[] = [
    { icon: 'film', title: 'Витрина', text: 'Новое на Nova, отмеченное вами и популярное на IMDb. Кадры сменяются сами, листать - стрелками или смахиванием.', href: '/', target: '.showcase' },
    { icon: 'star', title: 'Оцените фильм', text: 'Пять звёзд с половинками. По оценкам Nova подбирает, что посмотреть дальше, а друзья видят их в вашем профиле.', target: '.stars, #s-rating', via: FILM, try: 'click', hint: 'Нажмите на звезду', hintFor: '.stars' },
    { icon: 'play', title: 'Смотреть', text: 'Фильм начнётся с того места, где вы остановились, - на любом устройстве: телефоне, компьютере или телевизоре.', target: 'a.watch, a.btn[href^="/watch/"]', via: FILM, try: 'click', hint: 'Нажмите «Смотреть»' },
    { icon: 'captions', title: 'Плеер', text: 'Пробел - пауза, стрелки - перемотка на 10 секунд. Справа звук, озвучка, субтитры, качество и полный экран; подписи появляются при наведении.', target: '.bottom .row', via: 'a.watch, a.btn[href^="/watch/"]', pause: true },
    { icon: 'party', title: 'Смотреть вместе', text: 'Позовите друзей: видео пойдёт у всех одновременно, с чатом, реакциями и голосом. Позвать можно ссылкой.', target: 'button[aria-label="Смотреть вместе"]', via: 'a.watch, a.btn[href^="/watch/"]', pause: true },
    { icon: 'sparkle', title: 'Подбор', text: 'Опишите словами, что хочется: «короткое про космос», «комедия на вечер». Ниже - фильм дня и подборки по вашему вкусу.', href: '/feed', target: 'form.ask', try: 'free', hint: 'Попробуйте написать' },
    { icon: 'user', title: 'Ваш профиль', text: 'Любимые фильмы, оценки, статистика и подборки с друзьями. Отсюда же чаты, комната по ссылке и повтор этой экскурсии.', href: ME, target: 'header.ph' },
  ];

  let open = $state(false);
  let hintOn = $state(true);
  let step = $state(0);
  let dir = $state(1);
  let rect = $state<{ x: number; y: number; w: number; h: number } | null>(null);
  let cardH = $state(260);
  let vw = $state(1280);
  let vh = $state(720);
  let token = 0;
  const last = $derived(step === STEPS.length - 1);
  const s = $derived(STEPS[step]);
  const d = $derived(motionOn() && !window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 300 : 0);
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
        if (location.pathname !== want) {
          try { await goto(want); } catch { return; }
        }
      }
      if (!now.target || mine !== token) return;
      // Часть мест есть в двух видах (шапка на компьютере, нижняя панель на телефоне) - берём видимый.
      const find = (sel: string) => [...document.querySelectorAll<HTMLElement>(sel)].find((e) => e.getBoundingClientRect().width > 0) ?? null;
      let el: HTMLElement | null = null;
      for (let i = 0; i < 60 && mine === token; i++) {
        el = find(now.target);
        if (el) break;
        // Нужной страницы ещё нет (нажали «Дальше», а не само место) - открываем первую подходящую ссылку.
        if (i === 6 && now.via) {
          const link = find(now.via) as HTMLAnchorElement | null;
          if (link?.href) {
            try { await goto(new URL(link.href).pathname); } catch { return; }
          }
        }
        await sleep(100);
      }
      if (!el || mine !== token) return;
      // Заголовок раздела - подсвечиваем весь раздел.
      if (el.tagName === 'H2') el = el.closest('section') ?? el;
      hintOn = !now.hintFor || el.matches(now.hintFor);
      if (now.pause) document.querySelector('video')?.pause();
      // На узком экране - к верху: подпись встаёт под местом и не закрывает его.
      const block: ScrollLogicalPosition = innerWidth <= 700 && el.getBoundingClientRect().height < innerHeight / 2 ? 'start' : 'center';
      el.style.scrollMarginTop = '28px';
      el.style.scrollMarginBottom = '28px';
      el.scrollIntoView({ block, behavior: d ? 'smooth' : 'auto' });
      let frame = 0;
      let previous: { x: number; y: number; w: number; h: number } | null = null;
      // Страница после перехода может сама прокрутиться (возврат места): место ушло за экран - подводим к нему снова.
      let fixes = 0;
      let lastFix = performance.now();
      const follow = () => {
        if (mine !== token || !open || !el) return;
        const r = el.getBoundingClientRect();
        if ((r.bottom < 0 || r.top > innerHeight) && fixes < 4 && performance.now() - lastFix > 400) {
          fixes++;
          lastFix = performance.now();
          el.scrollIntoView({ block });
        }
        if (!previous || previous.x !== r.left || previous.y !== r.top || previous.w !== r.width || previous.h !== r.height) {
          previous = { x: r.left, y: r.top, w: r.width, h: r.height };
          rect = previous;
        }
        frame = requestAnimationFrame(follow);
      };
      follow();
      if (mine !== token) cancelAnimationFrame(frame);
    })();
    return () => {
      token++;
    };
  });

  // Шаг «нажмите»: нажатие внутри подсветки проходит к самой странице и переводит экскурсию дальше.
  $effect(() => {
    if (!open || s.try !== 'click') return;
    const at = step;
    const hit = (e: PointerEvent) => {
      const r = rect;
      if (!r || step !== at) return;
      if (e.clientX >= r.x && e.clientX <= r.x + r.w && e.clientY >= r.y && e.clientY <= r.y + r.h) setTimeout(() => step === at && go(at + 1), 350);
    };
    document.addEventListener('pointerup', hit, true);
    return () => document.removeEventListener('pointerup', hit, true);
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
    // Шаг «попробуйте написать»: стрелки в поле ввода - для текста, а не для листания экскурсии.
    if (e.key !== 'Escape' && e.target instanceof HTMLElement && e.target.closest('input, textarea')) return;
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
      <div class="spot" class:open={!!s.try} style:left="{rect.x - 8}px" style:top="{rect.y - 8}px" style:width="{rect.w + 16}px" style:height="{rect.h + 16}px" aria-hidden="true"></div>
      <!-- Нажатия мимо подсветки не доходят до страницы; внутри подсветки - только на шагах «попробуйте». -->
      <div class="block" style:left="0" style:top="0" style:width="100%" style:height="{Math.max(0, rect.y - 8)}px" aria-hidden="true"></div>
      <div class="block" style:left="0" style:top="{rect.y + rect.h + 8}px" style:width="100%" style:bottom="0" aria-hidden="true"></div>
      <div class="block" style:left="0" style:top="{rect.y - 8}px" style:width="{Math.max(0, rect.x - 8)}px" style:height="{rect.h + 16}px" aria-hidden="true"></div>
      <div class="block" style:left="{rect.x + rect.w + 8}px" style:top="{rect.y - 8}px" style:right="0" style:height="{rect.h + 16}px" aria-hidden="true"></div>
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
        {#if s.hint && rect && hintOn}<span class="hint"><i aria-hidden="true"></i>{s.hint}</span>{/if}
        <div class="actions">
          {#if step > 0}<button class="btn" onclick={() => go(step - 1)}>Назад</button>{/if}
          <span class="grow"></span>
          <button class="btn primary" onclick={() => (last ? close(true) : go(step + 1))}>{last ? 'Готово' : s.try === 'click' ? 'Пропустить шаг' : 'Дальше'}</button>
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
    pointer-events: none;
  }
  .veil > :global(*) { pointer-events: auto; }
  .block { position: fixed; }
  .spot.open { pointer-events: none; }
  .hint { display: inline-flex; align-items: center; gap: 10px; margin-top: 14px; padding: 8px 14px; border-radius: 999px; background: #ffffff14; font-size: 13px; font-weight: 600; }
  .hint i { width: 8px; height: 8px; border-radius: 50%; background: #fff; animation: blink 1.4s ease-in-out infinite; }
  @keyframes blink { 50% { opacity: 0.25; } }
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
    max-height: calc(100dvh - 36px);
    overflow-y: auto;
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
  @media (prefers-reduced-motion: reduce) {
    .spot, .badge { animation: none; transition: none; }
    .bar i { transition: none; }
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
