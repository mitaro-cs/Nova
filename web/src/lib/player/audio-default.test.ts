import { describe, expect, it } from 'vitest';
import { defaultAudioTrack } from './audio-default';

describe('defaultAudioTrack', () => {
  it('keeps the track explicitly marked as default', () => {
    expect(defaultAudioTrack([{ lang: 'en' }, { lang: 'ru', default: true }], ['en'])).toBe(1);
  });

  it('chooses the first language available in browser preferences', () => {
    expect(defaultAudioTrack([{ lang: 'ru' }, { lang: 'en-US' }], ['fr-FR', 'en-GB'])).toBe(1);
  });

  it('falls back to Russian, then to the first available track', () => {
    expect(defaultAudioTrack([{ lang: 'ja' }, { lang: 'ru-RU' }], ['fr'])).toBe(1);
    expect(defaultAudioTrack([{ lang: 'ja' }, { lang: 'en' }], [])).toBe(0);
  });

  it('reports no selection when the source has no audio tracks', () => {
    expect(defaultAudioTrack([])).toBe(-1);
  });
});
