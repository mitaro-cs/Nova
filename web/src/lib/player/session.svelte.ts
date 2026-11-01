// Конечный автомат воспроизведения: ведёт цепочку режимов (`plan.ts`), единственный владелец
// `<video>`. Не заработало - следующий вариант с того же места, без чёрного экрана; зависло посреди
// фильма - ступень вниз (не чаще раза в 20 с); перемотка - не зависание.
import { LAST_START_TIMEOUT_MS, START_TIMEOUT_MS, STALL_TIMEOUT_MS, type Mode, type Step } from './plan';

export type State = 'idle' | 'loading' | 'ready' | 'playing' | 'paused' | 'buffering' | 'ended' | 'failed';

/** Запускает поток. Сам режим выбирает цепочка, движок только исполняет шаг. */
export interface Engine {
  start(video: HTMLVideoElement, step: Step, at: number): Promise<void>;
  destroy(): void;
  /** Фатальная ошибка потока: `network` или `decode`. */
  onFatal(cb: (code: string) => void): void;
}

export interface PlayEvent {
  type: 'start' | 'fallback' | 'stall' | 'fatal';
  mode: Mode;
  to?: Mode;
  code?: string;
}

interface Deps {
  chain: Step[];
  makeEngine: (step: Step) => Engine;
  report: (e: PlayEvent) => void;
}

/** Ожидание сразу после перемотки - не зависание. */
const SEEK_GRACE_MS = 1500;
/** Снижать качество не чаще, чтобы не дёргать туда-сюда. */
const STALL_COOLDOWN_MS = 20_000;

export class PlaySession {
  state = $state<State>('idle');
  step = $state<Step | null>(null);
  failure = $state('');

  #chain: Step[];
  #makeEngine: Deps['makeEngine'];
  #report: Deps['report'];
  #video: HTMLVideoElement | null = null;
  #engine: Engine | null = null;
  #idx = 0;
  #gen = 0;
  #place = 0;
  #seekUntil = 0;
  #lastLower = -Infinity;
  #startTimer: ReturnType<typeof setTimeout> | null = null;
  #stallTimer: ReturnType<typeof setTimeout> | null = null;
  #listeners: [string, () => void][] = [];

  constructor(deps: Deps) {
    this.#chain = deps.chain.map((s) => ({ ...s }));
    this.#makeEngine = deps.makeEngine;
    this.#report = deps.report;
  }

