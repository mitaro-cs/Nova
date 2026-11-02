import { describe, expect, it } from 'vitest';
import { logoBox, logoPaths } from './introLogo';

const svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50.91"><path fill="#fff" fill-rule="evenodd" d="M1 1L2 2Z"/></svg>';

describe('знак для заставки', () => {
  it('контуры и размер берутся из logo.svg', () => {
    expect(logoPaths(svg)).toEqual(['M1 1L2 2Z']);
    expect(logoBox(svg)).toEqual({ w: 100, h: 50.91 });
  });

  it('без контуров и viewBox - пусто', () => {
    expect(logoPaths('<svg></svg>')).toEqual([]);
    expect(logoBox('<svg></svg>')).toBeNull();
  });
});
