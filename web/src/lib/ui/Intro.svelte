<script lang="ts">
  import { motionOn } from '$lib/motion';

  // Заставка при входе в сервис (вариант B, выбор владельца): буквы NOVA по очереди проявляются, под ними
  // бежит тонкая линия и появляется подпись, потом чёрный экран плавно гаснет. Идёт около 4,5 секунд.
  // Только прозрачность и сдвиг (их считает видеокарта): без размытия и без анимации самой страницы,
  // иначе на слабом компьютере заставка «лагала». Раз за сессию вкладки; анимации выключены - сразу.
  // Клики не держит (pointer-events: none).
  const KEY = 'nova-intro';
  const LETTERS = ['N', 'O', 'V', 'A'];
  const TOTAL = 4500;
  let show = $state(false);
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
    show = true;
    const t = setTimeout(() => (show = false), TOTAL);
    return () => clearTimeout(t);
  });
</script>

{#if show}
  <div class="intro" aria-hidden="true">
    <div class="center">
      <div class="word">
        {#each LETTERS as l, i (i)}<span style:--i={i}>{l}</span>{/each}
      </div>
      <i class="line"></i>
      <p>Ваш домашний кинотеатр</p>
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
    animation: intro-out 0.9s ease-in-out 3.5s both;
  }
  .center {
    display: grid;
    justify-items: center;
    gap: 18px;
  }
  .word {
    display: flex;
    gap: 0.06em;
    color: #fff;
    font-size: clamp(48px, 8vw, 104px);
    font-weight: 800;
    line-height: 1;
  }
  span {
    display: inline-block;
    will-change: transform, opacity;
    animation: letter 1s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(0.2s + var(--i) * 260ms);
  }
  .line {
    display: block;
    width: min(260px, 40vw);
    height: 2px;
    background: #ffffffb8;
    transform-origin: left center;
    will-change: transform;
    animation: sweep 1.6s cubic-bezier(0.5, 0, 0.2, 1) 1.1s both;
  }
  p {
    margin: 0;
    color: #ffffff9e;
    font-size: clamp(12px, 1.4vw, 15px);
    letter-spacing: 0.32em;
    text-transform: uppercase;
    will-change: transform, opacity;
    animation: letter 0.9s cubic-bezier(0.2, 0.8, 0.2, 1) 2s both;
  }
  @keyframes letter {
    from {
      opacity: 0;
      transform: translateY(16px);
    }
  }
  @keyframes sweep {
    from {
      transform: scaleX(0);
    }
  }
  @keyframes intro-out {
    to {
      opacity: 0;
    }
  }
</style>
