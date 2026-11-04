// Анимированные картинки для README: `node scripts/readme-art.mjs` (из web/). Всё - обычный SVG с CSS и SMIL
// без скриптов и внешних файлов, поэтому на GitHub играет прямо в <img>. Знак берётся из web/static/logo.svg.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const ROOT = resolve(import.meta.dirname, '../..');
const OUT = join(ROOT, 'docs/assets');
mkdirSync(OUT, { recursive: true });

const logo = readFileSync(join(ROOT, 'web/static/logo.svg'), 'utf8');
const D = logo.match(/ d="([^"]+)"/)[1];
const FONT = "Inter, 'Segoe UI', system-ui, -apple-system, Roboto, 'Helvetica Neue', Arial, sans-serif";

// Детерминированный «случайный» ряд: картинки получаются одинаковыми при каждой сборке.
let seed = 7;
const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);

const svg = (w, h, body, extra = '') =>
  `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" role="img"${extra}>\n${body}\n</svg>\n`;

// ---------------------------------------------------------------------------------------------- герой
function hero() {
  const W = 1280;
  const H = 500;
  const dust = Array.from({ length: 34 }, (_, i) => {
    const x = (rnd() * W).toFixed(0);
    const y = (rnd() * (H - 90) + 40).toFixed(0);
    const r = (rnd() * 1.4 + 0.6).toFixed(1);
    const d = (rnd() * 4 + 3).toFixed(1);
    const b = (rnd() * 4).toFixed(1);
    return `<circle class="dust" cx="${x}" cy="${y}" r="${r}" style="animation-duration:${d}s;animation-delay:-${b}s"/>`;
  }).join('\n    ');
  const holes = (y) =>
    `<g transform="translate(0 ${y})"><g class="strip">${Array.from({ length: 36 }, (_, i) => `<rect x="${i * 40 - 40}" y="0" width="18" height="11" rx="3"/>`).join('')}</g></g>`;

  const pills = ['Фильмы с вашего диска', 'Смотреть вместе', 'Ссылка для друзей', 'Без Docker и Java'];
  const widths = pills.map((t) => Math.round(t.length * 9.4 + 44));
  const total = widths.reduce((a, b) => a + b, 0) + 14 * (pills.length - 1);
  let px = (W - total) / 2;
  const pillSvg = pills
    .map((t, i) => {
      const w = widths[i];
      const g = `<g transform="translate(${px.toFixed(0)} 392)"><g class="pill" style="animation-delay:${(i * 0.18).toFixed(2)}s">
      <rect width="${w}" height="40" rx="20"/><text x="${w / 2}" y="26" text-anchor="middle">${t}</text></g></g>`;
      px += w + 14;
      return g;
    })
    .join('\n    ');

  const body = `<defs>
  <path id="mark" pathLength="1" fill-rule="evenodd" d="${D}"/>
  <radialGradient id="glowA" cx="50%" cy="50%" r="50%"><stop offset="0" stop-color="#fff" stop-opacity=".16"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient>
  <radialGradient id="glowB" cx="50%" cy="50%" r="50%"><stop offset="0" stop-color="#9db4ff" stop-opacity=".13"/><stop offset="1" stop-color="#9db4ff" stop-opacity="0"/></radialGradient>
  <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0c0c0f"/><stop offset="1" stop-color="#050506"/></linearGradient>
  <linearGradient id="sweep" x1="0" y1="0" x2="1" y2="0">
    <stop offset="0" stop-color="#fff" stop-opacity="0"/><stop offset=".5" stop-color="#fff" stop-opacity=".95"/><stop offset="1" stop-color="#fff" stop-opacity="0"/>
  </linearGradient>
  <linearGradient id="word" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#b9b9c2"/></linearGradient>
  <mask id="onMark" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="51"><use href="#mark" xlink:href="#mark" fill="#fff"/></mask>
  <clipPath id="typed"><rect class="typed" x="0" y="0" width="640" height="60"/></clipPath>
  <style>
    :root { --cycle: 11s; }
    text { font-family: ${FONT}; }
    .glowA { animation: driftA 13s ease-in-out infinite alternate; }
    .glowB { animation: driftB 16s ease-in-out infinite alternate; }
    @keyframes driftA { from { transform: translate(-70px, 10px) } to { transform: translate(120px, -30px) } }
    @keyframes driftB { from { transform: translate(90px, 20px) } to { transform: translate(-130px, -10px) } }
    .dust { fill: #fff; opacity: 0; animation: tw 5s ease-in-out infinite; }
    @keyframes tw { 0%,100% { opacity: 0 } 50% { opacity: .55 } }
    .strip rect { fill: #fff; opacity: .1; }
    .strip { animation: roll 2.4s linear infinite; }
    @keyframes roll { to { transform: translateX(40px) } }
    .stage { animation: stage var(--cycle) ease-in-out infinite; }
    @keyframes stage { 0% { opacity: 0 } 3% { opacity: 1 } 93% { opacity: 1 } 100% { opacity: 0 } }
    .outline { fill: none; stroke: #fff; stroke-width: .55; stroke-linejoin: round; stroke-dasharray: 1; stroke-dashoffset: 1;
      animation: draw var(--cycle) cubic-bezier(.5,.05,.2,1) infinite; }
    @keyframes draw {
      0% { stroke-dashoffset: 1; opacity: 1 } 24% { stroke-dashoffset: 0; opacity: 1 }
      30% { opacity: 1 } 38% { opacity: 0 } 100% { stroke-dashoffset: 0; opacity: 0 }
    }
    .fill { fill: #fff; opacity: 0; animation: fillin var(--cycle) ease-in-out infinite; }
    @keyframes fillin { 0%,20% { opacity: 0 } 33% { opacity: 1 } 100% { opacity: 1 } }
    .shine { animation: shine var(--cycle) cubic-bezier(.4,0,.2,1) infinite; opacity: 0; }
    @keyframes shine { 0%,34% { transform: translateX(-60px); opacity: 0 } 36% { opacity: 1 } 48% { transform: translateX(120px); opacity: 1 } 49%,100% { transform: translateX(120px); opacity: 0 } }
    .logo { animation: float 6s ease-in-out infinite alternate; }
    @keyframes float { from { transform: translateY(0) } to { transform: translateY(-6px) } }
    .word { animation: rise var(--cycle) cubic-bezier(.2,.8,.2,1) infinite; opacity: 0; }
    @keyframes rise { 0%,31% { opacity: 0; transform: translateY(16px) } 41% { opacity: 1; transform: translateY(0) } 100% { opacity: 1; transform: translateY(0) } }
    .typed { animation: type var(--cycle) steps(34, end) infinite; }
    @keyframes type { 0%,42% { width: 0 } 62% { width: 640px } 100% { width: 640px } }
    .caret { fill: #fff; animation: caret var(--cycle) steps(34, end) infinite, blink 1s steps(1) infinite; }
    @keyframes caret { 0%,42% { transform: translateX(0) } 62%,100% { transform: translateX(640px) } }
    @keyframes blink { 50% { opacity: 0 } }
    .tag { font-size: 25px; font-weight: 500; fill: #d4d4da; }
    .pill { opacity: 0; animation: pill var(--cycle) cubic-bezier(.2,.8,.2,1) infinite; }
    .pill rect { fill: #ffffff10; stroke: #ffffff26; }
    .pill text { font-size: 16px; font-weight: 600; fill: #e8e8ec; }
    @keyframes pill { 0%,64% { opacity: 0; transform: translateY(14px) } 74% { opacity: 1; transform: translateY(0) } 100% { opacity: 1; transform: translateY(0) } }
    .pill { transform-box: fill-box; }
  </style>
</defs>
<rect width="${W}" height="${H}" fill="url(#bg)"/>
<g class="glowA"><circle cx="360" cy="190" r="380" fill="url(#glowA)"/></g>
<g class="glowB"><circle cx="930" cy="300" r="340" fill="url(#glowB)"/></g>
<g>
    ${dust}
</g>
<g style="--y:0">${holes(14)}</g>
<g style="--y:0">${holes(H - 25)}</g>
<g class="stage">
  <g transform="translate(${(W - 250) / 2} 58) scale(2.5)"><g class="logo">
    <use class="outline" href="#mark" xlink:href="#mark"/>
    <use class="fill" href="#mark" xlink:href="#mark"/>
    <g mask="url(#onMark)"><g class="shine"><rect x="-30" y="-4" width="34" height="60" fill="url(#sweep)" transform="skewX(-18)"/></g></g>
  </g></g>
  <text class="word" x="${W / 2 + 13}" y="258" text-anchor="middle" font-size="60" font-weight="800" letter-spacing="26" fill="url(#word)">NOVA</text>
  <g transform="translate(${(W - 640) / 2} 292)">
    <g clip-path="url(#typed)"><text class="tag" x="0" y="34" textLength="640" lengthAdjust="spacing">Свой кинотеатр из папки с фильмами</text></g>
    <rect class="caret" x="0" y="8" width="3" height="34" rx="1.5"/>
  </g>
  ${pillSvg}
</g>`;
  writeFileSync(join(OUT, 'hero.svg'), svg(W, H, body));
}

