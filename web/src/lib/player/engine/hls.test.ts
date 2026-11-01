import { describe, it, expect, beforeEach } from 'vitest';
import { createHlsEngine, type HlsLike } from './hls';
import type { Step } from '../plan';

type Handler = (event: string, data?: unknown) => void;

class FakeHls {
  static Events = { MANIFEST_PARSED: 'manifest', LEVEL_SWITCHED: 'level', ERROR: 'error' };
  static ErrorTypes = { MEDIA_ERROR: 'media', NETWORK_ERROR: 'network' };
  static isSupported = () => true;
  static last: FakeHls | null = null;
  handlers = new Map<string, Handler[]>();
  levels = [
    { height: 1080, width: 1920 },
    { height: 1080, width: 1920 },
    { height: 720, width: 1280 },
  ];
  currentLevel = -1;
  loadLevel = -1;
  src = '';
  media: unknown = null;
  destroyed = false;
  loads = 0;
  recovers = 0;
  constructor(public config: Record<string, unknown>) {
    FakeHls.last = this;
  }
  on(event: string, cb: Handler) {
    this.handlers.set(event, [...(this.handlers.get(event) ?? []), cb]);
  }
  loadSource(u: string) {
    this.src = u;
  }
  attachMedia(v: unknown) {
    this.media = v;
  }
  destroy() {
    this.destroyed = true;
  }
  startLoad() {
    this.loads++;
  }
  recoverMediaError() {
    this.recovers++;
  }
  emit(event: string, data?: unknown) {
    for (const h of this.handlers.get(event) ?? []) h(event, data);
  }
}

const loader = async () => ({ default: FakeHls as unknown as HlsLike });
const auto: Step = { mode: 'hls', quality: 'auto', why: '' };
const video = {} as HTMLVideoElement;
const base = { fileId: 7, audio: 0, mbit: 20, light: true, loader };

describe('createHlsEngine', () => {
  beforeEach(() => {
    FakeHls.last = null;
  });

  it('грузит общий список с нужным звуком и начинает с места', async () => {
    const e = createHlsEngine(base);
    await e.start(video, auto, 90);
    const h = FakeHls.last!;
    expect(h.src).toBe('/api/files/7/hls/master.m3u8?audio=0');
    expect(h.config.startPosition).toBe(90);
    expect(h.media).toBe(video);
    expect(h.config.maxBufferLength).toBe(60);
  });

  it('высота: список этой высоты', async () => {
    const e = createHlsEngine(base);
    await e.start(video, { mode: 'hls', quality: 720, why: '' }, 0);
    expect(FakeHls.last!.src).toBe('/api/files/7/hls/720/index.m3u8?audio=0');
  });

  it('три фатальные ошибки восстанавливаются, четвёртая - onFatal', async () => {
    const e = createHlsEngine(base);
    const fatals: string[] = [];
    e.onFatal((c) => fatals.push(c));
    await e.start(video, auto, 0);
    const h = FakeHls.last!;
    for (let i = 0; i < 3; i++) h.emit('error', { fatal: true, type: 'network' });
    expect(fatals).toEqual([]);
    expect(h.loads).toBe(3);
    h.emit('error', { fatal: true, type: 'network' });
    expect(fatals).toEqual(['network']);
  });

  it('ошибка медиа - recoverMediaError, а потом decode', async () => {
    const e = createHlsEngine(base);
    const fatals: string[] = [];
    e.onFatal((c) => fatals.push(c));
    await e.start(video, auto, 0);
    const h = FakeHls.last!;
    for (let i = 0; i < 4; i++) h.emit('error', { fatal: true, type: 'media' });
    expect(h.recovers).toBe(3);
    expect(fatals).toEqual(['decode']);
  });

  it('нефатальные ошибки игнорируются', async () => {
    const e = createHlsEngine(base);
    const fatals: string[] = [];
    e.onFatal((c) => fatals.push(c));
    await e.start(video, auto, 0);
    FakeHls.last!.emit('error', { fatal: false, type: 'network' });
    expect(fatals).toEqual([]);
  });

  it('уровни: без повторов подписи, от лучшего', async () => {
    const e = createHlsEngine(base);
    let got: { i: number; height: number }[] = [];
    e.onLevels((l) => (got = l));
    await e.start(video, auto, 0);
    FakeHls.last!.emit('manifest');
    expect(got.map((l) => l.height)).toEqual([1080, 720]);
  });

  it('setLevel ставит уровень, destroy - освобождает', async () => {
    const e = createHlsEngine(base);
    await e.start(video, auto, 0);
    e.setLevel(2);
    expect(FakeHls.last!.currentLevel).toBe(2);
    expect(FakeHls.last!.loadLevel).toBe(2);
    e.destroy();
    expect(FakeHls.last!.destroyed).toBe(true);
  });

  it('destroy во время загрузки библиотеки - поток не стартует', async () => {
    let release!: () => void;
    const slow = () => new Promise<{ default: HlsLike }>((ok) => (release = () => ok({ default: FakeHls as unknown as HlsLike })));
    const e = createHlsEngine({ ...base, loader: slow });
    const started = e.start(video, auto, 0);
    e.destroy();
    release();
    await started;
    expect(FakeHls.last).toBeNull();
  });

  it('браузер без поддержки - onFatal(decode)', async () => {
    class NoHls extends FakeHls {
      static isSupported = () => false;
    }
    const e = createHlsEngine({ ...base, loader: async () => ({ default: NoHls as unknown as HlsLike }) });
    const fatals: string[] = [];
    e.onFatal((c) => fatals.push(c));
    await e.start(video, auto, 0);
    expect(fatals).toEqual(['decode']);
  });

  it('видео по ссылке: свой адрес, запас вперёд меньше', async () => {
    const e = createHlsEngine({ ...base, source: 'https://x/index.m3u8', light: false });
    await e.start(video, auto, 5);
    expect(FakeHls.last!.src).toBe('https://x/index.m3u8');
    expect(FakeHls.last!.config.maxBufferLength).toBe(20);
  });
});
