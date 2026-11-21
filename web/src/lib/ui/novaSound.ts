// Звуки Nova: синтез в браузере (WebAudio), без файлов. Заставка, знакомство и обучающий синематик (21.11).
// Цепочка: шины (сухая + ревербератор с преддилеем) → эквалайзер → компрессор-лимитер. Браузер разрешает звук только
// после нажатия: `unlock()` вызывать из обработчика нажатия; без разрешения звуки молча не играют.
/* eslint-disable @typescript-eslint/no-explicit-any */
let ac: AudioContext | null = null;
let bus: GainNode;
let verbIn: GainNode;
export let soundOn = true;
export function setSound(on: boolean) {
  soundOn = on;
}
function audio(): AudioContext {
  if (ac) return ac;
  ac = new (
    window.AudioContext || (window as any).webkitAudioContext
  )() as AudioContext;
  const lim = ac.createDynamicsCompressor();
  lim.threshold.value = -10;
  lim.knee.value = 6;
  lim.ratio.value = 12;
  lim.attack.value = 0.003;
  lim.release.value = 0.25;
  const lo = ac.createBiquadFilter();
  lo.type = "lowshelf";
  lo.frequency.value = 120;
  lo.gain.value = 2;
  const hi = ac.createBiquadFilter();
  hi.type = "highshelf";
  hi.frequency.value = 9000;
  hi.gain.value = -3;
  const master = ac.createGain();
  master.gain.value = 0.7;
  lo.connect(hi).connect(lim).connect(master).connect(ac.destination);
  const len = Math.floor(ac.sampleRate * 3.8),
    ir = ac.createBuffer(2, len, ac.sampleRate);
  for (let ch = 0; ch < 2; ch++) {
    const d = ir.getChannelData(ch);
    for (let k = 0; k < len; k++)
      d[k] =
        (Math.random() * 2 - 1) *
        Math.pow(1 - k / len, 2.6) *
        (k < 400 ? k / 400 : 1);
  }
  const verb = ac.createConvolver();
  verb.buffer = ir;
  const pre = ac.createDelay();
  pre.delayTime.value = 0.035;
  const damp = ac.createBiquadFilter();
  damp.type = "lowpass";
  damp.frequency.value = 5200;
  verbIn = ac.createGain();
  verbIn.gain.value = 0.32;
  verbIn.connect(pre).connect(verb).connect(damp).connect(lo);
  bus = ac.createGain();
  bus.connect(lo);
  return ac;
}
function out(pan = 0, send = 0.3) {
  const ac = audio();
  const g = ac.createGain(),
    p = ac.createStereoPanner();
  p.pan.value = pan;
  g.connect(p);
  p.connect(bus);
  const s2 = ac.createGain();
  s2.gain.value = send;
  p.connect(s2).connect(verbIn);
  return g;
}
function env(
  g: GainNode,
  at: number,
  peak: number,
  attack: number,
  dur: number,
) {
  g.gain.setValueAtTime(0, at);
  g.gain.linearRampToValueAtTime(peak, at + attack);
  g.gain.exponentialRampToValueAtTime(0.0001, at + dur);
}
// Пэд: на ноту три пилы с расстройкой, фильтр медленно открывается.
function pad(
  freqs: number[],
  at: number,
  dur: number,
  { gain = 0.05, attack = 2.2, cut = [300, 2600] } = {},
) {
  const ac = audio();
  freqs.forEach((f, n) =>
    [-7, 0, 7].forEach((det, v) => {
      const o = ac.createOscillator(),
        lp = ac.createBiquadFilter(),
        g = ac.createGain(),
        o2 = out((v - 1) * 0.55 + (n % 2 ? 0.1 : -0.1), 0.55);
      o.type = "sawtooth";
      o.frequency.value = f;
      o.detune.value = det + (Math.random() * 4 - 2);
      lp.type = "lowpass";
      lp.Q.value = 0.6;
      lp.frequency.setValueAtTime(cut[0], at);
      lp.frequency.exponentialRampToValueAtTime(cut[1], at + attack * 1.6);
      env(g, at, gain / 3, attack, dur);
      o.connect(lp).connect(g).connect(o2);
      o.start(at);
      o.stop(at + dur + 0.1);
    }),
  );
  // Саб под пэдом.
  const sub = ac.createOscillator(),
    sg = ac.createGain();
  sub.frequency.value = freqs[0] / 2;
  env(sg, at, gain * 0.9, attack, dur);
  sub.connect(sg).connect(out(0, 0.1));
  sub.start(at);
  sub.stop(at + dur + 0.1);
}
// FM-колокольчик: чистый звон с затуханием обертонов.
function bell(
  f: number,
  at: number,
  { gain = 0.06, dur = 2.2, ratio = 3.5, index = 2.2, pan = 0 } = {},
) {
  const ac = audio();
  const car = ac.createOscillator(),
    mod = ac.createOscillator(),
    mg = ac.createGain(),
    g = ac.createGain();
  car.frequency.value = f;
  mod.frequency.value = f * ratio;
  mg.gain.setValueAtTime(f * index, at);
  mg.gain.exponentialRampToValueAtTime(f * 0.05, at + dur * 0.6);
  mod.connect(mg).connect(car.frequency);
  env(g, at, gain, 0.004, dur);
  car.connect(g).connect(out(pan, 0.45));
  car.start(at);
  mod.start(at);
  car.stop(at + dur + 0.1);
  mod.stop(at + dur + 0.1);
}
function noiseBuf(dur: number) {
  const ac = audio();
  const len = Math.ceil(ac.sampleRate * dur),
    b = ac.createBuffer(1, len, ac.sampleRate),
    d = b.getChannelData(0);
  for (let k = 0; k < len; k++) d[k] = Math.random() * 2 - 1;
  return b;
}
// Нота «войлочного» инструмента (как приглушённое пианино или маримба): основной тон, мягкий второй и третий
// обертоны, короткое касание шумом, фильтр закрывается вместе с затуханием. Без сдвига высоты - не «мультяшно».
function felt(
  f: number,
  at: number,
  { gain = 0.09, dur = 0.9, pan = 0, bright = 2600 } = {},
) {
  const ac = audio();
  const lp = ac.createBiquadFilter(),
    g = ac.createGain();
  lp.type = "lowpass";
  lp.Q.value = 0.4;
  lp.frequency.setValueAtTime(bright, at);
  lp.frequency.exponentialRampToValueAtTime(
    Math.max(300, f * 1.2),
    at + dur * 0.7,
  );
  env(g, at, gain, 0.006, dur);
  lp.connect(g).connect(out(pan, 0.22));
  [
    [1, 1, "sine"],
    [2, 0.28, "sine"],
    [3, 0.08, "triangle"],
  ].forEach(([m, v, type]) => {
    const o = ac.createOscillator(),
      og = ac.createGain();
    o.type = type as OscillatorType;
    o.frequency.value = f * (m as number);
    og.gain.value = v as number;
    o.connect(og).connect(lp);
    o.start(at);
    o.stop(at + dur + 0.1);
  });
  const src = ac.createBufferSource(),
    bp = ac.createBiquadFilter(),
    ng = ac.createGain();
  src.buffer = noiseBuf(0.03);
  bp.type = "bandpass";
  bp.frequency.value = f * 4;
  bp.Q.value = 1.2;
  env(ng, at, gain * 0.25, 0.001, 0.025);
  src.connect(bp).connect(ng).connect(out(pan, 0.05));
  src.start(at);
}
// Пентатоника ре мажор: каждый следующий выбор - нота выше, звучит как мелодия, а не как одинаковые щелчки.
const SCALE = [
  587.33, 659.25, 739.99, 880, 987.77, 1174.66, 1318.51, 1479.98, 1760,
];
export type SoundKind =
  | "swell"
  | "shimmer"
  | "boom"
  | "click"
  | "select"
  | "unselect"
  | "fave"
  | "tick"
  | "resolve";
