import { describe, it, expect } from 'vitest';
import { languageRu, languagesRu } from './countries';

describe('языки', () => {
  it('переводит названия с IMDb', () => {
    expect(languagesRu(['English', 'Mandarin', 'Russian'])).toEqual(['английский', 'китайский', 'русский']);
  });

  it('неизвестный язык остаётся как есть', () => {
    expect(languageRu('Klingon')).toBe('Klingon');
  });

  it('знает редкие языки', () => {
    for (const l of ['Indonesian', 'Vietnamese', 'Bengali', 'Tamil', 'Romanian', 'Bulgarian', 'Serbian', 'Croatian', 'Icelandic', 'Estonian']) {
      expect(languageRu(l), l).toMatch(/[а-яё]/);
    }
  });
});
