// Постер, который на деле - широкий кадр на сплошной заливке (у новых фильмов IMDb так бывает: «WILDWOOD»
// полосой посередине, сверху и снизу ровный цвет). В карточке 2:3 он выглядит размытым и пустым - такой
// постер заменяем плиткой с названием.

const SIDE = 48;

/** Строки картинки (ширина `w`, высота `h`, RGBA): средняя яркость и разброс по ширине. */
function rows(data: Uint8ClampedArray, w: number, h: number) {
  const out: { mean: number; spread: number }[] = [];
  for (let y = 0; y < h; y++) {
    let sum = 0;
    let sq = 0;
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) * 4;
      const l = 0.299 * data[i] + 0.587 * data[i + 1] + 0.114 * data[i + 2];
      sum += l;
      sq += l * l;
    }
    const mean = sum / w;
    out.push({ mean, spread: Math.sqrt(Math.max(0, sq / w - mean * mean)) });
  }
  return out;
}

/**
 * Сверху и снизу - ровные полосы (разброс по строке < 4, соседние строки почти не отличаются), между ними -
 * кадр, у которого полоса шире, чем высока (≥ 1,6 : 1 с учётом пропорций картинки).
 */
export function isLetterboxed(data: Uint8ClampedArray, w: number, h: number, aspect = w / h): boolean {
  if (w < 8 || h < 8) return false;
  const r = rows(data, w, h);
  const flat = (i: number) => r[i].spread < 4;
  let top = 0;
  while (top < h && flat(top) && Math.abs(r[top].mean - r[0].mean) < 4) top++;
  let bottom = 0;
  while (bottom < h && flat(h - 1 - bottom) && Math.abs(r[h - 1 - bottom].mean - r[h - 1].mean) < 4) bottom++;
  const band = h - top - bottom;
  if (top < h * 0.12 || bottom < h * 0.12 || band < 3) return false;
  // Ширина полосы в долях высоты картинки: w/h * h = ширина картинки, высота полосы = band/h * высота.
  return aspect / (band / h) >= 1.6;
}

/** Загруженная картинка постера - с полосами? Чужие (не наши) адреса не читаются: считаем, что нет. */
export function posterLetterboxed(img: HTMLImageElement): boolean {
  try {
    if (!img.naturalWidth || !img.naturalHeight) return false;
    const w = SIDE;
    const h = Math.round((SIDE * img.naturalHeight) / img.naturalWidth);
    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return false;
    ctx.drawImage(img, 0, 0, w, h);
    return isLetterboxed(ctx.getImageData(0, 0, w, h).data, w, h, img.naturalWidth / img.naturalHeight);
  } catch {
    return false;
  }
}
