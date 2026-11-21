<script lang="ts">
  import { onMount } from 'svelte';
  import { sound } from './novaSound';

  // Обучающий синематик после знакомства (выбор владельца, 21 ноября): шесть настоящих экранов Nova (снимки самого
  // приложения из `static/intro`) на одной «сцене» 3 × 2; камера перелетает между ними, главный блок каждой сцены
  // подсвечен (остальное мягко темнеет), курсор нажимает настоящие кнопки, титры слева внизу, звук - `novaSound`.
  // Сцена 1440 × 810 масштабируется под окно. Только transform и opacity (+ свойства подсветки). «Пропустить» - всегда.
  let { onclose }: { onclose: () => void } = $props();
  let host: HTMLDivElement;
  let fit: HTMLDivElement;
  let stage: HTMLDivElement;
  let done = $state(false);

  const OUT = 'cubic-bezier(.16,1,.3,1)', IO = 'cubic-bezier(.65,0,.35,1)';
  const SCR = [[0, 0], [1500, 0], [3000, 0], [0, 880], [1500, 880], [3000, 880]];
  const SHOTS = [['home'], ['player', 'player-pause'], ['room'], ['feed', 'feed-typed'], ['title', 'title-rated'], ['profile']];
  // Блоки на снимках (x, y, ширина, высота) в точках экрана 1440 × 810.
  const BOX: Record<string, number[]> = {
    nav: [180, 0, 648, 76], hero: [56, 76, 1328, 496], cont: [56, 584, 1328, 226], card: [56, 621, 343, 189],
    pbar: [43, 700, 1354, 80], pplay: [700, 735, 40, 40], ptogether: [1360, 18, 36, 36], pause: [40, 400, 700, 300],
    side: [1080, 0, 360, 810], react: [378, 596, 324, 48],
    ask: [56, 163, 1328, 66], res: [56, 255, 520, 430],
    plate: [1160, 350, 200, 320], stars: [1206, 462, 150, 30],
    pfhead: [40, 300, 1360, 130], pffav: [56, 540, 940, 270], pfside: [1044, 538, 340, 272],
  };
  // Экран как на презентации: чуть меньше, вверху справа; слева внизу - место для титров.
  const K = 0.74, OX = 1440 - 1440 * K - 40, OY = 44;
  const toStage = (b: number[]) => [OX + b[0] * K, OY + b[1] * K, b[2] * K, b[3] * K];

  let run = 0;
  const alive = (t: number) => t === run;
  const wait = (ms: number, t: number) => new Promise<void>((ok, no) => setTimeout(() => (alive(t) ? ok() : no('stop')), ms));
  const A = (n: Element, kf: Keyframe[], o: KeyframeAnimationOptions) => n.animate(kf, { fill: 'both', easing: OUT, ...o });
  const el = (html: string) => { const d = document.createElement('div'); d.innerHTML = html.trim(); return d.firstElementChild as HTMLElement; };
  const words = (text: string) => text.split(' ').map((w) => `<span class="w"><span>${w}</span></span>`).join(' ');
  const reveal = (h: Element, t0 = 0) => [...h.querySelectorAll('.w > span')].forEach((s, i) => A(s, [{ transform: 'translateY(110%)' }, { transform: 'none' }], { duration: 900, delay: t0 + i * 70 }));

  // Курсор ведём как руку: по плавной дуге (offset-path), время зависит от расстояния.
  let from: [number, number] = [720, 760];
  function cursorTo(c: HTMLElement, x: number, y: number) {
    const [fx, fy] = from;
    const d = Math.hypot(x - fx, y - fy), bend = Math.min(90, d * 0.18);
    const mx = (fx + x) / 2 + ((y - fy) / (d || 1)) * bend, my = (fy + y) / 2 - ((x - fx) / (d || 1)) * bend;
    c.getAnimations().filter((a) => a.id === 'move').forEach((a) => a.cancel());
    c.style.offsetPath = `path('M ${fx} ${fy} Q ${mx} ${my} ${x} ${y}')`;
    const a = c.animate([{ offsetDistance: '0%' }, { offsetDistance: '100%' }], { duration: Math.max(560, Math.min(1500, 420 + d * 1.05)), easing: 'cubic-bezier(.45,.05,.2,1)', fill: 'both' });
    a.id = 'move';
    return a.finished.then(() => { from = [x, y]; });
  }

  function scale() {
    if (!host || !fit) return;
    const s = Math.min(host.clientWidth / 1440, host.clientHeight / 810);
    fit.style.transform = `translate(-50%, -50%) scale(${s})`;
  }

  async function play(t: number) {
    const cam = el(`<div class="cam">${SHOTS.map((v, i) => `<div class="scr" style="left:${SCR[i][0]}px;top:${SCR[i][1]}px">${v.map((n, k) => `<img class="${k ? 'alt' : 'base'}" src="/intro/${n}.jpg" alt="" ${k ? 'style="opacity:0"' : ''}>`).join('')}</div>`).join('')}</div>`);
    const dim = el('<div class="dim"></div>');
    const capbg = el('<div class="capbg"></div>');
    const cap = el('<div class="cap"><div class="eyebrow"></div><h2 class="h"></h2><p></p></div>');
    const c = el('<div class="cursor"><svg width="22" height="30" viewBox="0 0 22 30" aria-hidden="true"><path d="M3 3 L3 24 L8.5 18.8 L12.6 27.5 L16.4 25.8 L12.4 17.2 L19.5 17.2 Z" fill="#111" stroke="#fff" stroke-width="2" stroke-linejoin="round"/></svg></div>');
    const typing = el('<div class="typing"></div>');
    stage.append(cam, typing, dim, capbg, cap, c);
    const scr = [...cam.querySelectorAll<HTMLElement>('.scr')];
    let camP = `translate(${OX}px,${OY}px) scale(${K})`;
    cam.style.transform = camP;
    const camTo = (x: number, y: number, s: number, ms = 2200) => A(cam, [{ transform: camP }, { transform: (camP = `translate(${x}px,${y}px) scale(${s})`) }], { duration: ms, easing: IO });
    async function screen(i: number, ms = 2400) {
      scr.forEach((n, k) => A(n, [{ opacity: getComputedStyle(n).opacity }, { opacity: k === i ? 1 : 0 }], { duration: ms * 0.6, easing: IO }));
      await camTo(OX - SCR[i][0] * K, OY - SCR[i][1] * K, K, ms).finished;
    }
    const swap = (i: number, ms = 700) => A(scr[i].querySelector('.alt')!, [{ opacity: 0 }, { opacity: 1 }], { duration: ms, easing: IO });
    let dimP = { sx: '720px', sy: '400px', sw: '2400px', sh: '2400px' };
    function spot(b?: number[], pad = 40) {
      let next = { sx: '720px', sy: '400px', sw: '2400px', sh: '2400px' };
      if (b) { const [x, y, w, h] = toStage(b); next = { sx: x + w / 2 + 'px', sy: y + h / 2 + 'px', sw: w * 0.95 + pad * 1.6 + 'px', sh: h * 0.95 + pad * 1.6 + 'px' }; }
      A(dim, [{ '--sx': dimP.sx, '--sy': dimP.sy, '--sw': dimP.sw, '--sh': dimP.sh }, { '--sx': next.sx, '--sy': next.sy, '--sw': next.sw, '--sh': next.sh }], { duration: 1100, easing: IO });
      dimP = next;
    }
    const centerOf = (b: number[]) => { const [x, y, w, h] = toStage(b); return [x + w / 2, y + h / 2]; };
    async function tap(b: number[]) {
      const [x, y] = centerOf(b);
      await cursorTo(c, x, y); await wait(80, t); sound('click');
      A(c.firstElementChild!, [{ transform: 'scale(1)' }, { transform: 'scale(.86)' }, { transform: 'scale(1)' }], { duration: 260, fill: 'none' });
      const ring = el('<i class="ring"></i>'); ring.style.left = x + 'px'; ring.style.top = y + 'px'; stage.append(ring);
      A(ring, [{ opacity: 0.9, transform: 'scale(.4)' }, { opacity: 0, transform: 'scale(1.3)' }], { duration: 520, easing: 'ease-out' }).finished.then(() => ring.remove());
      await wait(260, t);
    }
    const showCursor = (on: boolean) => A(c, [{ opacity: on ? 0 : 1 }, { opacity: on ? 1 : 0 }], { duration: 300 });
    const parts = [...cap.children] as HTMLElement[];
    async function say(eb: string, h: string, p: string) {
      parts.forEach((n) => n.getAnimations().forEach((x) => x.cancel()));
      parts[0].textContent = eb; parts[1].innerHTML = words(h); parts[2].textContent = p;
      sound('tick');
      A(parts[0], [{ opacity: 0 }, { opacity: 1 }], { duration: 600 });
      reveal(parts[1], 100);
      A(parts[2], [{ opacity: 0, transform: 'translateY(8px)' }, { opacity: 1, transform: 'none' }], { duration: 800, delay: 500 });
    }
    const hush = async () => { parts.forEach((n) => A(n, [{ opacity: 1 }, { opacity: 0 }], { duration: 400 })); await wait(420, t); };

    scr.forEach((n, k) => k && (n.style.opacity = '0'));
    A(cam, [{ opacity: 0 }, { opacity: 1 }], { duration: 1200 });
    A(capbg, [{ opacity: 0 }, { opacity: 1 }], { duration: 900, delay: 400 });
    sound('swell');
    await wait(1000, t);

    // 1. Главная.
    spot(BOX.nav, 70);
    await say('Главная · разделы', 'Всё самое нужное - сверху', 'Главная, «Что смотреть», фильмы, сериалы, аниме и ваша коллекция - в одной строке.');
    await wait(3000, t); await hush();
    spot(BOX.hero, 20);
    await say('Главная · витрина', 'Большая витрина на каждый вечер', 'Новое на Nova, отмеченное вами и популярное на IMDb - кадры сменяются сами.');
    await wait(3200, t); await hush();
    spot(BOX.cont, 20);
    await say('Главная · продолжить', 'Место помнится везде', 'Начали на телевизоре - продолжите на телефоне с той же секунды.');
    showCursor(true); await wait(1800, t);
    await tap(BOX.card); await hush(); showCursor(false);

    // 2. Плеер.
    spot(); await screen(1);
    spot(BOX.pbar, 30);
    await say('Плеер', 'Ничего лишнего - только фильм', 'Название, «Закончится в», тонкая линия и одна строка значков: звук, озвучка, качество, картинка в картинке.');
    showCursor(true); await wait(2200, t);
    await tap(BOX.pplay); swap(1); spot(BOX.pause, 40);
    await hush();
    await say('Плеер · пауза', 'На паузе - крупно, что вы смотрите', 'Перемотка работает и на паузе: кадр меняется сразу, без загрузки.');
    await wait(2400, t);
    await tap(BOX.ptogether); await hush(); showCursor(false);

    // 3. Совместный просмотр.
    spot(); await screen(2);
    spot(BOX.side, 30);
    await say('Смотреть вместе', 'Комната: смотрите с друзьями синхронно', 'Пауза и перемотка у всех сразу, чат, голос и реакции. Позвать можно ссылкой.');
    await wait(3000, t);
    spot(BOX.react, 60);
    [0, 1, 2, 3].forEach((k) => setTimeout(() => alive(t) && sound('select', { k: 3 + k }), 300 + k * 160));
    await wait(2400, t); await hush();

    // 4. Что смотреть.
    spot(); await screen(3);
    spot(BOX.ask, 50);
    await say('Что смотреть', 'Спросите Nova обычными словами', 'Жанр, длина, годы или название - и подборка по вашему вкусу. Ниже - фильм дня.');
    showCursor(true);
    await tap([BOX.ask[0] + 90, BOX.ask[1], 400, BOX.ask[3]]);
    const [ax, ay, , ah] = toStage(BOX.ask);
    typing.style.left = ax + 71 * K + 'px'; typing.style.top = ay + ah / 2 + 'px'; typing.style.fontSize = 17 * K + 'px';
    typing.style.opacity = '1';
    for (const ch of 'Матрица') { typing.textContent += ch; sound('tick'); await wait(90 + Math.random() * 60, t); }
    await wait(250, t);
    swap(3, 500); A(typing, [{ opacity: 1 }, { opacity: 0 }], { duration: 300, delay: 200 });
    spot(BOX.res, 40);
    await wait(3000, t); await hush(); showCursor(false);

    // 5. Оценки.
    spot(); await screen(4);
    spot(BOX.plate, 50);
    await say('Оценки и отзывы', 'Оцените - и Nova запомнит ваш вкус', 'Пять звёзд с половинками, «Смотрел», «Нравится», «Хочу», подборки и отзыв для друзей.');
    showCursor(true); await wait(1400, t);
    const [sx, sy, sw] = toStage(BOX.stars);
    for (let k = 0; k < 5; k++) { await cursorTo(c, sx + ((k + 0.5) * sw) / 5, sy + 10); sound('select', { k }); }
    await tap(BOX.stars); swap(4, 400); sound('fave', { k: 2 });
    await wait(2600, t); await hush(); showCursor(false);

    // 6. Профиль.
    spot(); await screen(5);
    spot(BOX.pfhead, 30);
    await say('Профиль', 'Ваш кинодневник собирается сам', 'Уровень, цифры года, любимые фильмы, оценки и дневник по датам. Делитесь профилем с друзьями.');
    await wait(2600, t);
    spot(BOX.pffav, 30); await wait(2200, t);
    spot(BOX.pfside, 30); await wait(2200, t); await hush();

    // Общий план всех шести экранов и финал.
    spot();
    A(capbg, [{ opacity: 1 }, { opacity: 0 }], { duration: 900 });
    scr.forEach((n) => A(n, [{ opacity: getComputedStyle(n).opacity }, { opacity: 1 }], { duration: 1200, delay: 400 }));
    await camTo(40, 52, 0.29, 2600).finished;
    await wait(1400, t);
    sound('resolve');
    done = true;
  }

  onMount(() => {
    // Свойства подсветки - анимируемые длины (иначе градиент не «едет»).
    for (const name of ['--sx', '--sy', '--sw', '--sh']) {
      try { (CSS as unknown as { registerProperty: (p: object) => void }).registerProperty({ name, syntax: '<length>', inherits: false, initialValue: '0px' }); } catch { /* уже есть */ }
    }
    scale();
    addEventListener('resize', scale);
    const key = (e: KeyboardEvent) => e.key === 'Escape' && close();
    addEventListener('keydown', key);
    const t = ++run;
    play(t).catch(() => {});
    return () => {
      run++;
      removeEventListener('resize', scale);
      removeEventListener('keydown', key);
    };
  });
  function close() {
    run++;
    onclose();
  }
