// Аватары Nova «Чернила» (выбор владельца 27.11): 36 портретов тушью, собранных из частей - лицо, причёска, брови, глаза,
// нос, рот, борода, очки и украшения, головной убор, одежда. Свои рисунки, без чужих изображений. Поле 128×128, фон
// светлый (рисует Avatar). Рисунок собирается при первом показе и кэшируется - в сборке только детали (несколько КБ).
// Номер аватара - по кругу (avatar % 36); прежние 12 номеров зверят стали первыми двенадцатью портретами.

export interface Character {
  name: string;
  /** Цвет фона круга. */
  bg: string;
  /** Внутренность SVG (поле 128×128). */
  art: string;
}

const K = '#141416', F = '#fff';
const st = (w: number = 3) => `fill="none" stroke="${K}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round"`;
const fl = (w: number = 3) => `fill="${F}" stroke="${K}" stroke-width="${w}" stroke-linejoin="round"`;
const ink = `fill="${K}"`;

const FACE: Record<string, string> = {
  oval: `<path d="M39 58 Q39 31 64 31 Q89 31 89 58 L89 68 Q89 97 64 97 Q39 97 39 68Z" ${fl()}/>`,
  round: `<ellipse cx="64" cy="65" rx="27" ry="30" ${fl()}/>`,
  square: `<path d="M38 56 Q38 31 64 31 Q90 31 90 56 L90 76 Q88 93 75 97 L53 97 Q40 93 38 76Z" ${fl()}/>`,
  long: `<path d="M41 56 Q41 28 64 28 Q87 28 87 56 L87 74 Q86 100 64 101 Q42 100 41 74Z" ${fl()}/>`,
};
const EARS = `<ellipse cx="38" cy="68" rx="6" ry="8" ${fl()}/><ellipse cx="90" cy="68" rx="6" ry="8" ${fl()}/><path d="M37 65 q2 3 0 6M91 65 q-2 3 0 6" ${st(2)}/>`;
const NECK = `<path d="M55 90 L55 108 L73 108 L73 90Z" ${fl()}/><path d="M55 98 q9 5 18 0" ${st(2)}/>`;

const BODY: Record<string, string> = {
  tee: `<path d="M16 128 Q20 106 52 102 Q64 110 76 102 Q108 106 112 128Z" ${fl()}/><path d="M52 102 Q64 112 76 102" ${st()}/>`,
  hoodie: `<path d="M14 128 Q18 104 46 99 Q64 112 82 99 Q110 104 114 128Z" ${fl()}/><path d="M44 99 Q48 116 64 116 Q80 116 84 99" ${st()}/><path d="M58 114 v10M70 114 v10" ${st(2.4)}/><circle cx="58" cy="125" r="1.8" ${ink}/><circle cx="70" cy="125" r="1.8" ${ink}/>`,
  turtle: `<path d="M16 128 Q20 106 50 102 L78 102 Q108 106 112 128Z" ${ink}/>`,
  shirt: `<path d="M16 128 Q20 106 50 102 L64 116 L78 102 Q108 106 112 128Z" ${fl()}/><path d="M50 101 L57 113 L64 106 L71 113 L78 101" ${fl()}/><path d="M64 116 v12" ${st(2.4)}/><circle cx="64" cy="121" r="1.4" ${ink}/>`,
  jacket: `<path d="M14 128 Q18 104 48 100 L64 128Z" ${ink}/><path d="M114 128 Q110 104 80 100 L64 128Z" ${ink}/><path d="M50 101 L64 126 L78 101" ${fl()}/><path d="M48 100 L56 114 L52 116 L60 128M80 100 L72 114 L76 116 L68 128" ${st(2.2)}/>`,
  stripes: `<path d="M16 128 Q20 106 52 102 Q64 110 76 102 Q108 106 112 128Z" ${fl()}/><path d="M22 116 H106M18 124 H110M30 109 H44M84 109 H98" ${st(2.6)}/><path d="M52 102 Q64 110 76 102" ${st()}/>`,
  scarf: `<path d="M16 128 Q20 106 52 102 Q64 110 76 102 Q108 106 112 128Z" ${fl()}/>`,
};

