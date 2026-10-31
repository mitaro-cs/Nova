import { describe, it, expect } from 'vitest';
import { akaShown } from './aka';

describe('akaShown', () => {
  it('оставляет оригинал и названия с кириллицей, остальные языки убирает', () => {
    expect(
      akaShown(['First Man', 'El primer hombre en la luna', 'Aufbruch zum Mond', 'Первый человек', 'Человек на Луне'], 'First Man'),
    ).toEqual(['First Man', 'Первый человек', 'Человек на Луне']);
  });

  it('оригинал, которого нет в списке, всё равно остаётся первым', () => {
    expect(akaShown(['Primer hombre', 'Человек на Луне'], 'First Man')).toEqual(['First Man', 'Человек на Луне']);
  });

  it('пустой список без оригинала - пусто', () => {
    expect(akaShown([], null)).toEqual([]);
    expect(akaShown(['Aufbruch zum Mond'], null)).toEqual([]);
  });

  it('не больше 4 и без повторов', () => {
    const many = ['Один', 'Два', 'Три', 'Четыре', 'Пять', 'Один'];
    expect(akaShown(many, null)).toEqual(['Один', 'Два', 'Три', 'Четыре']);
  });
});
