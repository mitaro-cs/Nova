import { describe, it, expect } from 'vitest';
import { createNativeEngine } from './native';
import type { Step } from '../plan';

class FakeVideo extends EventTarget {
  src = '';
  currentTime = 0;
  loads = 0;
  removed = false;
  removeAttribute(name: string) {
    if (name === 'src') {
      this.src = '';
      this.removed = true;
    }
  }
  load() {
    this.loads++;
  }
}

const step = (mode: Step['mode']): Step => ({ mode, quality: 'orig', why: '' });
const ctx = { fileId: 7, audio: 0, hevc: false };
const v = () => new FakeVideo();
const asVideo = (x: FakeVideo) => x as unknown as HTMLVideoElement;

describe('createNativeEngine', () => {
  it('raw: ставит адрес файла, место - после загрузки метаданных', async () => {
    const video = v();
    const e = createNativeEngine(ctx);
    await e.start(asVideo(video), step('raw'), 125);
    expect(video.src).toBe('/api/files/7/raw');
    expect(video.currentTime).toBe(0);
    video.dispatchEvent(new Event('loadedmetadata'));
    expect(video.currentTime).toBe(125);
    expect(e.offset).toBe(0);
  });

  it('remux: адрес с места, смещение времени потока', async () => {
    const video = v();
    const e = createNativeEngine({ ...ctx, audio: 1, hevc: true });
    await e.start(asVideo(video), step('remux'), 60);
    expect(video.src).toBe('/api/files/7/play?t=60.00&audio=1&hevc=1');
    expect(e.offset).toBe(60);
    video.dispatchEvent(new Event('loadedmetadata'));
    expect(video.currentTime).toBe(0);
  });

  it('copy: HLS-копия', async () => {
    const video = v();
    await createNativeEngine(ctx).start(asVideo(video), step('copy'), 10.9);
    expect(video.src).toBe('/api/files/7/copy/0/index.m3u8?from=10');
  });

  it('destroy: снимает источник и слушатель', async () => {
    const video = v();
    const e = createNativeEngine(ctx);
    await e.start(asVideo(video), step('raw'), 50);
    e.destroy();
    expect(video.removed).toBe(true);
    expect(video.loads).toBe(1);
    video.dispatchEvent(new Event('loadedmetadata'));
    expect(video.currentTime).toBe(0);
  });
});
