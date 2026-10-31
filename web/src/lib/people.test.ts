import { describe, it, expect } from 'vitest';
import { personHref } from './people';

describe('personHref', () => {
  it('ключ - оригинальное имя, русское - отдельным параметром', () => {
    const u = new URL(personHref({ name: 'Райан Гослинг', orig: 'Ryan Gosling', job: 'actor', id: 'nm0331516' }), 'http://x');
    expect(u.searchParams.get('name')).toBe('Ryan Gosling');
    expect(u.searchParams.get('ru')).toBe('Райан Гослинг');
    expect(u.searchParams.get('id')).toBe('nm0331516');
  });

  it('без orig ссылка как раньше', () => {
    const u = new URL(personHref({ name: 'Тим Роббинс', job: 'actor' }), 'http://x');
    expect(u.searchParams.get('name')).toBe('Тим Роббинс');
    expect(u.searchParams.has('ru')).toBe(false);
  });
});