// ----------------------------------------------------------------------------------- как это работает
function flow() {
  const W = 1280;
  const H = 360;
  const dots = (path, n, dur) =>
    Array.from({ length: n }, (_, i) => `<circle r="5.5" fill="#fff"><animateMotion dur="${dur}s" begin="${((i * dur) / n).toFixed(2)}s" repeatCount="indefinite" path="${path}" keyPoints="0;1" keyTimes="0;1" calcMode="linear"/><animate attributeName="opacity" values="0;1;1;0" keyTimes="0;.12;.88;1" dur="${dur}s" begin="${((i * dur) / n).toFixed(2)}s" repeatCount="indefinite"/></circle>`).join('\n  ');

  const toNova = 'M 250 170 L 480 170';
  const toDev = ['M 800 170 C 880 170 880 78 960 78', 'M 800 170 L 960 170', 'M 800 170 C 880 170 880 262 960 262'];

  const body = `<defs>
  <path id="mark" pathLength="1" fill-rule="evenodd" d="${D}"/>
  <style>
    text { font-family: ${FONT}; }
    .t { font-size: 19px; font-weight: 700; fill: #f4f4f6; }
    .s { font-size: 15px; fill: #9a9aa4; }
    .line { stroke: #ffffff30; stroke-width: 2; fill: none; stroke-dasharray: 6 8; animation: march 1.2s linear infinite; }
    @keyframes march { to { stroke-dashoffset: -14 } }
    .card { fill: #131316; stroke: #ffffff22; }
    .pulse { transform-box: fill-box; transform-origin: center; animation: pulse 3.2s ease-in-out infinite; }
    @keyframes pulse { 0%,100% { transform: scale(1); opacity: .5 } 50% { transform: scale(1.09); opacity: 0 } }
    .bob { animation: bob 4s ease-in-out infinite alternate; }
    @keyframes bob { from { transform: translateY(0) } to { transform: translateY(-5px) } }
    .bar { fill: #ffffff2e; }
    .play { fill: #fff; transform-box: fill-box; transform-origin: center; animation: beat 2.4s ease-in-out infinite; }
    @keyframes beat { 0%,100% { transform: scale(1) } 50% { transform: scale(1.18) } }
    .reel { transform-box: fill-box; transform-origin: center; animation: spin 6s linear infinite; }
    @keyframes spin { to { transform: rotate(360deg) } }
  </style>
</defs>
<rect width="${W}" height="${H}" rx="28" fill="#0a0a0c"/>
<rect x=".5" y=".5" width="${W - 1}" height="${H - 1}" rx="27.5" fill="none" stroke="#ffffff1c"/>

<!-- линии -->
<path class="line" d="${toNova}"/>
${toDev.map((p) => `<path class="line" d="${p}"/>`).join('\n')}
${dots(toNova, 3, 2.7)}
${toDev.map((p, i) => dots(p, 2, 2.4 + i * 0.3)).join('\n')}

<!-- папка -->
<g class="bob">
  <rect class="card" x="60" y="104" width="190" height="132" rx="22"/>
  <path d="M92 150 h28 l10 12 h58 a8 8 0 0 1 8 8 v34 a8 8 0 0 1 -8 8 h-96 a8 8 0 0 1 -8 -8 v-46 a8 8 0 0 1 8 -8z" fill="#ffffff1a" stroke="#fff" stroke-opacity=".7" stroke-width="2"/>
  <g class="reel"><circle cx="156" cy="187" r="13" fill="none" stroke="#fff" stroke-width="2"/><circle cx="156" cy="187" r="2.5" fill="#fff"/><circle cx="156" cy="179" r="2.6" fill="#fff"/><circle cx="156" cy="195" r="2.6" fill="#fff"/><circle cx="148" cy="187" r="2.6" fill="#fff"/><circle cx="164" cy="187" r="2.6" fill="#fff"/></g>
</g>
<text class="t" x="155" y="284" text-anchor="middle">Папка с фильмами</text>
<text class="s" x="155" y="308" text-anchor="middle">на диске хоста</text>

<!-- Nova -->
<g>
  <rect class="card pulse" x="480" y="92" width="320" height="156" rx="30" style="fill:none;stroke:#fff"/>
  <rect class="card" x="480" y="92" width="320" height="156" rx="30"/>
  <g transform="translate(560 120) scale(1.6)"><use href="#mark" xlink:href="#mark" fill="#fff"/></g>
  <text class="t" x="640" y="226" text-anchor="middle">Nova на вашем компьютере</text>
</g>
<text class="s" x="640" y="284" text-anchor="middle">описания, постеры, перепаковка видео</text>
<text class="s" x="640" y="308" text-anchor="middle">и постоянная ссылка для друзей</text>

<!-- устройства -->
<g class="bob" style="animation-delay:-.6s">
  <rect class="card" x="960" y="44" width="210" height="68" rx="16"/>
  <rect class="bar" x="978" y="60" width="60" height="8" rx="4"/><rect class="bar" x="978" y="76" width="100" height="8" rx="4"/>
  <circle cx="1136" cy="78" r="16" fill="#ffffff14"/><path class="play" d="M1131 70 l13 8 -13 8z"/>
</g>
<g class="bob" style="animation-delay:-1.4s">
  <rect class="card" x="960" y="136" width="210" height="68" rx="16"/>
  <rect x="980" y="146" width="26" height="48" rx="6" fill="#ffffff14" stroke="#ffffff55"/>
  <rect class="bar" x="1020" y="154" width="60" height="8" rx="4"/><rect class="bar" x="1020" y="170" width="96" height="8" rx="4"/>
  <path class="play" d="M1144 160 l13 8 -13 8z"/>
</g>
<g class="bob" style="animation-delay:-2.2s">
  <rect class="card" x="960" y="228" width="210" height="68" rx="16"/>
  <rect x="978" y="240" width="52" height="34" rx="5" fill="#ffffff14" stroke="#ffffff55"/><rect x="996" y="278" width="16" height="3" rx="1.5" fill="#ffffff55"/>
  <rect class="bar" x="1046" y="250" width="60" height="8" rx="4"/><rect class="bar" x="1046" y="266" width="86" height="8" rx="4"/>
  <path class="play" d="M1142 252 l13 8 -13 8z"/>
</g>
<text class="t" x="1065" y="326" text-anchor="middle">Браузер, телефон, ТВ</text>`;
  writeFileSync(join(OUT, 'flow.svg'), svg(W, H, body));
}

