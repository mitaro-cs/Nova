// «Что смотреть»: кадры из фильмов и краткая информация о них - то, что ещё не смотрели, новое сверху.

import { bar, library, type Screen } from './app';
import { feedCard } from './cards';

export const feedScreen: Screen = async (root) => {
  const lib = await library();
  const items = lib.items
    .filter((c) => c.status !== 'watched' && c.status !== 'dropped')
    .sort((a, b) => b.addedAt - a.addedAt)
    .slice(0, 24);
  let h = bar('feed') + '<section class="page"><h1 class="title">Подбор</h1>';
  if (!items.length) h += '<div class="empty small">Пока нечего предложить</div>';
  else {
    h += '<div class="feedgrid">';
    for (let i = 0; i < items.length; i++) h += feedCard(items[i], 'feed-');
    h += '</div>';
  }
  root.innerHTML = h + '</section>';
  const first = root.querySelector('.card');
  if (first) first.setAttribute('data-first', '');
};
