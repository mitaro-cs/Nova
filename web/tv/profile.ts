// «Профиль»: кто вошёл и выход. Оформление профиля, друзья и чаты - в приложении на телефоне и компьютере.

import { bar, library, session, type Screen } from './app';
import { esc } from './util';

export const profileScreen: Screen = async (root) => {
  const lib = await library();
  const me = session.me;
  const watched = lib.items.filter((c) => c.status === 'watched').length;
  const watching = lib.items.filter((c) => c.status === 'watching').length;
  root.innerHTML =
    bar('profile') +
    '<section class="page"><h1 class="title">' + esc(me ? me.user.name : 'Профиль') + '</h1>' +
    (me ? '<p class="hint">@' + esc(me.user.login) + '</p>' : '') +
    '<p class="hint">Просмотрено: ' + watched + '  ·  Смотрю: ' + watching + '</p>' +
    '<div class="btns"><a class="f btn pri" data-first data-k="logout" data-act="logout">Выйти</a></div></section>';
};
