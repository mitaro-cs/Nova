import type { Card, MiniTitle, NowItem, Person, ProfileDecor, ProfileStats, Relation } from '$lib/types';

/** Ответ `GET /api/members/{id}`: страница человека. Закрытое владельцем сервер не отдаёт: `stats` - null, списки пусты. */
export interface ProfilePage {
  person: Person;
  me: boolean;
  relation: Relation | null;
  stats: ProfileStats | null;
  decor: ProfileDecor;
  reviews: {
    id: number;
    stars: number | null;
    body: string;
    createdAt: number;
    title?: MiniTitle;
    ext?: { id: string; kind: string; title: string; year: number | null; poster: string | null };
  }[];
  watched: Card[];
  watching: NowItem[];
  hidden: { stats: boolean; watched: boolean; reviews: boolean };
}

export type ProfileTab = 'home' | 'stats' | 'watched' | 'reviews';
export const TABS: ProfileTab[] = ['home', 'stats', 'watched', 'reviews'];
