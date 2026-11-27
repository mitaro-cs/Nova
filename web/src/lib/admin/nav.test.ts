import { describe, expect, it } from 'vitest';
import { resolve, SECTIONS } from './nav';

describe('разделы «Управления»', () => {
  it('без адреса - Сводка, первая вкладка', () => {
    expect(resolve(null, null)).toEqual({ section: 'summary', sub: 'now' });
  });
  it('прежние адреса ведут на новые места', () => {
    expect(resolve('folders', null)).toEqual({ section: 'library', sub: 'folders' });
    expect(resolve('settings', null)).toEqual({ section: 'system', sub: 'server' });
    expect(resolve('version', null)).toEqual({ section: 'system', sub: 'update' });
    expect(resolve('pult', null)).toEqual({ section: 'summary', sub: 'now' });
  });
  it('неизвестная вкладка - первая у раздела', () => {
    expect(resolve('viewers', 'nope')).toEqual({ section: 'viewers', sub: 'people' });
    expect(resolve('viewers', 'logins')).toEqual({ section: 'viewers', sub: 'logins' });
  });
  it('мастер - отдельно', () => {
    expect(resolve('setup', null).section).toBe('setup');
  });
  it('пять разделов, у вкладок разные id', () => {
    expect(SECTIONS).toHaveLength(5);
    const ids = SECTIONS.flatMap((s) => s.subs.map((x) => s.id + '/' + x.id));
    expect(new Set(ids).size).toBe(ids.length);
  });
});