// Поверх шеи: воротник водолазки, шарф.
const TOP: Record<string, string> = {
  turtle: `<rect x="50" y="94" width="28" height="14" rx="6" ${ink}/><path d="M53 99h22M53 103h22" stroke="${F}" stroke-width="1.4" opacity=".55"/>`,
  scarf: `<path d="M48 96 Q64 108 80 96 L82 105 Q64 117 46 105Z" ${ink}/><path d="M70 109 L74 128 L65 128 L63 111Z" ${ink}/><path d="M52 101 q12 7 24 0" stroke="${F}" stroke-width="1.4" fill="none" opacity=".5"/>`,
};
// Причёски: back - за головой, front - поверх лба.
const HAIR: Record<string, { back?: string; front: string }> = {
  crop: { front: `<path d="M38 58 Q35 28 64 26 Q93 28 90 58 Q86 41 72 40 Q64 44 56 40 Q42 42 38 58Z" ${ink}/>` },
  side: { front: `<path d="M37 60 Q34 26 66 25 Q94 27 91 54 Q86 38 66 38 Q52 40 46 50 Q42 54 37 60Z" ${ink}/><path d="M58 30 Q70 33 82 42" stroke="${F}" stroke-width="1.6" fill="none" opacity=".5"/>` },
  bob: { front: `<path d="M33 78 Q30 26 64 25 Q98 26 95 78 L95 90 L84 90 L84 52 Q76 44 64 47 Q50 44 44 52 L44 90 L33 90Z" ${ink}/>` },
  long: { back: `<path d="M30 60 Q28 22 64 22 Q100 22 98 60 L102 118 Q90 124 82 112 L46 112 Q38 124 26 118Z" ${ink}/>`, front: `<path d="M39 60 Q38 30 64 29 Q90 30 89 60 Q82 42 64 42 Q60 36 54 44 Q46 46 39 60Z" ${ink}/>` },
  afro: { front: [[40, 40, 13], [52, 28, 14], [68, 25, 15], [83, 32, 14], [92, 46, 12], [36, 54, 10], [94, 58, 10]].map(([x, y, r]) => `<circle cx="${x}" cy="${y}" r="${r}" ${ink}/>`).join('') },
  bun: { front: `<circle cx="64" cy="20" r="11" ${ink}/><path d="M40 56 Q40 31 64 31 Q88 31 88 56 Q84 40 64 39 Q44 40 40 56Z" ${ink}/><path d="M56 22 q8 -5 16 0" stroke="${F}" stroke-width="1.6" fill="none" opacity=".5"/>` },
  pony: { back: `<path d="M86 40 Q108 46 104 78 Q100 96 94 102 Q98 78 88 58Z" ${ink}/>`, front: `<path d="M39 58 Q38 30 64 29 Q90 30 89 58 Q84 40 64 40 Q46 40 39 58Z" ${ink}/>` },
  bald: { front: `<path d="M48 40 q8 -6 18 -5" stroke="${K}" stroke-width="2" fill="none" opacity=".35"/>` },
  spiky: { front: `<path d="M38 58 L36 34 L46 40 L46 24 L56 34 L62 20 L68 33 L78 22 L80 36 L90 30 L90 58 Q84 42 64 41 Q44 42 38 58Z" ${ink}/>` },
  braids: { back: `<path d="M34 64 Q30 92 36 118" stroke="${K}" stroke-width="9" fill="none" stroke-linecap="round"/><path d="M94 64 Q98 92 92 118" stroke="${K}" stroke-width="9" fill="none" stroke-linecap="round"/><path d="M30 86 l8 -3M31 98 l8 -3M32 110 l8 -2M90 83 l8 3M89 95 l8 3M88 107 l8 2" stroke="${F}" stroke-width="1.6" opacity=".6"/>`, front: `<path d="M38 62 Q36 28 64 27 Q92 28 90 62 Q86 44 64 44 Q42 44 38 62Z" ${ink}/><path d="M64 28 v16" stroke="${F}" stroke-width="1.8"/>` },
  fringe: { front: `<path d="M37 62 Q34 26 64 25 Q94 26 91 62 L86 50 L80 54 L76 46 L70 52 L64 45 L58 52 L52 46 L48 54 L42 50Z" ${ink}/>` },
  wavy: { back: `<path d="M30 58 Q26 22 64 21 Q102 22 98 58 Q104 72 96 84 Q102 98 92 110 L36 110 Q26 98 32 84 Q24 72 30 58Z" ${ink}/>`, front: `<path d="M39 60 Q36 30 64 29 Q92 30 89 60 Q86 46 76 42 Q66 48 56 41 Q44 44 39 60Z" ${ink}/>` },
  buzz: { front: `<path d="M39 56 Q39 30 64 30 Q89 30 89 56 Q84 42 64 42 Q44 42 39 56Z" fill="${K}" opacity=".28"/>` + [[48, 38], [56, 35], [64, 34], [72, 35], [80, 38], [44, 46], [84, 46], [52, 41], [76, 41]].map(([x, y]) => `<circle cx="${x}" cy="${y}" r="1.1" ${ink}/>`).join('') },
  curtains: { front: `<path d="M38 64 Q34 26 64 26 Q94 26 90 64 Q86 46 70 42 L64 50 L58 42 Q42 46 38 64Z" ${ink}/>` },
  mohawk: { front: `<path d="M56 46 L54 22 L60 28 L62 16 L66 26 L70 16 L72 28 L76 22 L74 46Z" ${ink}/><path d="M40 56 Q40 34 56 32M88 56 Q88 34 72 32" ${st(2)}/>` },
};
const BROWS: Record<string, string> = {
  flat: `<path d="M48 56 h11M69 56 h11" ${st(2.8)}/>`,
  up: `<path d="M48 55 q5 -4 11 -1M69 54 q6 -3 11 1" ${st(2.8)}/>`,
  angry: `<path d="M48 54 l11 3M80 54 l-11 3" ${st(2.8)}/>`,
  thick: `<path d="M47 56 q6 -4 13 -1v3 q-7 -3 -13 1Z M68 55 q7 -3 13 1 v-3 q-6 -4 -13 -1Z" ${ink} stroke="${K}" stroke-width="1.5"/>`,
  raised: `<path d="M48 55 q5 -2 11 0M69 50 q6 -4 11 0" ${st(2.8)}/>`,
};
const EYES: Record<string, string> = {
  dot: `<circle cx="54" cy="66" r="2.8" ${ink}/><circle cx="74" cy="66" r="2.8" ${ink}/>`,
  happy: `<path d="M50 67 q4 -5 8 0M70 67 q4 -5 8 0" ${st(2.6)}/>`,
  open: `<ellipse cx="54" cy="66" rx="4.6" ry="3.6" ${fl(2.2)}/><ellipse cx="74" cy="66" rx="4.6" ry="3.6" ${fl(2.2)}/><circle cx="55" cy="66" r="2" ${ink}/><circle cx="75" cy="66" r="2" ${ink}/>`,
  wink: `<circle cx="54" cy="66" r="2.8" ${ink}/><path d="M70 67 q4 -4 8 0" ${st(2.6)}/>`,
  sleepy: `<path d="M50 67 q4 3 8 0M70 67 q4 3 8 0" ${st(2.6)}/>`,
  lash: `<circle cx="54" cy="66" r="2.8" ${ink}/><circle cx="74" cy="66" r="2.8" ${ink}/><path d="M50 63 l-3 -2M78 63 l3 -2" ${st(2)}/>`,
};
const NOSE: Record<string, string> = {
  line: `<path d="M64 69 q-4 8 0 11 h4" ${st(2.4)}/>`,
  small: `<path d="M62 78 q2 2 4 0" ${st(2.4)}/>`,
  hook: `<path d="M65 66 l-5 12 q3 3 7 1" ${st(2.4)}/>`,
  round: `<path d="M60 78 q4 4 8 0" ${st(2.4)}/><circle cx="64" cy="75" r="2.4" fill="none" stroke="${K}" stroke-width="2"/>`,
};
const MOUTH: Record<string, string> = {
  smile: `<path d="M56 86 q8 7 16 0" ${st(2.6)}/>`,
  grin: `<path d="M54 85 q10 12 20 0Z" ${ink} stroke="${K}" stroke-width="2.4" stroke-linejoin="round"/><path d="M57 87 h14" stroke="${F}" stroke-width="2"/>`,
  neutral: `<path d="M58 88 h12" ${st(2.6)}/>`,
  smirk: `<path d="M57 88 q8 2 14 -4" ${st(2.6)}/>`,
  o: `<ellipse cx="64" cy="88" rx="3.4" ry="4" ${ink}/>`,
  lips: `<path d="M56 86 q4 -3 8 0 q4 -3 8 0 q-8 8 -16 0Z" ${ink}/>`,
};
const FACIAL: Record<string, string> = {
  none: '',
  beard: `<path d="M39 68 Q40 104 64 106 Q88 104 89 68 Q84 84 76 86 Q64 80 52 86 Q44 84 39 68Z" ${ink}/><path d="M57 88 q7 4 14 0" stroke="${F}" stroke-width="2.4" fill="none" stroke-linecap="round"/>`,
  moustache: `<path d="M52 83 Q58 77 64 81 Q70 77 76 83 Q70 84 64 82 Q58 84 52 83Z" ${ink} stroke="${K}" stroke-width="2"/>`,
  goatee: `<path d="M58 92 Q64 104 70 92 Q64 95 58 92Z" ${ink}/><path d="M55 83 Q64 79 73 83" ${st(3)}/>`,
  stubble: [[48, 86], [52, 91], [58, 94], [64, 95], [70, 94], [76, 91], [80, 86], [55, 89], [73, 89], [61, 92], [67, 92]].map(([x, y]) => `<circle cx="${x}" cy="${y}" r="1" ${ink}/>`).join(''),
};
const ACC: Record<string, string> = {
  round: `<circle cx="54" cy="66" r="8.5" fill="none" stroke="${K}" stroke-width="2.6"/><circle cx="74" cy="66" r="8.5" fill="none" stroke="${K}" stroke-width="2.6"/><path d="M62.5 65 h3M45.5 64 l-6 -2M82.5 64 l6 -2" ${st(2.4)}/>`,
  square: `<rect x="44" y="59" width="19" height="13" rx="3" fill="none" stroke="${K}" stroke-width="2.6"/><rect x="65" y="59" width="19" height="13" rx="3" fill="none" stroke="${K}" stroke-width="2.6"/><path d="M63 64 h2M44 62 l-5 -1M84 62 l5 -1" ${st(2.4)}/>`,
  shades: `<path d="M43 60 h20 v6 q-1 7 -10 7 q-9 0 -10 -7Z M65 60 h20 v6 q-1 7 -10 7 q-9 0 -10 -7Z" ${ink}/><path d="M63 62 h2M43 61 l-5 -1M85 61 l5 -1" ${st(2.4)}/><path d="M47 63 l4 -1" stroke="${F}" stroke-width="1.6" opacity=".7"/>`,
  earrings: `<circle cx="37" cy="80" r="3" fill="none" stroke="${K}" stroke-width="2.2"/><circle cx="91" cy="80" r="3" fill="none" stroke="${K}" stroke-width="2.2"/>`,
  freckles: [[48, 74], [52, 77], [46, 78], [80, 74], [76, 77], [82, 78]].map(([x, y]) => `<circle cx="${x}" cy="${y}" r="1.2" ${ink}/>`).join(''),
  headphones: `<path d="M32 66 Q30 18 64 18 Q98 18 96 66" ${st(4)}/><rect x="27" y="58" width="11" height="20" rx="5" ${ink}/><rect x="90" y="58" width="11" height="20" rx="5" ${ink}/>`,
  mole: `<circle cx="78" cy="82" r="1.6" ${ink}/>`,
  pencil: `<path d="M86 52 L104 40" stroke="${K}" stroke-width="4" stroke-linecap="round"/><path d="M104 40 l3 -2" stroke="${K}" stroke-width="2"/>`,
};
const HAT: Record<string, string> = {
  none: '',
  beanie: `<path d="M36 56 Q36 22 64 22 Q92 22 92 56Z" ${ink}/><rect x="34" y="48" width="60" height="10" rx="4" ${fl(2.4)}/><path d="M42 50 v6M50 50 v6M58 50 v6M66 50 v6M74 50 v6M82 50 v6" ${st(1.8)}/><circle cx="64" cy="18" r="6" ${ink}/>`,
  cap: `<path d="M37 54 Q37 26 64 26 Q91 26 91 54Z" ${ink}/><path d="M80 50 Q106 48 110 56 Q96 58 82 56Z" ${ink}/><path d="M64 27 v24" stroke="${F}" stroke-width="1.4" opacity=".5"/>`,
  beret: `<path d="M34 46 Q36 26 66 26 Q96 28 94 44 Q90 52 64 50 Q40 52 34 46Z" ${ink}/><path d="M64 26 l2 -6" ${st(3)}/>`,
  fedora: `<path d="M44 44 Q44 24 64 24 Q84 24 84 44Z" ${ink}/><ellipse cx="64" cy="46" rx="34" ry="6" ${ink}/><path d="M45 40 h38" stroke="${F}" stroke-width="2.4"/>`,
  bow: `<path d="M74 30 L86 22 L86 38Z M98 30 L86 22 L86 38Z" ${ink}/><circle cx="86" cy="30" r="3" ${fl(2)}/>`,
};

