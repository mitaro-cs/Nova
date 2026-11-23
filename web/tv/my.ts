// «Коллекция»: то, что отмечено - «Буду смотреть», «Смотрю», «Просмотрено» (своё, с сервиса).

import { bar, library, type Screen } from './app';
import { row } from './home';

export const myScreen: Screen = async (root) => {
  const lib = await library(true);
  const groups: [string, string][] = [
    ['want', 'Буду смотреть'],
    ['watching', 'Смотрю'],
    ['watched', 'Просмотрено'],
  ];
  let h = bar('my') + '<section class="page"><h1 class="title">Моё</h1></section><div class="rows">';
  let any = false;
  for (let i = 0; i < groups.length; i++) {
    const g = groups[i];
    const items = lib.items.filter((c) => c.status === g[0]);
    if (!items.length) continue;
    any = true;
    h += row('my-' + g[0], g[1], items, false);
  }
  if (!any) h += '<div class="empty small">Здесь будет то, что вы отметите: «Хочу», «Смотрю» и «Просмотрено»</div>';
  root.innerHTML = h + '</div>';
  const first = root.querySelector('.card');
  if (first) first.setAttribute('data-first', '');
};
