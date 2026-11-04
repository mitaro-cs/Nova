import { api, post } from '$lib/api';
import { goto } from '$app/navigation';
import { session } from '$lib/session.svelte';
import { toast } from '$lib/ui/toast.svelte';
import { noteWatchedAt } from '$lib/progressSync.svelte';

// Значки в шапке: просьбы дружбы, непрочитанные сообщения, просьбы добавить фильм (хосту). Опрос раз в 30 с, пока вкладка
// видна, и сразу при возвращении на неё.
export interface ReadyWish {
  id: string;
  title: string;
  kind: string;
  titleId: number | null;
}
// `ready` - фильмы и сериалы, о которых зритель просил и которые хост уже добавил (пока не открыты).
export const badges = $state<{ requests: number; unread: number; wishes: number; ready: ReadyWish[] }>({ requests: 0, unread: 0, wishes: 0, ready: [] });

/** Ссылка на добавленное: свой фильм - страница фильма, иначе карточка ленты. */
export const readyHref = (r: ReadyWish) => (r.titleId ? `/title/${r.titleId}` : `/ext/${encodeURIComponent(r.id)}`);

/** Открыли добавленное - просьба прочитана, значок гаснет. */
export function seeReady(r: ReadyWish) {
  badges.ready = badges.ready.filter((x) => x.id !== r.id);
  void post(`/api/me/wishes/${encodeURIComponent(r.id)}/seen`).catch(() => {});
}

let pending: Promise<void> | undefined;

export function refreshBadges(): Promise<void> {
  if (pending) return pending;
  pending = loadBadges().finally(() => { pending = undefined; });
  return pending;
}

async function loadBadges() {
  try {
    const { watchedAt, invitation, ready = [], ...counts } = await api<{ requests: number; unread: number; wishes: number; ready?: ReadyWish[]; watchedAt?: number; invitation?: {id: number; from: string; room: string} | null }>('/api/badges');
    // Просили - добавили: один раз за вкладку говорим о каждом, значок на колокольчике остаётся, пока не откроют.
    for (const r of ready) {
      const key = `nova-ready:${session.me?.user.profile?.id ?? ''}:${r.id}`;
      let told = false; try { told = sessionStorage.getItem(key) === '1'; } catch {}
      if (told) continue;
      try { sessionStorage.setItem(key, '1'); } catch {}
      toast(`${r.kind === 'series' ? 'Сериал' : 'Фильм'} «${r.title}» добавлен - можно смотреть`, 15000, {label: 'Открыть', run: () => { seeReady(r); void goto(readyHref(r)); }});
    }
    badges.ready = ready;
    if (invitation) {
      const key = `nova-invite:${session.me?.user.profile?.id ?? ''}`;
      let last = 0; try { last = Number(sessionStorage.getItem(key) ?? 0); } catch {}
      if (invitation.id > last) {
        try { sessionStorage.setItem(key, String(invitation.id)); } catch {}
        toast(`${invitation.from} приглашает смотреть вместе`, 15000, {label: 'Открыть', run: () => { void goto(`/room/${encodeURIComponent(invitation.room)}`); }});
      }
    }
    Object.assign(badges, counts);
    noteWatchedAt(watchedAt);
  } catch {
    /* сервер недоступен - значки подождут */
  }
}

export function watchBadges(): () => void {
  refreshBadges();
  const tick = () => document.visibilityState === 'visible' && refreshBadges();
  const timer = setInterval(tick, 5000);
  document.addEventListener('visibilitychange', tick);
  // Окно приложения могло быть видно, но без фокуса: вернулись к компьютеру - сразу сверить.
  window.addEventListener('focus', tick);
  return () => {
    window.removeEventListener('focus', tick);
    clearInterval(timer);
    document.removeEventListener('visibilitychange', tick);
  };
}
