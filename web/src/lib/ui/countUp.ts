// Число «набегает» от нуля до значения (1,3 с, замедление к концу). При выключенных анимациях
// (`html.no-motion`) и у пользователей без requestAnimationFrame - сразу итог.
type Opts = { value: number; decimals?: number };

const fmt = (v: number, decimals: number) => (decimals ? v.toFixed(decimals).replace('.', ',') : String(Math.round(v)));

export function countUp(node: HTMLElement, opts: Opts) {
  let frame = 0;
  let shown = 0;
  function run({ value, decimals = 0 }: Opts) {
    cancelAnimationFrame(frame);
    const still = document.documentElement.classList.contains('no-motion');
    if (still || typeof requestAnimationFrame !== 'function') {
      shown = value;
      node.textContent = fmt(value, decimals);
      return;
    }
    const from = shown;
    let t0 = 0;
    const step = (t: number) => {
      if (!t0) t0 = t;
      const k = Math.min(1, (t - t0) / 1300);
      shown = from + (value - from) * (1 - Math.pow(1 - k, 3));
      node.textContent = fmt(shown, decimals);
      if (k < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
  }
  run(opts);
  return {
    update: run,
    destroy: () => cancelAnimationFrame(frame),
  };
}
