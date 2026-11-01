import { describe, it, expect } from 'vitest';
import { keyAction, type KeyInput, type KeyState } from './input';

const state: KeyState = { pipOk: true, canNext: true, hasSubs: true, inIntro: true, duration: 7200 };
const key = (code: string, extra: Partial<KeyInput> = {}): KeyInput => ({
  code,
  key: extra.key ?? code,
  shiftKey: false,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  repeat: false,
  ...extra,
});

describe('keyAction', () => {
  it('пробел и K - пауза', () => {
    expect(keyAction(key('Space'), state)).toEqual({ type: 'toggle' });
    expect(keyAction(key('KeyK'), state)).toEqual({ type: 'toggle' });
  });

  it('стрелки и J L - перемотка на 10 секунд', () => {
    expect(keyAction(key('ArrowRight'), state)).toEqual({ type: 'seek', delta: 10 });
    expect(keyAction(key('KeyL'), state)).toEqual({ type: 'seek', delta: 10 });
    expect(keyAction(key('ArrowLeft'), state)).toEqual({ type: 'seek', delta: -10 });
    expect(keyAction(key('KeyJ'), state)).toEqual({ type: 'seek', delta: -10 });
  });

  it('вверх и вниз - громкость', () => {
    expect(keyAction(key('ArrowUp'), state)).toEqual({ type: 'volume', delta: 0.1 });
    expect(keyAction(key('ArrowDown'), state)).toEqual({ type: 'volume', delta: -0.1 });
  });

  it('F, M, P, ? - во весь экран, без звука, картинка в картинке, подсказка', () => {
    expect(keyAction(key('KeyF'), state)).toEqual({ type: 'fullscreen' });
    expect(keyAction(key('KeyM'), state)).toEqual({ type: 'mute' });
    expect(keyAction(key('KeyP'), state)).toEqual({ type: 'pip' });
    expect(keyAction(key('Slash', { key: '?', shiftKey: true }), state)).toEqual({ type: 'keys' });
  });

  it('P без картинки в картинке и N без следующей серии - ничего', () => {
    expect(keyAction(key('KeyP'), { ...state, pipOk: false })).toBeNull();
    expect(keyAction(key('KeyN'), { ...state, canNext: false })).toBeNull();
    expect(keyAction(key('KeyN'), state)).toEqual({ type: 'next' });
  });

  it('S - только внутри заставки, C - только при наличии субтитров', () => {
    expect(keyAction(key('KeyS'), state)).toEqual({ type: 'skipIntro' });
    expect(keyAction(key('KeyS'), { ...state, inIntro: false })).toBeNull();
    expect(keyAction(key('KeyC'), state)).toEqual({ type: 'captions' });
    expect(keyAction(key('KeyC'), { ...state, hasSubs: false })).toBeNull();
  });

  it('цифры - к десятым долям фильма, пока длительность известна', () => {
    expect(keyAction(key('Digit5'), state)).toEqual({ type: 'percent', tenth: 5 });
    expect(keyAction(key('Digit5'), { ...state, duration: 0 })).toBeNull();
  });

  it('Shift + < > - скорость', () => {
    expect(keyAction(key('Period', { shiftKey: true }), state)).toEqual({ type: 'rate', delta: 0.25 });
    expect(keyAction(key('Comma', { shiftKey: true }), state)).toEqual({ type: 'rate', delta: -0.25 });
    expect(keyAction(key('Period'), state)).toBeNull();
  });

  it('Esc', () => {
    expect(keyAction(key('Escape'), state)).toEqual({ type: 'escape' });
  });

  it('сочетания с Ctrl, Alt и Cmd - не наши', () => {
    expect(keyAction(key('KeyF', { ctrlKey: true }), state)).toBeNull();
    expect(keyAction(key('KeyF', { metaKey: true }), state)).toBeNull();
    expect(keyAction(key('KeyF', { altKey: true }), state)).toBeNull();
  });

  it('удержание клавиши не повторяет одноразовые действия, но гасится', () => {
    expect(keyAction(key('Space', { repeat: true }), state)).toEqual({ type: 'swallow' });
    expect(keyAction(key('ArrowRight', { repeat: true }), state)).toEqual({ type: 'seek', delta: 10 });
  });

  it('неизвестная клавиша - null', () => {
    expect(keyAction(key('KeyZ'), state)).toBeNull();
  });
});
