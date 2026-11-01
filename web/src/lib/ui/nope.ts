import { drop, setMark } from '$lib/marks';
import type { Card, ExtCard } from '$lib/types';
import { toast } from './toast.svelte';

/** «Не интересно»: больше не показывать и не предлагать (то же, что «Больше не смотрю»), с отменой. */
export function nope(card?: Card, ext?: ExtCard) {
  if (card) return void drop(card, card.title);
  if (!ext) return;
  const before = ext.mark ?? null;
  void setMark(ext, 'dropped');
  toast(`«${ext.title}» - больше не предлагаем`, 6000, { label: 'Вернуть', run: () => void setMark(ext, before) });
}