// ---------- 36 человек ----------
// [имя, лицо, причёска, брови, глаза, нос, рот, борода, [аксессуары], шапка, одежда]
const PEOPLE: [string, string, string, string, string, string, string, string, string[], string, string][] = [
  ['Анна', 'oval', 'bob', 'up', 'dot', 'small', 'smile', 'none', ['earrings'], 'none', 'turtle'],
  ['Марк', 'square', 'side', 'flat', 'dot', 'line', 'smirk', 'stubble', [], 'none', 'jacket'],
  ['Кира', 'round', 'afro', 'raised', 'happy', 'small', 'grin', 'none', ['freckles'], 'none', 'stripes'],
  ['Лёша', 'long', 'curtains', 'flat', 'open', 'hook', 'neutral', 'none', ['round'], 'none', 'shirt'],
  ['Вера', 'oval', 'bun', 'up', 'lash', 'small', 'lips', 'none', ['earrings'], 'none', 'scarf'],
  ['Тимур', 'square', 'buzz', 'thick', 'dot', 'round', 'smile', 'beard', [], 'none', 'hoodie'],
  ['Ника', 'round', 'fringe', 'up', 'wink', 'small', 'smile', 'none', ['freckles'], 'bow', 'tee'],
  ['Дима', 'oval', 'crop', 'angry', 'dot', 'line', 'neutral', 'none', ['square'], 'none', 'turtle'],
  ['Соня', 'long', 'long', 'up', 'dot', 'small', 'smile', 'none', ['headphones'], 'none', 'hoodie'],
  ['Гоша', 'round', 'bald', 'raised', 'happy', 'round', 'grin', 'moustache', [], 'none', 'stripes'],
  ['Лена', 'oval', 'wavy', 'flat', 'sleepy', 'small', 'smile', 'none', ['mole'], 'none', 'shirt'],
  ['Арсений', 'long', 'spiky', 'angry', 'dot', 'hook', 'smirk', 'goatee', ['earrings'], 'none', 'jacket'],
  ['Полина', 'oval', 'pony', 'up', 'open', 'small', 'o', 'none', [], 'none', 'tee'],
  ['Саша', 'square', 'crop', 'flat', 'dot', 'line', 'smile', 'none', ['shades'], 'cap', 'hoodie'],
  ['Мира', 'round', 'braids', 'up', 'happy', 'small', 'grin', 'none', ['freckles'], 'none', 'stripes'],
  ['Олег', 'square', 'side', 'thick', 'sleepy', 'round', 'neutral', 'beard', ['square'], 'none', 'shirt'],
  ['Ева', 'long', 'bob', 'raised', 'lash', 'small', 'lips', 'none', [], 'beret', 'turtle'],
  ['Рома', 'oval', 'mohawk', 'angry', 'wink', 'line', 'grin', 'none', ['earrings'], 'none', 'jacket'],
  ['Ульяна', 'oval', 'long', 'up', 'happy', 'small', 'smile', 'none', ['round'], 'none', 'scarf'],
  ['Филипп', 'long', 'buzz', 'flat', 'dot', 'hook', 'smirk', 'stubble', [], 'beanie', 'hoodie'],
  ['Зоя', 'round', 'afro', 'up', 'open', 'small', 'smile', 'none', ['headphones'], 'none', 'tee'],
  ['Борис', 'square', 'bald', 'thick', 'dot', 'round', 'neutral', 'beard', ['round'], 'none', 'jacket'],
  ['Тая', 'oval', 'fringe', 'up', 'dot', 'small', 'o', 'none', ['freckles', 'earrings'], 'none', 'stripes'],
  ['Игорь', 'oval', 'crop', 'raised', 'happy', 'line', 'grin', 'moustache', [], 'fedora', 'shirt'],
  ['Варя', 'round', 'bun', 'raised', 'wink', 'small', 'smile', 'none', ['pencil'], 'none', 'tee'],
  ['Кирилл', 'long', 'curtains', 'flat', 'sleepy', 'line', 'neutral', 'goatee', ['headphones'], 'none', 'hoodie'],
  ['Ася', 'oval', 'wavy', 'up', 'lash', 'small', 'lips', 'none', ['shades'], 'none', 'turtle'],
  ['Глеб', 'square', 'spiky', 'flat', 'dot', 'hook', 'smile', 'stubble', [], 'none', 'stripes'],
  ['Лиза', 'round', 'pony', 'up', 'happy', 'small', 'grin', 'none', ['earrings'], 'cap', 'tee'],
  ['Егор', 'oval', 'side', 'angry', 'open', 'round', 'smirk', 'none', ['square'], 'none', 'jacket'],
  ['Яна', 'long', 'braids', 'raised', 'dot', 'small', 'smile', 'none', ['mole'], 'none', 'scarf'],
  ['Миша', 'round', 'curtains', 'up', 'wink', 'small', 'grin', 'none', ['freckles'], 'beanie', 'stripes'],
  ['Даша', 'oval', 'long', 'flat', 'sleepy', 'small', 'neutral', 'none', ['round', 'earrings'], 'none', 'shirt'],
  ['Стас', 'square', 'crop', 'thick', 'dot', 'line', 'neutral', 'beard', ['shades'], 'none', 'turtle'],
  ['Аля', 'round', 'bob', 'up', 'open', 'small', 'o', 'none', ['headphones'], 'none', 'hoodie'],
  ['Пётр', 'long', 'bald', 'raised', 'dot', 'hook', 'smile', 'moustache', ['round'], 'beret', 'jacket'],
];

function draw([, face, hair, brow, eyes, nose, mouth, facial, acc, hat, body]: (typeof PEOPLE)[number]): string {
  const h = HAIR[hair];
  return (
    (h.back ?? '') + BODY[body] + NECK + (TOP[body] ?? '') + EARS + FACE[face] + h.front +
    BROWS[brow] + EYES[eyes] + NOSE[nose] + FACIAL[facial] + (facial === 'beard' ? '' : MOUTH[mouth]) +
    acc.map((a) => ACC[a]).join('') + HAT[hat]
  );
}

const cache = new Map<number, string>();
export const CHARACTERS: Character[] = PEOPLE.map((p, i) => ({
  name: p[0],
  bg: '#f1f1f2',
  get art() {
    let a = cache.get(i);
    if (a === undefined) cache.set(i, (a = draw(p)));
    return a;
  },
}));

export const characterOf = (avatar: number) => CHARACTERS[((avatar % CHARACTERS.length) + CHARACTERS.length) % CHARACTERS.length];
