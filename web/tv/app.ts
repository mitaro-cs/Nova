// Общее для экранов ТВ: переходы, обработчик клавиш экрана, библиотека, шапка.

import { api } from './api';
import { esc } from './util';
import { ICONS, type IconName } from '../src/lib/ui/icons';
import type { Library, Me } from '../src/lib/types';

export type Cleanup = () => void;

export interface Route {
  name: string;
  args: string[];
  query: URLSearchParams;
}

export type Screen = (root: HTMLElement, route: Route) => Promise<Cleanup | void> | Cleanup | void;

/** Экран может перехватить клавишу: вернуть true - дальше не обрабатывать. */
export const ctx: { onKey: ((e: KeyboardEvent) => boolean) | null } = { onKey: null };

export function go(path: string, replace?: boolean) {
  const hash = '#' + path;
  if (replace) location.replace(location.pathname + location.search + hash);
  else location.hash = path;
}

export function back() {
  if (history.length > 1) history.back();
  else go('/', true);
}

// ---------- кто вошёл ----------

export const session: { me: Me | null } = { me: null };

export async function loadMe(): Promise<Me | null> {
  try {
    session.me = await api<Me>('/api/me');
  } catch (e) {
    session.me = null;
  }
  return session.me;
}

// ---------- библиотека: один запрос на все экраны, свежесть - 20 секунд ----------

let lib: Library | null = null;
let libAt = 0;

export async function library(force?: boolean): Promise<Library> {
  if (!lib || force || Date.now() - libAt > 20000) {
    lib = await api<Library>('/api/library');
    libAt = Date.now();
  }
  return lib;
}

export function forgetLibrary() {
  libAt = 0;
}

// ---------- панель разделов (такая же, как на телефоне и компьютере) ----------

const TABS: [string, string, string, IconName][] = [
  ['home', '/', 'Главная', 'home'],
  ['feed', '/feed', 'Подбор', 'squareplay'],
  ['profile', '/profile', 'Профиль', 'user'],
  ['my', '/my', 'Подборки', 'bookmark'],
  ['search', '/search', 'Поиск', 'search'],
];

function icon(name: IconName): string {
  return (
    '<svg width="30" height="30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" ' +
    'stroke-linecap="round" stroke-linejoin="round"><path d="' + ICONS[name] + '"/></svg>'
  );
}

/** Одна панель внизу на всех экранах: Главное, Что смотреть, Профиль, Коллекция, Поиск. */
export function bar(active: string): string {
  let h = '<nav class="dock">';
  for (let i = 0; i < TABS.length; i++) {
    const t = TABS[i];
    h +=
      '<a class="f d' + (t[0] === active ? ' on' : '') + '" data-k="tab-' + t[0] + '" data-href="' + t[1] + '">' +
      icon(t[3]) + '<span>' + esc(t[2]) + '</span></a>';
  }
  return h + '</nav>';
}

/** Разделы главной полосой над кадром: Моё кино, Фильмы, Сериалы (как полоса на телефоне). */
export function sections(active: string): string {
  const list: [string, string, string][] = [
    ['home', '/', 'Моё кино'],
    ['movies', '/movies', 'Фильмы'],
    ['series', '/series', 'Сериалы'],
  ];
  let h = '<nav class="strip">';
  for (let i = 0; i < list.length; i++) {
    const s = list[i];
    h += '<a class="f sec-tab' + (s[0] === active ? ' on' : '') + '" data-k="sec-' + s[0] + '" data-href="' + s[1] + '">' + esc(s[2]) + '</a>';
  }
  return h + '</nav>';
}
