<script lang="ts">
  import { motionOn } from '$lib/motion';
  import { logoBox, logoPaths } from './introLogo';
  import { sound, unlock } from './novaSound';

  // Заставка при входе - вариант A «Тишина» (выбор владельца, 21 ноября; как у Apple TV+): на чёрном мягкий свет
  // проходит сквозь знак Nova, знак остаётся белым, буквы NOVA спокойно собираются, звучит мягкий аккорд и звон блика
  // (если браузер уже разрешил звук - вход только что нажали). Знак берётся из `/logo.svg` (в сборку не входит), не
  // загрузился - заставки нет. Только прозрачность и сдвиг. Раз за сессию вкладки; анимации выключены - сразу.
  // Клики не держит (pointer-events: none).
  const KEY = 'nova-intro';
  const LETTERS = ['N', 'O', 'V', 'A'];
  const TOTAL = 5200;
  let show = $state(false);
  let d = $state('');
  let box = $state({ w: 100, h: 50.91 });

  $effect(() => {
    let seen = false;
    try {
      seen = sessionStorage.getItem(KEY) === '1';
      sessionStorage.setItem(KEY, '1');
    } catch {
      seen = false;
    }
    // Автотесты (Playwright) - без заставки: она на пару секунд закрывает страницу от проверок вёрстки.
    if (seen || !motionOn() || navigator.webdriver) return;
    let live = true;
    const timers: ReturnType<typeof setTimeout>[] = [];
    fetch('/logo.svg')
      .then((r) => (r.ok ? r.text() : ''))
      .then((svg) => {
        const paths = logoPaths(svg);
        if (!live || !paths.length) return;
        d = paths.join(' ');
        box = logoBox(svg) ?? box;
        show = true;
        unlock();
        timers.push(setTimeout(() => sound('swell'), 60), setTimeout(() => sound('shimmer'), 1500), setTimeout(() => (show = false), TOTAL));
      })
      .catch(() => {});
    return () => {
      live = false;
      timers.forEach(clearTimeout);
    };
  });
</script>

{#if show}
  <div class="intro" aria-hidden="true">
    <svg class="mark" viewBox="0 0 {box.w} {box.h}">
      <defs>
        <clipPath id="intro-clip"><path {d} clip-rule="evenodd" /></clipPath>
        <linearGradient id="intro-light" x1="0" x2="1" y1="0" y2="0">
          <stop offset="0" stop-color="#fff" stop-opacity="0" />
          <stop offset="0.3" stop-color="#fff" stop-opacity="0" />
          <stop offset="0.46" stop-color="#fff" />
          <stop offset="0.5" stop-color="#dcdce2" />
          <stop offset="0.54" stop-color="#fff" />
          <stop offset="0.7" stop-color="#fff" stop-opacity="0" />
          <stop offset="1" stop-color="#fff" stop-opacity="0" />
        </linearGradient>
        <linearGradient id="intro-solid" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0" stop-color="#fff" />
          <stop offset="1" stop-color="#d2d2d8" />
        </linearGradient>
      </defs>
      <g clip-path="url(#intro-clip)">
        <!-- Свет проходит сквозь знак слева направо, потом знак проявляется целиком. -->
        <rect class="sweep" x="0" y="0" width={box.w * 3} height={box.h} fill="url(#intro-light)" />
        <rect class="solid" x="0" y="0" width={box.w} height={box.h} fill="url(#intro-solid)" />
      </g>
    </svg>
    <p class="word">{#each LETTERS as l, i (l)}<span style:--i={i} style:--o={i - 1.5}>{l}</span>{/each}</p>
  </div>
{/if}

<style>
  .intro {
    position: fixed;
    inset: 0;
    z-index: 3000;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: clamp(26px, 4vh, 44px);
    background: #000;
    pointer-events: none;
    animation: out 0.9s ease-in 4.3s both;
  }
  .mark {
    width: clamp(200px, 26vw, 340px);
    height: auto;
    animation: settle 4.2s cubic-bezier(0.25, 0.1, 0.25, 1) both;
  }
  .sweep {
    transform: translateX(-66.6%);
    animation: sweep 2.6s cubic-bezier(0.45, 0, 0.25, 1) 0.3s both;
  }
  .solid {
    opacity: 0;
    animation: fade 1.4s ease-in-out 1.7s both;
  }
  .word {
    display: flex;
    gap: 0.55em;
    margin: 0;
    color: #fff;
    font-size: clamp(26px, 3vw, 40px);
    font-weight: 700;
    letter-spacing: 0.08em;
  }
  .word span {
    opacity: 0;
    transform: translateX(calc(var(--o) * 10px));
    animation: letter 1.5s cubic-bezier(0.25, 0.1, 0.25, 1) both;
    animation-delay: calc(2.5s + var(--i) * 110ms);
  }
  @keyframes settle {
    from {
      transform: scale(1.06);
    }
  }
  @keyframes sweep {
    to {
      transform: translateX(0);
    }
  }
  @keyframes fade {
    to {
      opacity: 1;
    }
  }
  @keyframes letter {
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes out {
    to {
      opacity: 0;
    }
  }
</style>
