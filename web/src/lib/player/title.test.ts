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
it('серия уже в названии ссылки - не повторяем', () => {
  expect(playbackHeading('The Sopranos - 4 сезон, 7 серия', '', 4, 7)).toBe('The Sopranos - 4 сезон, 7 серия');
  expect(playbackHeading('Клан Сопрано', '', 4, 7)).toBe('Клан Сопрано · 4 сезон, 7 серия');
  expect(playbackHeading('Сериал 14 сезон, 7 серия', '', 4, 7)).toBe('Сериал 14 сезон, 7 серия · 4 сезон, 7 серия');
});

it('экран паузы: название отдельно, сезон и серия - строкой ниже', async () => {
  const { pauseHeading } = await import('./title');
  expect(pauseHeading('Сопрано - 4 сезон, 8 серия', '')).toEqual({ name: 'Сопрано', sub: '4 сезон · 8 серия' });
  expect(pauseHeading('Тьма', '1 сезон, 2 серия · Ложь', 1, 2)).toEqual({ name: 'Тьма', sub: '1 сезон · 2 серия · Ложь' });
  expect(pauseHeading('Интерстеллар', '')).toEqual({ name: 'Интерстеллар', sub: '' });
});
