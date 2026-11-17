import {expect,it} from 'vitest';import {playbackTitle} from './title';
it('оставляет название без оригинала и технических сведений',()=>{
  expect(playbackTitle('Тёмный рыцарь | The Dark Knight (2008, 4K)')).toBe('Тёмный рыцарь');
  expect(playbackTitle('Тёмный рыцарь (2008, 4K)')).toBe('Тёмный рыцарь');
  expect(playbackTitle('1984')).toBe('1984');expect(playbackTitle('Оно (часть вторая)')).toBe('Оно (часть вторая)');
});
