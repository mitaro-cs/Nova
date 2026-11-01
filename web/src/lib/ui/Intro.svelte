<script lang="ts">
  import { motionOn } from '$lib/motion';

  // Заставка при входе в сервис (вариант «Стена постеров», выбор владельца): кадры слетаются на экран и
  // складываются в стену, из неё проявляется NOVA, стена гаснет и сам чёрный экран плавно уходит.
  // Идёт около 4,5 секунд. Постеры - со стены входа (`/api/site/wall`, без входа); их ещё нет или
  // сайт не ответил - цветные плитки. Только прозрачность и сдвиг (их считает видеокарта): без
  // размытия и без анимации самой страницы, иначе на слабом компьютере заставка «лагала». Раз за
  // сессию вкладки; анимации выключены - сразу. Клики не держит (pointer-events: none).
  const KEY = 'nova-intro';
  const LETTERS = ['N', 'O', 'V', 'A'];
  const TOTAL = 4500;
  const TILES = Array.from({ length: 18 }, (_, i) => i);
  // Откуда слетается каждая плитка (px) и оттенок заглушки.
  const XS = [-90, 70, -60, 100, -110, 60, 80, -70, 120, -50, 90, -100, 50, -120, 70, -60, 110, -80];
  const YS = [-60, 80, 70, -90, 50, -70, -50, 90, -40, 70, -80, 40, 90, -60, -90, 50, -40, 80];
  const HUES = [8, 205, 40, 140, 285, 340];
  let show = $state(false);
  let posters = $state<string[]>([]);

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

  $effect(() => {
    if (!show) return;
    let live = true;
    fetch('/api/site/wall', { headers: { 'X-Fsfy': '1' } })
      .then((r) => (r.ok ? r.json() : []))
      .then((list: string[]) => live && (posters = list), () => {});
    return () => (live = false);
  });
</script>

{#if show}
  <div class="intro" aria-hidden="true">
    <div class="wall">
      {#each TILES as i (i)}
        <span class="tile" style:--x="{XS[i]}px" style:--y="{YS[i]}px" style:--d={(i % 6) + Math.floor(i / 6)} style:--h={HUES[i % 6]}>
          {#if posters.length}
            <img
              src={posters[(i * 5) % posters.length]}
              alt=""
              decoding="async"
              onload={(e) => e.currentTarget.classList.add('on')}
            />
          {/if}
        </span>
      {/each}
    </div>
    <div class="word">
      {#each LETTERS as l, i (i)}<span style:--i={i}>{l}</span>{/each}
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
  .wall {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    grid-template-rows: repeat(3, minmax(0, 1fr));
    gap: 2px;
    animation: wall-out 0.9s ease-in 2.4s both;
  }
  @media (max-width: 640px) {
    .wall {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      grid-template-rows: repeat(6, minmax(0, 1fr));
    }
  }
  .tile {
    position: relative;
    display: block;
    overflow: hidden;
    background: hsl(var(--h) 42% 30%);
    will-change: transform, opacity;
    animation: tile 0.9s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--d) * 80ms);
  }
  .tile img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.4s;
  }
  .tile img:global(.on) {
    opacity: 1;
  }
  .word {
    position: relative;
    display: flex;
    gap: 0.06em;
    color: #fff;
    font-size: clamp(48px, 8vw, 104px);
    font-weight: 800;
    line-height: 1;
    text-shadow: 0 2px 28px rgba(0, 0, 0, 0.7);
  }
  .word span {
    display: inline-block;
    will-change: transform, opacity;
    animation: letter 0.8s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(1.6s + var(--i) * 100ms);
  }
  @keyframes tile {
    from {
      opacity: 0;
      transform: translate(var(--x), var(--y)) scale(1.5);
    }
    to {
      opacity: 0.55;
    }
  }
  @keyframes letter {
    from {
      opacity: 0;
      transform: translateY(16px);
    }
  }
  @keyframes wall-out {
    to {
      opacity: 0;
    }
  }
  @keyframes intro-out {
    to {
      opacity: 0;
    }
  }
</style>
