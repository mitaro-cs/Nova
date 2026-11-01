import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { PlaySession, type Engine, type PlayEvent } from './session.svelte';
import { LAST_START_TIMEOUT_MS, START_TIMEOUT_MS, STALL_TIMEOUT_MS, type Step } from './plan';

class FakeVideo extends EventTarget {
  currentTime = 0;
  seeking = false;
  paused = true;
}

class FakeEngine implements Engine {
  started: { step: Step; at: number }[] = [];
  destroyed = false;
  private fatal: ((code: string) => void) | null = null;
  constructor(public step: Step) {}
  async start(_v: HTMLVideoElement, step: Step, at: number) {
    this.started.push({ step, at });
  }
  destroy() {
    this.destroyed = true;
  }
  onFatal(cb: (code: string) => void) {
    this.fatal = cb;
  }
  fail(code: string) {
    this.fatal?.(code);
  }
}

const raw: Step = { mode: 'raw', quality: 'orig', why: '' };
const remux: Step = { mode: 'remux', quality: 'orig', why: '' };
const hls: Step = { mode: 'hls', quality: 'auto', why: '' };

function setup(chain: Step[]) {
  const video = new FakeVideo();
  const engines: FakeEngine[] = [];
  const events: PlayEvent[] = [];
  const session = new PlaySession({
    chain,
    makeEngine: (step) => {
      const e = new FakeEngine(step);
      engines.push(e);
      return e;
    },
    report: (e) => events.push(e),
  });
  session.attach(video as unknown as HTMLVideoElement);
  return { video, engines, events, session };
}

const fire = (v: FakeVideo, name: string) => v.dispatchEvent(new Event(name));

