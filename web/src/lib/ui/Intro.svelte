<script lang="ts">
  import { motionOn } from '$lib/motion';
  import { logoBox, logoPaths, longestSubpath } from './introLogo';

  // Заставка при входе в сервис (выбор владельца, 2 ноября): на чёрном светящейся линией рисуется контур
  // нашего знака, заливается, по нему проходит свет, ниже проявляется NOVA; всё это чуть-чуть «дышит»
  // (медленный наезд), в конце экран плавно уходит. Идёт 5,25 секунды. Знак берётся из `/logo.svg`
  // (в сборку не входит), не загрузился - заставки нет. Только прозрачность и сдвиг (их считает
  // видеокарта): без размытия и фильтров. Раз за сессию вкладки; анимации выключены - сразу. Клики не
  // держит (pointer-events: none).
  const KEY = 'nova-intro';
  const LETTERS = ['N', 'O', 'V', 'A'];
  const TOTAL = 5250;
  let show = $state(false);
  let d = $state('');
  let box = $state({ w: 100, h: 50.91 });
  let len = $state(400);

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
    let timer: ReturnType<typeof setTimeout> | undefined;
    fetch('/logo.svg')
      .then((r) => (r.ok ? r.text() : ''))
      .then((svg) => {
        const paths = logoPaths(svg);
        if (!live || !paths.length) return;
        d = paths.join(' ');
        len = longestSubpath(d);
        box = logoBox(svg) ?? box;
        show = true;
        timer = setTimeout(() => (show = false), TOTAL);
      })
      .catch(() => {});
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });
</script>

{#if show}
  <div class="intro" aria-hidden="true">
    <div class="drift">
      <svg class="mark" viewBox="0 0 {box.w} {box.h}" style:--len="{len}px">
        <defs>
          <clipPath id="intro-clip"><path {d} clip-rule="evenodd" /></clipPath>
          <linearGradient id="intro-sweep" x1="0" x2="1" y1="0" y2="0">
            <stop offset="0" stop-color="#fff" stop-opacity="0" />
            <stop offset="0.5" stop-color="#cfd6e4" stop-opacity="0.9" />
            <stop offset="1" stop-color="#fff" stop-opacity="0" />
          </linearGradient>
        </defs>
        <path class="line" {d} />
        <path class="fill" {d} fill-rule="evenodd" />
        <g clip-path="url(#intro-clip)"><rect class="sweep" x="-30" y="-2" width="30" height={box.h + 4} fill="url(#intro-sweep)" /></g>
      </svg>
      <div class="word">
        {#each LETTERS as l, i (i)}<span style:--i={i}>{l}</span>{/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .intro {
    position: fixed;
    inset: 0;
    z-index: 2000;
    display: grid;
    place-items: center;
    background: #000;
    pointer-events: none;
    will-change: opacity;
    animation: intro-out 0.5s cubic-bezier(0.45, 0, 0.55, 1) 4.75s both;
  }
  .drift {
    position: relative;
    display: grid;
    justify-items: center;
    gap: clamp(18px, 4.5vw, 56px);
    will-change: transform;
    animation: drift 5.25s cubic-bezier(0.37, 0, 0.63, 1) both;
  }
  .mark {
    width: clamp(240px, 42vw, 640px);
    height: auto;
    overflow: visible;
  }
  .line {
    fill: none;
    stroke-linejoin: round;
    stroke-linecap: round;
    stroke-dasharray: var(--len) calc(var(--len) * 3);
    stroke-dashoffset: var(--len);
    stroke: #fff;
    stroke-width: 0.26;
    animation-name: draw, line-out;
    animation-duration: 1.8s, 1.1s;
    animation-delay: 0.35s, 1.7s;
    animation-fill-mode: both, both;
  }
  .fill {
    fill: #fff;
    will-change: opacity;
    animation: fill-in 1.1s cubic-bezier(0.22, 1, 0.36, 1) 1.7s both;
  }
  .sweep {
    will-change: transform;
    animation: sweep 1.1s cubic-bezier(0.65, 0, 0.35, 1) 2.4s both;
  }
  .word {
    display: flex;
    gap: 0.42em;
    color: #fff;
    font-size: clamp(22px, 3.9vw, 52px);
    font-weight: 700;
    line-height: 1;
  }
  .word span {
    display: inline-block;
    will-change: transform, opacity;
    animation: letter 0.9s cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(2.6s + var(--i) * 100ms);
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes line-out {
    to {
      opacity: 0.45;
    }
  }
  @keyframes fill-in {
    from {
      opacity: 0;
    }
  }
  @keyframes sweep {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(160px);
    }
  }
  @keyframes letter {
    from {
      opacity: 0;
      transform: translateY(14px);
    }
  }
  @keyframes drift {
    to {
      transform: scale(1.045);
    }
  }
  @keyframes intro-out {
    to {
      opacity: 0;
    }
  }
</style>