</script>

<div class="cine" bind:this={host} role="dialog" aria-modal="true" aria-label="Обучение: как устроена Nova">
  <div class="fit" bind:this={fit}><div class="stage" bind:this={stage}></div></div>
  {#if done}
    <div class="final">
      <img src="/logo.svg" alt="" />
      <h2>Всё готово</h2>
      <button class="btn primary" type="button" onclick={close}>Открыть главную</button>
    </div>
  {:else}
    <button class="skip" type="button" onclick={close}>Пропустить</button>
  {/if}
</div>

<style>
  .cine { position: fixed; inset: 0; z-index: 2300; background: #000; color: #fff; overflow: hidden; }
  .fit { position: absolute; left: 50%; top: 50%; width: 1440px; height: 810px; transform-origin: 50% 50%; }
  .stage { position: absolute; inset: 0; overflow: hidden; }
  .stage :global(.cam) { position: absolute; left: 0; top: 0; width: 4440px; height: 1690px; transform-origin: 0 0; }
  .stage :global(.scr) { position: absolute; width: 1440px; height: 810px; overflow: hidden; border-radius: 18px; background: #0b0b0d; box-shadow: 0 40px 120px -30px #000, 0 0 0 1px #ffffff1f; }
  .stage :global(.scr img) { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .stage :global(.dim) { position: absolute; inset: 0; z-index: 22; pointer-events: none; background: radial-gradient(var(--sw, 2400px) var(--sh, 2400px) at var(--sx, 720px) var(--sy, 400px), #0000 0%, #0000 70%, #0000008c 100%); }
  .stage :global(.capbg) { position: absolute; inset: 0; z-index: 24; background: radial-gradient(60% 55% at 18% 100%, #000e 0%, #000a 45%, #0000 75%); opacity: 0; }
  .stage :global(.cap) { position: absolute; z-index: 25; left: 72px; bottom: 96px; width: 640px; }
  .stage :global(.eyebrow) { color: #8e8e96; font-size: 12px; font-weight: 700; letter-spacing: 0.22em; text-transform: uppercase; opacity: 0; }
  .stage :global(.h) { margin: 14px 0 0; font-size: 46px; font-weight: 800; letter-spacing: -0.035em; line-height: 1.04; }
  .stage :global(.h .w) { display: inline-block; overflow: hidden; vertical-align: top; padding-bottom: 0.08em; }
  .stage :global(.h .w > span) { display: inline-block; transform: translateY(110%); }
  .stage :global(.cap p) { margin: 12px 0 0; color: #d0d0d6; font-size: 17px; line-height: 1.5; opacity: 0; }
  .stage :global(.cursor) { position: absolute; left: 0; top: 0; z-index: 30; width: 22px; height: 30px; offset-anchor: 3px 3px; offset-rotate: 0deg; offset-path: path('M 720 760'); opacity: 0; pointer-events: none; }
  .stage :global(.cursor svg) { display: block; transform-origin: 3px 3px; filter: drop-shadow(0 4px 10px #0008); }
  .stage :global(.ring) { position: absolute; z-index: 29; width: 44px; height: 44px; margin: -22px 0 0 -22px; border-radius: 50%; border: 2px solid #ffffffcc; pointer-events: none; opacity: 0; }
  .stage :global(.typing) { position: absolute; z-index: 21; color: #fff; font-weight: 500; transform: translateY(-50%); opacity: 0; pointer-events: none; }
  .skip { position: absolute; z-index: 5; top: max(18px, env(safe-area-inset-top)); right: max(18px, env(safe-area-inset-right)); padding: 10px 18px; border: 1px solid #ffffff33; border-radius: 999px; background: #000000a0; color: #fff; font: inherit; font-size: 14px; font-weight: 600; cursor: pointer; }
  .skip:hover { background: #ffffff1f; }
  .skip:focus-visible, .final button:focus-visible { outline: 2px solid #fff; outline-offset: 3px; }
  .final { position: absolute; inset: 0; z-index: 6; display: grid; place-content: center; justify-items: center; gap: 18px; text-align: center; background: radial-gradient(60% 60% at 50% 50%, #000000e6, #000); animation: in 1.2s ease both; }
  .final img { width: 120px; }
  .final h2 { margin: 0; font-size: clamp(36px, 5vw, 56px); font-weight: 800; letter-spacing: -0.035em; }
  @keyframes in { from { opacity: 0; } }
</style>
