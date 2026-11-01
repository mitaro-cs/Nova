// Пропуск заставки и титров, следующая серия с отсчётом (как у Netflix). Правила - те же, что у прежнего плеера.

/** Внутри заставки: граница на секунду раньше конца, чтобы кнопка не мигала на стыке. */
export function inIntro(time: number, intro: [number, number] | null): boolean {
  return !!intro && time >= intro[0] && time < intro[1] - 1;
}

export interface CreditsState {
  /** С какой секунды начинаются титры (или null). */
  credits: number | null;
  hasNext: boolean;
  series: boolean;
  /** Зритель уже нажал «Остаться». */
  skipped: boolean;
  failed: boolean;
}

/** Титры сериала со следующей серией: пора показать карточку «Далее» и отсчёт. */
export function inCredits(time: number, s: CreditsState): boolean {
  return s.hasNext && s.series && s.credits !== null && time >= s.credits && !s.skipped && !s.failed;
}

/** Меньше 90 секунд до конца: пора готовить следующее видео по ссылке. */
export function nearEnd(time: number, duration: number): boolean {
  return duration > 0 && time >= duration - 90;
}

/** Обратный отсчёт до следующей серии; в конце - `onDone` один раз. */
export class NextEpisode {
  left = $state<number | null>(null);
  #timer: ReturnType<typeof setInterval> | null = null;
  #done: () => void;

  constructor(onDone: () => void) {
    this.#done = onDone;
  }

  start(from: number) {
    this.#stop();
    this.left = from;
    this.#timer = setInterval(() => {
      if (this.left === null) return this.#stop();
      this.left--;
      if (this.left <= 0) {
        this.#stop();
        this.#done();
      }
    }, 1000);
  }

  cancel() {
    this.#stop();
    this.left = null;
  }

  /** Остановить отсчёт, не сбрасывая число (видео поставили на паузу в титрах). */
  pause() {
    this.#stop();
  }

  #stop() {
    if (this.#timer) clearInterval(this.#timer);
    this.#timer = null;
  }
}
