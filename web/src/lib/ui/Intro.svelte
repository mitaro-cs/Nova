<script lang="ts">
  import { motionOn } from '$lib/motion';
  import { logoBox, logoPaths } from './introLogo';

  // Заставка при входе - «Тушь» (выбор владельца 25.11, сцена 4 из шести): знак Nova проступает живым пятном, как капля
  // туши в воде, - края растекаются по мягкому шуму слева направо, пятно заполняет знак до конца; затем подпись NOVA и
  // сцена растворяется в приложение. Без звука (звуки владелец не одобрил). Знак - тот же `/logo.svg` (в сборку не
  // входит); не загрузился - заставки нет. Раз за сессию вкладки; анимации выключены или автотест - сразу приложение.
  // Клики не держит (pointer-events: none).
  const KEY = 'nova-intro';
  const REVEAL = 2.1;
  const TOTAL = 3.6;
  let show = $state(false);
  let leaving = $state(false);
  let canvas = $state<HTMLCanvasElement>();
  let word = $state<HTMLParagraphElement>();
  let d = '';
  let box = { w: 100, h: 50.91 };

  $effect(() => {
    let seen = false;
    try {
      seen = sessionStorage.getItem(KEY) === '1';
      sessionStorage.setItem(KEY, '1');
    } catch {
      seen = false;
    }
    if (seen || !motionOn() || navigator.webdriver) return;
    let live = true;
    fetch('/logo.svg')
      .then((r) => (r.ok ? r.text() : ''))
      .then((svg) => {
        const paths = logoPaths(svg);
        if (!live || !paths.length) return;
        d = paths.join(' ');
        box = logoBox(svg) ?? box;
        show = true;
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  });

  // Покадрово, по одной шкале: маска на сетке над знаком (шум + расстояние от точки слева), фронт пятна растёт.
  $effect(() => {
    if (!show || !canvas) return;
    const cv = canvas;
    const ctx = cv.getContext('2d');
    if (!ctx) return;
    const COLS = 72, ROWS = 37;
    const field = noise(COLS, ROWS);
    const mask = document.createElement('canvas');
    mask.width = COLS;
    mask.height = ROWS;
    const mx = mask.getContext('2d')!;
    const img = mx.createImageData(COLS, ROWS);
    const shape = document.createElement('canvas');
    const path = new Path2D(d);
    let geo = { x: 0, y: 0, w: 0, h: 0 };
    const fit = () => {
      const k = Math.min(2, devicePixelRatio || 1);
      cv.width = Math.round(innerWidth * k);
      cv.height = Math.round(innerHeight * k);
      const w = Math.min(cv.width * 0.34, cv.height * 1.05, 620 * k);
      const h = (w * box.h) / box.w;
      geo = { x: (cv.width - w) / 2, y: cv.height * 0.46 - h / 2, w, h };
      shape.width = Math.ceil(w);
      shape.height = Math.ceil(h);
      const sx = shape.getContext('2d')!;
      sx.setTransform(w / box.w, 0, 0, w / box.w, 0, 0);
      sx.fillStyle = '#fff';
      sx.fill(path, 'evenodd');
      if (word) word.style.top = `${(geo.y + h) / k + Math.max(28, h / k / 2.6)}px`;
    };
    fit();
    addEventListener('resize', fit);
    const t0 = performance.now();
    let raf = 0;
    const tick = (now: number) => {
      const t = (now - t0) / 1000;
      const q = Math.min(1, Math.max(0, (t - 0.1) / (REVEAL - 0.1)));
      const front = -0.05 + 1.2 * (1 - (1 - q) * (1 - q));
      for (let j = 0; j < ROWS; j++)
        for (let i = 0; i < COLS; i++) {
          const u = i / (COLS - 1), v = j / (ROWS - 1);
          const dx = 0.36 * u - 0.06, dy = ((v - 0.5) * 0.326 - 0.03) * 0.56;
          const val = Math.sqrt(dx * dx + dy * dy) * 1.6 + field[j * COLS + i] * 0.55;
          img.data[(j * COLS + i) * 4 + 3] = Math.max(0, Math.min(1, (front - val) * 6)) * 255;
        }
      mx.putImageData(img, 0, 0);
      ctx.clearRect(0, 0, cv.width, cv.height);
      ctx.globalCompositeOperation = 'source-over';
      ctx.drawImage(shape, geo.x, geo.y, geo.w, geo.h);
      ctx.globalCompositeOperation = 'destination-in';
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(mask, geo.x, geo.y, geo.w, geo.h);
      ctx.globalCompositeOperation = 'source-over';
      if (t > TOTAL - 0.6 && !leaving) leaving = true;
      if (t < TOTAL) raf = requestAnimationFrame(tick);
      else show = false;
    };
    raf = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(raf);
      removeEventListener('resize', fit);
    };
  });

  /** Гладкий шум на сетке (два слоя разной частоты) - живой край пятна. */
  function noise(cols: number, rows: number): Float32Array {
    const g: number[] = [];
    for (let i = 0; i < 9 * 6; i++) g.push(Math.random());
    const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
    const at = (x: number, y: number) => {
      const gx = x * 8, gy = y * 5, ix = Math.floor(gx), iy = Math.floor(gy), fx = gx - ix, fy = gy - iy;
      const v = (a: number, b: number) => g[Math.min(5, b) * 9 + Math.min(8, a)];
      const sx = fx * fx * (3 - 2 * fx), sy = fy * fy * (3 - 2 * fy);
      return lerp(lerp(v(ix, iy), v(ix + 1, iy), sx), lerp(v(ix, iy + 1), v(ix + 1, iy + 1), sx), sy);
    };
    const f = new Float32Array(cols * rows);
    for (let j = 0; j < rows; j++)
      for (let i = 0; i < cols; i++) {
        // Те же координаты, что в одобренном макете: шум по сцене, знак - её середина.
        const x = 0.32 + 0.36 * (i / (cols - 1)), y = 0.45 + (j / (rows - 1) - 0.5) * 0.326;
        f[j * cols + i] = at(x, y) * 0.6 + at((x * 2.3) % 1, (y * 2.3) % 1) * 0.4;
      }
    return f;
  }
</script>

{#if show}
  <div class="intro" class:leaving aria-hidden="true">
    <canvas bind:this={canvas}></canvas>
    <p class="word" bind:this={word}>NOVA</p>
  </div>
{/if}

<style>
  .intro {
    position: fixed;
    inset: 0;
    z-index: 3000;
    background: #000;
    pointer-events: none;
    transition: opacity 0.6s ease-in;
  }
  .intro.leaving {
    opacity: 0;
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .word {
    position: absolute;
    left: 0;
    right: 0;
    top: 62%;
    margin: 0;
    padding-left: 0.42em;
    color: #fff;
    font-size: clamp(14px, 1.6vw, 22px);
    font-weight: 700;
    letter-spacing: 0.42em;
    text-align: center;
    opacity: 0;
    animation: word 0.8s ease-out 2s both;
  }
  @keyframes word {
    from {
      opacity: 0;
      letter-spacing: 0.9em;
    }
    to {
      opacity: 1;
      letter-spacing: 0.42em;
    }
  }
</style>