  attach(video: HTMLVideoElement) {
    this.#detach();
    this.#video = video;
    const on = (name: string, fn: () => void) => {
      video.addEventListener(name, fn);
      this.#listeners.push([name, fn]);
    };
    on('playing', () => this.#onPlaying());
    // Данные первого кадра пришли: старт удался, даже если браузер не дал запустить сам (автозапуск
    // без нажатия заблокирован) - зритель нажмёт «Смотреть». Менять из-за этого режим нельзя.
    on('loadeddata', () => this.#onReady());
    on('canplay', () => this.#onReady());
    on('pause', () => this.#onPause());
    on('waiting', () => this.#onWaiting());
    on('error', () => this.#fail(this.#gen, 'decode'));
    on('ended', () => this.#onEnded());
  }

  /** Начать с первого варианта цепочки с места `at` (секунды). */
  start(at: number) {
    this.#place = at;
    this.#begin(0, at, true);
  }

  /** Последнее известное место просмотра: с него начнётся запасной вариант. */
  setPlace(t: number) {
    this.#place = t;
  }

  seek(t: number) {
    this.#place = t;
    this.#seekUntil = Date.now() + SEEK_GRACE_MS;
    if (this.#step()?.mode === 'remux') this.#begin(this.#idx, t, false);
    else if (this.#video) this.#video.currentTime = t;
  }

  /** После сбоя: заново с первого варианта, с того же места. */
  retry() {
    this.#begin(0, this.#place, true);
  }

  destroy() {
    this.#gen++;
    this.#clearTimers();
    this.#stopEngine();
    this.#detach();
    this.#video = null;
    this.state = 'idle';
  }

  #step(): Step | undefined {
    return this.#chain[this.#idx];
  }

  #begin(idx: number, at: number, announce: boolean) {
    this.#clearTimers();
    // Сначала «загрузка», потом останавливаем прежний поток: его остановка у `<video>` может дать
    // событие `pause`, а оно не должно считаться паузой зрителя.
    this.state = 'loading';
    this.#stopEngine();
    const gen = ++this.#gen;
    this.#idx = idx;
    this.failure = '';
    const step = this.#chain[idx];
    if (!step) {
      this.#finish('no_variant', 'raw');
      return;
    }
    this.step = step;
    if (announce) this.#report({ type: 'start', mode: step.mode });
    const engine = this.#makeEngine(step);
    this.#engine = engine;
    engine.onFatal((code) => this.#fail(gen, code));
    const last = idx === this.#chain.length - 1;
    this.#startTimer = setTimeout(() => this.#fail(gen, 'timeout'), last ? LAST_START_TIMEOUT_MS : START_TIMEOUT_MS);
    if (this.#video) engine.start(this.#video, step, at).catch(() => this.#fail(gen, 'network'));
  }

  #fail(gen: number, code: string) {
    if (gen !== this.#gen) return;
    const next = this.#idx + 1;
    const cur = this.#step();
    if (cur && next < this.#chain.length) {
      this.#report({ type: 'fallback', mode: cur.mode, to: this.#chain[next].mode, code });
      this.#begin(next, this.#place, false);
    } else {
      this.#finish(code, cur?.mode ?? 'raw');
    }
  }

  #finish(code: string, mode: Mode) {
    this.#clearTimers();
    this.#stopEngine();
    this.failure = code === 'timeout' ? 'network' : code;
    this.state = 'failed';
    this.#report({ type: 'fatal', mode, code: this.failure });
  }

  #onPlaying() {
    this.#clearTimers();
    this.state = 'playing';
  }

  #onReady() {
    if (this.state !== 'loading') return;
    if (this.#startTimer) clearTimeout(this.#startTimer);
    this.#startTimer = null;
    this.state = 'ready';
  }

  #onPause() {
    if (this.state === 'playing' || this.state === 'buffering') {
      this.#clearStall();
      this.state = 'paused';
    }
  }

  #onEnded() {
    this.#clearTimers();
    this.state = 'ended';
  }

  #onWaiting() {
    if (this.state !== 'playing' && this.state !== 'buffering' && this.state !== 'paused') return;
    if (this.state === 'paused') return;
    this.state = 'buffering';
    this.#clearStall();
    const gen = this.#gen;
    // Ожидание считается с конца окна после перемотки: сама перемотка - не зависание.
    const wait = Math.max(0, this.#seekUntil - Date.now()) + STALL_TIMEOUT_MS;
    this.#stallTimer = setTimeout(() => this.#onStall(gen), wait);
  }

  #onStall(gen: number) {
    if (gen !== this.#gen || this.state !== 'buffering') return;
    const now = Date.now();
    if (now - this.#lastLower < STALL_COOLDOWN_MS) return;
    const cur = this.#step();
    if (!cur) return;
    this.#lastLower = now;
    if (cur.mode !== 'hls') {
      const next = this.#chain[this.#idx + 1];
      if (!next) return;
      this.#report({ type: 'stall', mode: cur.mode, to: next.mode });
      this.#begin(this.#idx + 1, this.#place, false);
    } else if (cur.quality !== 'auto') {
      this.#report({ type: 'stall', mode: 'hls', to: 'hls' });
      this.#chain[this.#idx] = { ...cur, quality: 'auto' };
      this.#begin(this.#idx, this.#place, false);
    } else {
      // Адаптивное качество подстраивается само (hls.js): только отмечаем в журнале.
      this.#report({ type: 'stall', mode: 'hls' });
    }
  }

  #clearStall() {
    if (this.#stallTimer) clearTimeout(this.#stallTimer);
    this.#stallTimer = null;
  }

  #clearTimers() {
    if (this.#startTimer) clearTimeout(this.#startTimer);
    this.#startTimer = null;
    this.#clearStall();
  }

  #stopEngine() {
    this.#engine?.destroy();
    this.#engine = null;
  }

  #detach() {
    if (this.#video) for (const [name, fn] of this.#listeners) this.#video.removeEventListener(name, fn);
    this.#listeners = [];
  }
}