describe('PlaySession', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('start: первый вариант, состояние loading, событие start', () => {
    const { engines, events, session } = setup([raw, hls]);
    session.start(30);
    expect(session.state).toBe('loading');
    expect(engines).toHaveLength(1);
    expect(engines[0].started[0].at).toBe(30);
    expect(events).toEqual([{ type: 'start', mode: 'raw' }]);
  });

  it('falls_back_after_start_timeout: нет первого кадра за 8 с - следующий вариант с того же места', () => {
    const { engines, events, session } = setup([raw, hls]);
    session.start(30);
    vi.advanceTimersByTime(START_TIMEOUT_MS + 1);
    expect(engines).toHaveLength(2);
    expect(engines[0].destroyed).toBe(true);
    expect(engines[1].step.mode).toBe('hls');
    expect(engines[1].started[0].at).toBe(30);
    expect(events).toContainEqual({ type: 'fallback', mode: 'raw', to: 'hls', code: 'timeout' });
  });

  it('место для запасного варианта - последнее известное', () => {
    const { engines, session } = setup([raw, hls]);
    session.start(30);
    session.setPlace(55);
    vi.advanceTimersByTime(START_TIMEOUT_MS + 1);
    expect(engines[1].started[0].at).toBe(55);
  });

  it('playing снимает таймер старта: запасной вариант не включается', () => {
    const { video, engines, session } = setup([raw, hls]);
    session.start(0);
    fire(video, 'playing');
    expect(session.state).toBe('playing');
    vi.advanceTimersByTime(START_TIMEOUT_MS * 3);
    expect(engines).toHaveLength(1);
  });

  it('fatal_error_goes_to_next_step: фатальная ошибка движка', () => {
    const { engines, events, session } = setup([remux, hls]);
    session.start(10);
    engines[0].fail('network');
    expect(engines).toHaveLength(2);
    expect(events).toContainEqual({ type: 'fallback', mode: 'remux', to: 'hls', code: 'network' });
  });

  it('ошибка <video> - тоже переход', () => {
    const { video, engines, session } = setup([raw, hls]);
    session.start(10);
    fire(video, 'error');
    expect(engines).toHaveLength(2);
  });

  it('chain_exhausted_is_failed_with_code: варианты кончились - failed и событие fatal', () => {
    const { engines, events, session } = setup([raw, hls]);
    session.start(0);
    engines[0].fail('decode');
    engines[1].fail('network');
    expect(session.state).toBe('failed');
    expect(session.failure).toBe('network');
    expect(events.at(-1)).toEqual({ type: 'fatal', mode: 'hls', code: 'network' });
  });

  it('пустая цепочка - failed с кодом no_variant', () => {
    const { session, events } = setup([]);
    session.start(0);
    expect(session.state).toBe('failed');
    expect(session.failure).toBe('no_variant');
    expect(events.at(-1)).toMatchObject({ type: 'fatal', code: 'no_variant' });
  });

  it('последний вариант ждёт дольше: хост может готовить плейлист десятки секунд', () => {
    const { session } = setup([hls]);
    session.start(0);
    vi.advanceTimersByTime(START_TIMEOUT_MS + 1);
    expect(session.state).toBe('loading');
    vi.advanceTimersByTime(LAST_START_TIMEOUT_MS);
    expect(session.state).toBe('failed');
    expect(session.failure).toBe('network');
  });

  it('при переходе состояние уже loading, когда старый движок останавливается', () => {
    const states: string[] = [];
    const video = new FakeVideo();
    let session!: PlaySession;
    session = new PlaySession({
      chain: [raw, hls],
      makeEngine: (step) => {
        const e = new FakeEngine(step);
        e.destroy = () => void states.push(session.state);
        return e;
      },
      report: () => {},
    });
    session.attach(video as unknown as HTMLVideoElement);
    session.start(0);
    fire(video, 'playing');
    session.seek(10);
    session.start(20);
    expect(states).toEqual(['loading']);
  });

  it('stall_lowers_quality_once_per_20s: зависание посреди фильма - ступень вниз, не чаще раза в 20 с', () => {
    const { video, engines, events, session } = setup([raw, hls]);
    session.start(0);
    fire(video, 'playing');
    fire(video, 'waiting');
    expect(session.state).toBe('buffering');
    vi.advanceTimersByTime(STALL_TIMEOUT_MS + 1);
    expect(engines).toHaveLength(2);
    expect(engines[1].step.mode).toBe('hls');
    expect(events).toContainEqual({ type: 'stall', mode: 'raw', to: 'hls' });
    // Снова зависло сразу - второго перехода нет (гистерезис).
    fire(video, 'playing');
    fire(video, 'waiting');
    vi.advanceTimersByTime(STALL_TIMEOUT_MS + 1);
    expect(events.filter((e) => e.type === 'stall')).toHaveLength(1);
  });

  it('зависание в адаптивном режиме с выбранной высотой - на авто', () => {
    const { video, engines, session } = setup([{ mode: 'hls', quality: 720, why: '' }, raw]);
    session.start(0);
    fire(video, 'playing');
    fire(video, 'waiting');
    vi.advanceTimersByTime(STALL_TIMEOUT_MS + 1);
    expect(engines).toHaveLength(2);
    expect(engines[1].started[0].step).toMatchObject({ mode: 'hls', quality: 'auto' });
  });

  it('playing до истечения порога - зависания не было', () => {
    const { video, engines, session } = setup([raw, hls]);
    session.start(0);
    fire(video, 'playing');
    fire(video, 'waiting');
    vi.advanceTimersByTime(STALL_TIMEOUT_MS - 500);
    fire(video, 'playing');
    vi.advanceTimersByTime(STALL_TIMEOUT_MS * 2);
    expect(engines).toHaveLength(1);
    expect(session.state).toBe('playing');
  });

  it('seek_waiting_is_not_stall: ожидание сразу после перемотки не считается зависанием', () => {
    const { video, engines, events, session } = setup([raw, hls]);
    session.start(0);
    fire(video, 'playing');
    session.seek(120);
    fire(video, 'waiting');
    vi.advanceTimersByTime(STALL_TIMEOUT_MS + 1);
    expect(engines).toHaveLength(1);
    expect(events.some((e) => e.type === 'stall')).toBe(false);
  });

  it('перемотка при перепаковке перезапускает поток с нового места', () => {
    const { video, engines, session } = setup([remux, hls]);
    session.start(0);
    fire(video, 'playing');
    session.seek(300);
    expect(engines).toHaveLength(2);
    expect(engines[1].step.mode).toBe('remux');
    expect(engines[1].started[0].at).toBe(300);
    expect(engines[0].destroyed).toBe(true);
  });

  it('перемотка при прямом файле двигает currentTime', () => {
    const { video, engines, session } = setup([raw, hls]);
    session.start(0);
    fire(video, 'playing');
    session.seek(300);
    expect(video.currentTime).toBe(300);
    expect(engines).toHaveLength(1);
  });

  it('late_engine_answer_is_ignored: ответ старого движка после нового start', () => {
    const { engines, session } = setup([raw, hls]);
    session.start(0);
    const old = engines[0];
    session.start(50);
    expect(old.destroyed).toBe(true);
    old.fail('network');
    expect(engines).toHaveLength(2);
    expect(session.state).toBe('loading');
  });

  it('retry: заново с первого варианта с того же места', () => {
    const { engines, session } = setup([raw, hls]);
    session.start(0);
    session.setPlace(70);
    engines[0].fail('decode');
    engines[1].fail('network');
    expect(session.state).toBe('failed');
    session.retry();
    expect(session.state).toBe('loading');
    expect(engines.at(-1)!.step.mode).toBe('raw');
    expect(engines.at(-1)!.started[0].at).toBe(70);
  });

  it('пауза и конец - состояния', () => {
    const { video, session } = setup([raw]);
    session.start(0);
    fire(video, 'playing');
    fire(video, 'pause');
    expect(session.state).toBe('paused');
    fire(video, 'ended');
    expect(session.state).toBe('ended');
  });

  it('destroy: движок остановлен, события больше не действуют', () => {
    const { video, engines, session } = setup([raw, hls]);
    session.start(0);
    session.destroy();
    expect(engines[0].destroyed).toBe(true);
    fire(video, 'error');
    vi.advanceTimersByTime(START_TIMEOUT_MS * 2);
    expect(engines).toHaveLength(1);
  });
});
