// Контур знака Nova для заставки: берём его из того же `logo.svg`, что и везде (в сборку не попадает -
// грузится, только когда заставка идёт).

/** Все контуры (`d`) из текста SVG; нет ни одного - пустой список. */
export function logoPaths(svg: string): string[] {
  return [...svg.matchAll(/\sd="([^"]+)"/g)].map((m) => m[1]);
}

/**
 * Длина самого длинного контура (в единицах `viewBox`): штрих такой длины рисует каждый контур от начала
 * к концу, и самый длинный дорисовывается ровно к концу анимации. Не измерили - с запасом.
 */
export function longestSubpath(d: string): number {
  try {
    const probe = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    let longest = 0;
    for (const sub of d.split(/(?=M)/)) {
      probe.setAttribute('d', sub);
      longest = Math.max(longest, probe.getTotalLength());
    }
    return longest > 1 ? Math.ceil(longest) : 400;
  } catch {
    return 400;
  }
}

/** Размер области знака из `viewBox` («0 0 100 50.91»); не разобрали - null. */
export function logoBox(svg: string): { w: number; h: number } | null {
  const m = /viewBox="\s*[\d.-]+\s+[\d.-]+\s+([\d.]+)\s+([\d.]+)\s*"/.exec(svg);
  return m ? { w: Number(m[1]), h: Number(m[2]) } : null;
}
