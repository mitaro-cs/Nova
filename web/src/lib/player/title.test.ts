import {expect,it} from 'vitest';import {playbackHeading,playbackTitle} from './title';
it('оставляет название без оригинала и технических сведений',()=>{
  expect(playbackTitle('Тёмный рыцарь | The Dark Knight (2008, 4K)')).toBe('Тёмный рыцарь');
  expect(playbackTitle('Тёмный рыцарь (2008, 4K)')).toBe('Тёмный рыцарь');
  expect(playbackTitle('1984')).toBe('1984');expect(playbackTitle('Оно (часть вторая)')).toBe('Оно (часть вторая)');
  expect(playbackTitle('Клан Сопрано 1 сезон 1 серия смотреть онлайн')).toBe('Клан Сопрано 1 сезон 1 серия');
});
it('заголовок - фильм и серия, без подписей источника',()=>{
  expect(playbackHeading('Клан Сопрано','1 сезон, 2 серия · Название')).toBe('Клан Сопрано · 1 сезон, 2 серия');
  expect(playbackHeading('Друзья','',1,5)).toBe('Друзья · 1 сезон, 5 серия');
  expect(playbackHeading('Тёмный рыцарь | The Dark Knight','')).toBe('Тёмный рыцарь');
});