export function sound(
  kind: SoundKind,
  opt: {
    k?: number;
  } = {},
) {
  if (!soundOn) return;
  let ac: AudioContext;
  try {
    ac = audio();
  } catch {
    return;
  }
  if (ac.state !== "running") return;
  const t = ac.currentTime + 0.01;
  switch (kind) {
    case "swell":
      pad([130.81, 196, 246.94, 329.63], t, 8, { gain: 0.06, attack: 2.6 });
      break; // Cmaj7, медленно раскрывается
    case "shimmer":
      [1567.98, 2093, 2349.3, 3135.96].forEach((f, k) =>
        bell(f, t + k * 0.11, {
          gain: 0.025,
          dur: 2.6,
          ratio: 2.01,
          index: 1.2,
          pan: -0.5 + k * 0.33,
        }),
      );
      break;
    case "boom": {
      const o = ac.createOscillator(),
        g = ac.createGain();
      o.frequency.setValueAtTime(90, t);
      o.frequency.exponentialRampToValueAtTime(34, t + 0.9);
      env(g, t, 0.55, 0.008, 2.2);
      o.connect(g).connect(out(0, 0.5));
      o.start(t);
      o.stop(t + 2.4);
      const src = ac.createBufferSource(),
        lp = ac.createBiquadFilter(),
        ng = ac.createGain();
      src.buffer = noiseBuf(0.4);
      lp.type = "lowpass";
      lp.frequency.value = 900;
      env(ng, t, 0.35, 0.002, 0.35);
      src.connect(lp).connect(ng).connect(out(0, 0.6));
      src.start(t);
      break;
    }
    case "click": {
      // мягкий «ток»
      const o = ac.createOscillator(),
        g = ac.createGain();
      o.type = "sine";
      o.frequency.setValueAtTime(1150, t);
      o.frequency.exponentialRampToValueAtTime(520, t + 0.03);
      env(g, t, 0.16, 0.002, 0.07);
      o.connect(g).connect(out(0, 0.12));
      o.start(t);
      o.stop(t + 0.1);
      break;
    }
    case "select": {
      // выбор: нота пентатоники (k - номер выбора) и тихая квинта следом
      const f =
        SCALE[Math.min(SCALE.length - 1, Math.max(0, Math.round(opt.k ?? 0)))];
      felt(f, t, { gain: 0.085, dur: 0.85, pan: 0.1 });
      felt(f * 1.5, t + 0.045, { gain: 0.03, dur: 0.6, pan: -0.15 });
      break;
    }
    case "unselect":
      // снять выбор: та же нота, ниже, короче и глуше
      felt(SCALE[Math.max(0, Math.round(opt.k ?? 0))] / 2, t, {
        gain: 0.06,
        dur: 0.4,
        bright: 1200,
      });
      break;
    case "fave":
      // любимый фильм: тёплое арпеджио из трёх нот
      [0, 2, 4].forEach((n, i) =>
        felt(SCALE[n + Math.min(4, Math.round(opt.k ?? 0))], t + i * 0.07, {
          gain: 0.07 - i * 0.012,
          dur: 1.1,
          pan: -0.2 + i * 0.2,
        }),
      );
      break;
    case "tick": {
      // клавиша
      const src = ac.createBufferSource(),
        hp = ac.createBiquadFilter(),
        g = ac.createGain();
      src.buffer = noiseBuf(0.02);
      hp.type = "highpass";
      hp.frequency.value = 4500;
      env(g, t, 0.05, 0.001, 0.018);
      src
        .connect(hp)
        .connect(g)
        .connect(out((Math.random() - 0.5) * 0.3, 0.05));
      src.start(t);
      break;
    }
    case "resolve":
      pad([130.81, 196, 261.63, 329.63, 392], t, 6, {
        gain: 0.055,
        attack: 0.6,
        cut: [600, 3800],
      });
      [1046.5, 1567.98].forEach((f, k) =>
        bell(f, t + 0.4 + k * 0.18, {
          gain: 0.04,
          dur: 3,
          ratio: 2.01,
          index: 1,
        }),
      );
      break;
  }
}

/** Разрешить звук: вызывать из обработчика нажатия (браузер иначе держит звук выключенным). */
export function unlock() {
  try {
    void audio().resume();
  } catch {
    /* звука нет - и ладно */
  }
}