// -------------------------------------------------------------------------------------------- возможности
function features() {
  const W = 1280;
  const cardW = 402;
  const cardH = 156;
  const gap = 28;
  const cards = [
    { t: 'Любые файлы', a: 'MP4, MKV, AVI, HEVC -', b: 'плеер сам выберет способ', icon: 'play' },
    { t: 'Место везде', a: 'Начали на ноутбуке -', b: 'продолжите с телефона', icon: 'sync' },
    { t: 'Вместе с друзьями', a: 'Комнаты, чат, голос,', b: 'отзывы и общие подборки', icon: 'people' },
    { t: 'Доступ из интернета', a: 'Постоянная ссылка', b: 'без настройки роутера', icon: 'globe' },
    { t: 'Описания сами', a: 'Постеры, сюжет, актёры', b: 'и кадры - с IMDb', icon: 'search' },
    { t: 'Безопасно', a: 'Argon2id и вход в два шага,', b: 'фильмы остаются у вас', icon: 'shield' },
  ];
  const icons = {
    play: `<circle class="ring" cx="0" cy="0" r="26" fill="none" stroke="#fff" stroke-width="2"/><circle class="ring r2" cx="0" cy="0" r="26" fill="none" stroke="#fff" stroke-width="2"/><path class="beat" d="M-8 -13 L14 0 -8 13z" fill="#fff"/>`,
    sync: `<g class="spin"><path d="M-22 -4 A22 22 0 0 1 14 -17" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round"/><path d="M14 -26 v10 h-10" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/><path d="M22 4 A22 22 0 0 1 -14 17" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round"/><path d="M-14 26 v-10 h10" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/></g>`,
    people: `<g class="b1"><circle cx="-18" cy="-6" r="8" fill="#fff"/><path d="M-32 18 a14 12 0 0 1 28 0z" fill="#fff"/></g><g class="b2"><circle cx="18" cy="-6" r="8" fill="#fff" opacity=".7"/><path d="M4 18 a14 12 0 0 1 28 0z" fill="#fff" opacity=".7"/></g><g class="b3"><circle cx="0" cy="-14" r="9" fill="#fff"/><path d="M-15 12 a15 13 0 0 1 30 0z" fill="#fff"/></g>`,
    globe: `<circle r="24" fill="none" stroke="#fff" stroke-width="2.5"/><ellipse rx="10" ry="24" fill="none" stroke="#fff" stroke-width="2" opacity=".7"/><path d="M-24 0 H24 M-20 -12 H20 M-20 12 H20" stroke="#fff" stroke-width="2" opacity=".6" fill="none"/><g class="orbit"><circle cx="30" cy="0" r="4.5" fill="#fff"/></g>`,
    search: `<g class="scan"><circle cx="-4" cy="-4" r="17" fill="none" stroke="#fff" stroke-width="3"/><path d="M9 9 L24 24" stroke="#fff" stroke-width="4" stroke-linecap="round"/></g><rect class="sc" x="-26" y="-26" width="52" height="3" rx="1.5" fill="#fff" opacity=".7"/>`,
    shield: `<path d="M0 -26 L22 -18 V2 C22 15 11 23 0 28 C-11 23 -22 15 -22 2 V-18z" fill="#ffffff14" stroke="#fff" stroke-width="2.5" stroke-linejoin="round"/><path class="tick" pathLength="1" d="M-9 2 L-2 10 L11 -6" fill="none" stroke="#fff" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"/>`,
  };
  const H = cardH * 2 + gap + 8;
  const items = cards
    .map((c, i) => {
      const x = (i % 3) * (cardW + gap) + 8;
      const y = Math.floor(i / 3) * (cardH + gap) + 4;
      return `<g transform="translate(${x} ${y})"><g class="card" style="animation-delay:${(i * 0.55).toFixed(2)}s">
  <rect class="bg" width="${cardW}" height="${cardH}" rx="26"/>
  <rect class="edge" width="${cardW}" height="${cardH}" rx="26"/>
  <g transform="translate(66 ${cardH / 2})">${icons[c.icon]}</g>
  <text class="h" x="120" y="62">${c.t}</text>
  <text class="p" x="120" y="92">${c.a}</text>
  <text class="p" x="120" y="114">${c.b}</text>
</g></g>`;
    })
    .join('\n');
  const body = `<defs>
  <style>
    text { font-family: ${FONT}; }
    .bg { fill: #121215; }
    .edge { fill: none; stroke: #fff; stroke-opacity: .12; animation: edge 5s ease-in-out infinite; }
    .card .edge { animation-delay: inherit; }
    @keyframes edge { 0%,100% { stroke-opacity: .1 } 50% { stroke-opacity: .34 } }
    .h { font-size: 22px; font-weight: 700; fill: #f6f6f8; }
    .p { font-size: 15.5px; fill: #a1a1ab; }
    .ring { transform-box: fill-box; transform-origin: center; animation: ring 2.8s ease-out infinite; }
    .ring.r2 { animation-delay: 1.4s; }
    @keyframes ring { 0% { transform: scale(.7); opacity: .9 } 100% { transform: scale(1.35); opacity: 0 } }
    .beat { transform-box: fill-box; transform-origin: center; animation: beat 2.8s ease-in-out infinite; }
    @keyframes beat { 0%,100% { transform: scale(1) } 50% { transform: scale(1.14) } }
    .spin { animation: spin 5s linear infinite; }
    @keyframes spin { to { transform: rotate(360deg) } }
    .b1 { animation: hop 2.6s ease-in-out infinite; } .b2 { animation: hop 2.6s ease-in-out .35s infinite; } .b3 { animation: hop 2.6s ease-in-out .7s infinite; }
    @keyframes hop { 0%,100% { transform: translateY(0) } 50% { transform: translateY(-5px) } }
    .orbit { animation: spin 6s linear infinite; }
    .scan { animation: scan 4s ease-in-out infinite alternate; }
    @keyframes scan { from { transform: translate(-5px, 2px) } to { transform: translate(6px, -3px) } }
    .sc { animation: sc 2.2s ease-in-out infinite; }
    @keyframes sc { 0%,100% { transform: translateY(0); opacity: 0 } 50% { transform: translateY(52px); opacity: .8 } }
    .tick { stroke-dasharray: 1; animation: tick 3.4s ease-in-out infinite; }
    @keyframes tick { 0% { stroke-dashoffset: 1 } 35%,85% { stroke-dashoffset: 0 } 100% { stroke-dashoffset: 1 } }
  </style>
</defs>
${items}`;
  writeFileSync(join(OUT, 'features.svg'), svg(W, H, body));
}

// -------------------------------------------------------------------------------------------- разделитель
function line() {
  const W = 1280;
  const body = `<defs>
  <linearGradient id="g" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#fff" stop-opacity="0"/><stop offset=".5" stop-color="#fff" stop-opacity=".9"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>
  <linearGradient id="base" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#fff" stop-opacity="0"/><stop offset=".5" stop-color="#fff" stop-opacity=".16"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>
</defs>
<rect x="0" y="9" width="${W}" height="2" fill="url(#base)"/>
<rect y="8" width="320" height="4" rx="2" fill="url(#g)"><animate attributeName="x" from="-320" to="${W}" dur="3.6s" repeatCount="indefinite"/></rect>`;
  writeFileSync(join(OUT, 'line.svg'), svg(W, 20, body));
}

hero();
flow();
features();
line();
console.log('Готово:', OUT);
