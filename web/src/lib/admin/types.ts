export interface UpdateInfo {
  state: 'idle' | 'checking' | 'latest' | 'available' | 'downloading' | 'installing' | 'error';
  latest: string | null;
  progress: number | null;
  error: string | null;
  checkedAt: number | null;
}

/** `GET /api/admin/health`: диагностика приложения. */
export interface Health {
  version: string;
  uptimeSec: number;
  dbBytes: number;
  /** Последние предупреждения и ошибки, новые первыми (время - мс Unix). */
  errors: { at: number; level: 'WARN' | 'ERROR'; target: string; message: string }[];
  /** Источники видео: `down` - выключен на 10 минут после трёх ошибок подряд. */
  sources: { site: string; label: string; state: 'ok' | 'down' | 'unknown'; fails: number; ok_ms: number | null }[];
}

export interface Status {
  system?: { supported: boolean; sampledAt: number; cpu: number | null; memoryUsed: number | null; memoryTotal: number | null; diskFree: number | null; diskTotal: number | null };
  version: string;
  /** null - сервер без приложения (`fsfy serve`): обновлять нечего. */
  update: UpdateInfo | null;
  titles: number;
  files: number;
  size: number;
  metaPending: number;
  /** Свои фильмы с файлами, которым описание не нашлось. */
  unmatched: number;
  unprobed: number;
  scan: { running: boolean; at: number | null; error: string | null; progress: number | null; last: { files: number; added: number; missing: number; titles: number } | null };
  meta: { running: boolean; error: string | null; done: number };
  ffmpeg: boolean;
  ffprobe: boolean;
  watching: { user: string; title: string; episode: string; position: number; duration: number; net: PlayNet | null; backdrop?: string | null; poster?: string | null; avatar?: number; photo?: string | null; href?: string }[];
  canPick: boolean;
  publicUrl: string | null;
  /** Своя папка для фильмов (null - ещё нет). */
  inbox: string | null;
  /** Копирование перетащенных в окно файлов. */
  import: { running: boolean; filesTotal: number; filesDone: number; bytesTotal: number; bytesDone: number; current: string | null; error: string | null };
}

/** Как идёт просмотр у зрителя - последний отчёт его плеера. */
export interface PlayNet {
  /** Скорость до хоста, Мбит/с (null - замер не вышел). */
  mbit: number | null;
  /** «orig», «auto» или высота вроде «720p». */
  quality: string;
  /** Сколько раз видео останавливалось на загрузку. */
  stalls: number;
  at: number;
}

/** `GET /api/admin/metrics`: показатели «Обзора» (библиотека, зрители, активность, диск). */
export interface Metrics {
  at: number;
  library: {
    movies: number;
    series: number;
    episodes: number;
    /** Суммарная длительность всех видео, секунд. */
    seconds: number;
    addedWeek: number;
    addedMonth: number;
    hidden: number;
    resolutions: { q: string; files: number; bytes: number }[];
    codecs: { codec: string; files: number }[];
  };
  people: { users: number; newWeek: number; loginsDay: number; loginsWeek: number; failedWeek: number; activeWeek: number };
  activity: {
    /** Две недели, старые первыми: `day` - начало суток (мс Unix), `views` - сколько файлов смотрели, `people` - сколько человек. */
    days: { day: number; views: number; people: number }[];
    /** Просмотры по часам суток (24 числа) за месяц. */
    hours: number[];
    top: { id: string; href: string; title: string; year: number | null; kind: string; viewers: number; finished: number }[];
  };
  social: { reviews: number; messages: number; lists: number; friends: number; wishes: number };
  disk: { db: number; images: number; hls: number; copy: number; cacheOther: number; backups: number; logs: number };
  /** Сколько копий базы лежит в папке `backups`. */
  backups: number;
}
