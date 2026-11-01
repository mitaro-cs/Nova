<script lang="ts">
  import { goto } from '$app/navigation';
  import { untrack } from 'svelte';
  import type { Snippet } from 'svelte';
  import { api } from '$lib/api';
  import { clock } from '$lib/fmt';
  import { forgetCatalog } from '$lib/catalog.svelte';
  import { extChanged } from '$lib/discover/sheet.svelte';
  import { noteOwnSave, progressSync } from '$lib/progressSync.svelte';
  import type { FileItem, PlayInfo, TitleDetail } from '$lib/types';
  import Controls from './Controls.svelte';
  import Overlay from './Overlay.svelte';
  import Panels from './Panels.svelte';
  import { heightLabel } from './codecs';
  import { createHlsEngine, type HlsEngine, type Level } from './engine/hls';
  import { createLinkEngine } from './engine/link';
  import { createNativeEngine, type NativeEngine } from './engine/native';
  import { keyAction } from './input';
  import { levels as ladderOf } from './levels';
  import { nativeLevels } from './native-master';
  import { buildChain, detectCaps, type Quality, type Step } from './plan';
  import { createProgress, elsewherePosition } from './progress';
  import { qualityTier } from './quality-label';
  import { browserName, reportEvent } from './report';
  import { PlaySession, type Engine, type PlayEvent } from './session.svelte';
  import { inCredits as inCreditsAt, inIntro as inIntroAt, nearEnd, NextEpisode } from './skip.svelte';
  import { forgetSpeed, measureSpeed, startQuality } from './speed';
  import { needsSeek, remotePosition, type PlayerSync } from './sync';
  import { loadSubtitles, savedLook, saveLook, type SubLook } from './tracks';
  import { cueAt, type Cue } from './vtt';

  // Плеер: оболочка над модулями. Режим воспроизведения выбирает цепочка (`plan.ts`), ход ведёт
  // `PlaySession` (переход к запасному режиму с того же места, снижение качества при зависании),
  // а здесь - только то, что видит и делает зритель: управление, меню, клавиши, жесты, комната.
  // `src` - видео не с компьютера хоста (Rutube, VK, свой сайт, прямая ссылка - поток через сервер):
  // тот же плеер, без субтитров и озвучек.
  let {
    fileId = 0,
    src = null,
    onfail,
    sync = null,
    sidebar = false,
    alts = [],
    onalt,
    onwrong,
    expect = null,
    nextSource = null,
    onnext,
    onfinish,
    onNearEnd,
    children,
  }: {
    fileId?: number;
    /** `linkId` - ссылка, у которой сохраняем место; `start` - откуда начать (где остановился). */
    src?: { url: string; hls: boolean; title: string; linkId?: number; start?: number } | null;
    onfail?: () => void;
    sync?: PlayerSync | null;
    sidebar?: boolean;
    /** Видео по ссылке: другие видео этого фильма (другая озвучка) - меню «Озвучка». */
    alts?: { id: number; label: string }[];
    onalt?: (id: number) => void;
    /** Видео по ссылке оказалось не тем фильмом - убрать источник и взять следующий. */
    onwrong?: () => void;
    /** Длительность фильма (с): видео заметно короче или длиннее - похоже, это не он. */
    expect?: number | null;
    nextSource?: { id: number; title: string } | null;
    onnext?: () => void;
    onfinish?: () => void;
    onNearEnd?: () => void;
    children?: Snippet;
  } = $props();

  let info = $state<PlayInfo | null>(null);
  let video: HTMLVideoElement | undefined = $state();
  let box: HTMLDivElement | undefined = $state();
  let localError = $state('');
  let toast = $state('');

  // ---------- воспроизведение ----------
  let session = $state.raw<PlaySession | null>(null);
  let nativeEngine: NativeEngine | null = null;
  let hlsEngine: HlsEngine | null = null;
  let quality = $state<Quality>('orig');
  let audio = $state(0);
  let hlsHeight = $state<number | null>(null);
  let srcLevels = $state<Level[]>([]);
  let srcLevel = $state(-1);
  // Скорость до хоста у этого зрителя (Мбит/с) и сколько раз видео останавливалось - для выбора
  // качества и для «Управления» у хоста. Замер - сразу, параллельно со сведениями о файле.
  let mbit = $state<number | null>(null);
  let stallCount = 0;
  // Уже подвисало у этого зрителя - следующие серии сразу в автокачестве.
  let slow = false;
  const speed = typeof window === 'undefined' ? Promise.resolve(null) : measureSpeed();
  let desiredPlaying = true;

  let current = $state(0);
  let buffered = $state(0);
  let offset = $state(0);
  let mediaDuration = $state(0);
  let paused = $state(true);
  let volume = $state(1);
  let lastVolume = 1;
  let muted = $state(false);
  let full = $state(false);
  let shown = $state(true);
  let dragging = $state(false);
  let panel = $state('');
  let rate = $state(1);
  let jump = $state<{ side: 'left' | 'right'; n: number; key: number } | null>(null);
  let topH = $state(80);
  let bottomH = $state(120);
  let roomBusy = $state(false);
  let mutedByBrowser = $state(false);

  const pipOk = typeof document !== 'undefined' && !!document.pictureInPictureEnabled;
  const mode = $derived(session?.step?.mode ?? 'raw');
  const failed = $derived(session?.state === 'failed');
  const waiting = $derived(!info && !localError ? true : session?.state === 'loading' || session?.state === 'buffering');
  const duration = $derived(info?.play.duration || (mediaDuration ? offset + mediaDuration : 0));
  const time = $derived(offset + current);
  const intro = $derived(info?.markers.intro ?? null);
  const hevcOk = $derived.by(() => {
    if (!info?.play.hevc) return true;
    const caps = detectCaps(video);
    return info.play.tenBit ? caps.hevcMain10 : caps.hevcMain;
  });
  const origOk = $derived.by(() => {
    const p = info?.play;
    if (!p) return false;
    return (p.direct && audio === 0 && hevcOk) || (p.remux && !p.transcode && hevcOk) || (!p.remux && !info?.hls);
  });
  const ladder = $derived(ladderOf(info?.play.height ?? null));
  const bufferedPct = $derived(duration ? Math.min(100, ((offset + buffered) / duration) * 100) : 0);

  const qualityLabel = $derived.by(() => {
    const auto = `Авто${hlsHeight ? ` · ${qualityTier(hlsHeight)}p` : ''}`;
    if (src) return srcLevel === -1 ? auto : `${srcLevels.find((l) => l.i === srcLevel)?.label ?? qualityTier(srcLevels.find((l) => l.i === srcLevel)?.height ?? 720)}p`;
    const step = session?.step;
    if (step?.mode !== 'hls') return 'Оригинал';
    return step.quality === 'auto' ? auto : typeof step.quality === 'number' ? heightLabel(step.quality) : 'Оригинал';
  });

  const failText = $derived.by(() => {
    if (!failed) return '';
    const code = session?.failure;
    if (src) return 'Видео с сайта не воспроизводится - откройте его плеером сайта';
    if (code === 'network') return 'Видео не загружается. Проверьте интернет или скачайте файл';
    return info?.play.remux
      ? 'Видео не воспроизводится. Попробуйте ещё раз или скачайте файл'
      : 'Этот формат браузер не играет. Скачайте файл или напишите администрации Nova';
  });
  const error = $derived(localError || failText);

  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  function say(text: string) {
    clearTimeout(toastTimer);
    toast = text;
    toastTimer = setTimeout(() => (toast = ''), 2200);
  }

  function onEvent(e: PlayEvent) {
    reportEvent(e, { browser: browserName() });
    if (e.type === 'stall') {
      stallCount++;
      slow = true;
      forgetSpeed();
      say('Интернет медленный - снизили качество');
    } else if (e.type === 'fallback') {
      say('Переключаю режим…');
    }
  }

  function makeEngine(step: Step): Engine {
    const caps = detectCaps(video);
    if (src) {
      const e = createLinkEngine({ url: src.url, hls: src.hls, nativeHls: caps.nativeHls, startLevel: srcLevel });
      if ('onLevels' in e && e.onLevels && e.onLevel) {
        hlsEngine = e as HlsEngine;
        e.onLevels((l) => (srcLevels = l));
        e.onLevel((h) => (hlsHeight = h));
      }
      return e;
    }
    if (step.mode === 'hls') {
      const e = createHlsEngine({ fileId, audio, mbit, light: true });
      e.onLevel((h) => (hlsHeight = h));
      hlsEngine = e;
      return e;
    }
    const e = createNativeEngine({ fileId, audio, hevc: hevcOk });
    nativeEngine = e;
    return e;
  }

  // Идёт смена потока (качество, дорожка, переход): остановка прежнего не считается паузой зрителя.
  let switching = false;

  /** Начать (или перезапустить после смены дорожки/качества) с места `at`. */
  function begin(at: number) {
    if (!info || !video) return;
    switching = true;
    session?.destroy();
    nativeEngine = null;
    hlsEngine = null;
    hlsHeight = null;
    offset = 0;
    current = 0;
    buffered = 0;
    mediaDuration = 0;
    localError = '';
    const caps = detectCaps(video);
    const chain: Step[] = src
      ? [{ mode: src.hls ? 'hls' : 'raw', quality: 'auto', why: 'видео по ссылке' }]
      : buildChain(info, caps, { quality, audio, slow });
    // Safari: у видео по ссылке уровни качества - из мастер-плейлиста.
    if (src?.hls && caps.nativeHls && src.linkId) {
      srcLevels = [];
      srcLevel = -1;
      fetch(`/api/links/${src.linkId}/stream/index.m3u8`)
        .then(async (r) => (r.ok ? nativeLevels(await r.text()) : []))
        .then((l) => (srcLevels = l.map((x) => ({ i: x.i, height: x.height, label: x.label }))))
        .catch(() => {});
    }
    const s = new PlaySession({ chain, makeEngine, report: onEvent });
    session = s;
    s.attach(video);
    s.start(at);
  }

  function destroyPlayback() {
    session?.destroy();
    session = null;
    nativeEngine = null;
    hlsEngine = null;
  }

  // ---------- источник: файл хоста или видео по ссылке ----------
  let resourceGeneration = 0;
  const sourceIdentity = $derived(src ? `link:${src.linkId ?? ''}:${src.url}` : `file:${fileId}`);
  let ready = false;
  let remote: { playing: boolean; position: number } | null = null;
  let remoteAt = 0;
  let nextPrepared = false;
  let ended = $state(false);
  let creditsSkipped = $state(false);
  let subId = $state('');
  let cues = $state<Cue[]>([]);
  let subLook = $state<SubLook>(savedLook());
  let elsewhere = $state<number | null>(null);
  let mismatchOff = $state(false);
  const nextEpisode = new NextEpisode(() => goNext());

  $effect(() => {
    void sourceIdentity;
    return untrack(() => {
      const resource = ++resourceGeneration;
      const id = fileId;
      const s = src;
      untrack(() => {
        srcLevel = -1;
        srcLevels = [];
        desiredPlaying = true;
      });
      ready = false;
      current = 0;
      offset = 0;
      mediaDuration = 0;
      buffered = 0;
      paused = true;
      info = null;
      localError = '';
      nextEpisode.cancel();
      ended = false;
      nextPrepared = false;
      creditsSkipped = false;
      subId = '';
      cues = [];
      elsewhere = null;
      destroyPlayback();
      if (s) {
        // Внешний источник: без запроса к серверу - и без отслеживания (begin читает и меняет состояние).
        untrack(() => {
          info = {
            play: { direct: !s.hls, remux: false, transcode: false, hevc: false, tenBit: false, duration: null, width: null, height: null, audio: [], subs: [] },
            titleId: 0, kind: 'movie', title: s.title, episode: '', prev: null, next: null, nextInfo: null, season: null, position: 0,
            markers: { intro: null, credits: null, manual: false }, trickplay: null, hls: false,
          };
          begin(s.start && s.start > 5 ? s.start : 0);
        });
        return destroyPlayback;
      }
      api<PlayInfo>(`/api/files/${id}/info`).then(
        async (i) => {
          if (resource !== resourceGeneration) return;
          // Замер сам укладывается в 2,5 с (медленно - считает по пришедшему); 3 с - на всякий случай.
          mbit = await Promise.race([speed, new Promise<null>((r) => setTimeout(() => r(null), 3000))]);
          if (resource !== resourceGeneration) return;
          info = i;
          audio = Math.max(0, i.play.audio.findIndex((a) => a.default));
          quality = slow ? 'auto' : startQuality(mbit, i.bitrate ?? null);
          if (quality === 'auto' && i.hls) say(`Интернет ${mbit} Мбит/с - включили автокачество`);
          const forced = i.play.subs.find((x) => x.forced && x.lang?.startsWith('ru'));
          if (forced) void pickSubs(forced.id);
          const start = sync && remote
            ? remotePosition(remote, remoteAt, performance.now())
            : i.position > 5 && (!i.play.duration || i.position < i.play.duration - 20) ? i.position : 0;
          begin(start);
        },
        () => {
          if (resource === resourceGeneration) localError = 'Не удалось открыть файл';
        },
      );
      return destroyPlayback;
    });
  });

  // ---------- события <video> ----------
  function onLoaded() {
    if (!video) return;
    try {
      video.volume = volume;
    } catch {
      /* iOS не даёт менять громкость */
    }
    video.playbackRate = rate;
    switching = false;
    // В комнате играет и стоит то, что у всех: сначала - состояние комнаты.
    if (sync) {
      ready = true;
      // Пока грузилось, у остальных видео шло дальше.
      if (remote) applyRemote({ ...remote, position: remotePosition(remote, remoteAt, performance.now()) });
      return;
    }
    if (desiredPlaying) video.play().catch(() => (paused = true));
    else video.pause();
  }

  function onTime() {
    if (!video) return;
    current = video.currentTime;
    offset = nativeEngine?.offset ?? 0;
    if (video.buffered.length) buffered = video.buffered.end(video.buffered.length - 1);
    session?.setPlace(time);
    progress.save();
    if (src && nearEnd(time, duration) && !nextPrepared) {
      nextPrepared = true;
      onNearEnd?.();
    }
  }

  function onEnded() {
    ended = true;
    onfinish?.();
    void progress.stop();
    if (canNext) {
      creditsSkipped = false;
      nextEpisode.start(Math.min(nextEpisode.left ?? 5, 5));
    }
  }

  // ---------- прогресс ----------
  const progress = createProgress({
    url: () => (src ? (src.linkId ? `/api/links/${src.linkId}/progress` : null) : `/api/progress/${fileId}`),
    position: () => time,
    duration: () => duration,
    net: () => {
      if (src) return undefined;
      const shownQuality = quality === 'orig' ? 'orig' : hlsHeight ? `${hlsHeight}p` : quality === 'auto' ? 'auto' : `${quality}p`;
      return { mbit, stalls: stallCount, quality: shownQuality };
    },
    onSave: noteOwnSave,
  });

  $effect(() => {
    const onHide = () => void progress.save(true);
    window.addEventListener('pagehide', onHide);
    document.addEventListener('visibilitychange', onHide);
    return () => {
      void progress.stop();
      forgetCatalog();
      extChanged();
      window.removeEventListener('pagehide', onHide);
      document.removeEventListener('visibilitychange', onHide);
    };
  });

  // Смотрели на другом устройстве: предложить перейти туда, где остановились. В комнате место задаёт комната.
  $effect(() => {
    if (!progressSync.stamp || sync) return;
    untrack(() => void checkElsewhere());
  });
  async function checkElsewhere() {
    const url = src ? (src.linkId ? `/api/links/${src.linkId}/resume` : null) : `/api/progress/${fileId}`;
    if (!url || !duration) return;
    try {
      const r = await api<{ position: number | null }>(url);
      elsewhere = elsewherePosition(r.position, time, duration) ?? elsewhere;
    } catch {
      /* не вышло - останемся на своём месте */
    }
  }

  // Видео по ссылке сильно отличается по длине от фильма (больше 12%) - предупредить.
  const mismatch = $derived(
    !!src && !mismatchOff && !!expect && duration > 60 && Math.abs(duration - expect) / expect > 0.12
      ? { duration, expect: expect!, canReport: !!onwrong }
      : null,
  );

  // ---------- заставка, титры, следующая серия ----------
  const canNext = $derived(!!info?.next || !!nextSource);
  const inIntro = $derived(inIntroAt(time, intro));
  const inCredits = $derived(
    inCreditsAt(time, { credits: info?.markers.credits ?? null, hasNext: !!info?.next, series: info?.kind === 'series', skipped: creditsSkipped, failed: !!error }),
  );

  $effect(() => {
    if (inCredits && !paused) {
      nextEpisode.start(untrack(() => nextEpisode.left) ?? 10);
      return () => nextEpisode.pause();
    }
    if (!inCredits && !untrack(() => video?.ended)) nextEpisode.cancel();
  });
  $effect(() => {
    if (ended && nextSource && !creditsSkipped) nextEpisode.start(5);
  });

  function goNext() {
    nextEpisode.cancel();
    if (nextSource) {
      onnext?.();
      return;
    }
    if (!info?.next) return;
    // В комнате следующая серия - у всех сразу: комната сменит файл.
    if (sync) sync.local({ fileId: info.next, playing: true, position: 0 });
    else goto(`/watch/${info.next}`, { replace: true });
  }

  async function back() {
    await Promise.race([progress.stop(), new Promise((resolve) => setTimeout(resolve, 300))]);
    if (sync?.back) return sync.back();
    if (history.length > 1) history.back();
    else goto(info?.titleId ? `/title/${info.titleId}` : '/discover');
  }

  // ---------- действия зрителя ----------
  function seek(t: number, report = true) {
    if (!video || !duration || !session) return;
    t = Math.max(0, Math.min(duration - 1, t));
    if (mode === 'remux') {
      const local = t - offset;
      // Внутри уже полученного куска - обычная перемотка, иначе новый поток с нужного места.
      if (local >= 0 && local <= buffered) video.currentTime = local;
      else {
        offset = t;
        current = 0;
        session.seek(t);
      }
    } else {
      session.seek(t);
    }
    if (report && sync) {
      const playing = remote ? remote.playing : !paused;
      remote = { playing, position: t };
      remoteAt = performance.now();
      sync.local({ playing, position: t });
    }
    poke();
  }

  function toggle() {
    if (!video) return;
    // В комнате «пауза или пуск» решает состояние комнаты, а не своё видео: у зрителя оно могло
    // стоять (браузер не дал запустить, ещё грузится) - и «пауза» запускала видео у всех.
    const play = sync && remote ? !remote.playing : video.paused;
    desiredPlaying = play;
    if (play) hlsEngine?.resumeBuffering();
    if (play) {
      if (sync) playRemote();
      else video.play().catch(() => {});
    } else video.pause();
    if (sync) {
      remote = { playing: play, position: time };
      remoteAt = performance.now();
      sync.local({ playing: play, position: time });
    }
    poke();
  }

  function setAudio(i: number) {
    panel = '';
    if (i === audio) return;
    audio = i;
    begin(time);
  }

  function setQuality(q: Quality) {
    panel = '';
    if (q === quality) return;
    quality = q;
    begin(time);
  }

  function pickLevel(i: number) {
    panel = '';
    srcLevel = i;
    if (hlsEngine) hlsEngine.setLevel(i);
    else if (video && src?.hls && src.linkId) {
      // Safari играет HLS сам: другой уровень - тот же поток с высотой в запросе, с того же места.
      const url = new URL(i === -1 ? src.url : `/api/links/${src.linkId}/stream/index.m3u8`, location.href);
      const height = srcLevels.find((l) => l.i === i)?.height;
      if (height) url.searchParams.set('height', String(height));
      const at = video.currentTime;
      video.src = url.href;
      video.addEventListener('loadedmetadata', () => (video!.currentTime = at), { once: true });
    }
    say(i === -1 ? 'Качество - автоматически' : `Качество ${srcLevels.find((l) => l.i === i)?.label ?? qualityTier(srcLevels.find((l) => l.i === i)?.height ?? 720)}p`);
  }

  async function pickSubs(id: string) {
    panel = '';
    subId = id;
    cues = [];
    if (!id) return;
    const loaded = await loadSubtitles(fileId, id);
    if (loaded === null) subId = '';
    else cues = loaded;
  }
  function setLook(p: Partial<SubLook>) {
    subLook = { ...subLook, ...p };
    saveLook(subLook);
  }
  const subtitle = $derived(subId ? cueAt(cues, time) : '');

  function setRate(r: number) {
    rate = Math.max(0.25, Math.min(3, Math.round(r * 100) / 100));
    if (video) video.playbackRate = rate;
    say(rate === 1 ? 'Обычная скорость' : `Скорость ×${String(rate).replace('.', ',')}`);
  }

  function setVolume(v: number) {
    volume = Math.max(0, Math.min(1, v));
    if (volume > 0) lastVolume = volume;
    muted = volume === 0;
    say(muted ? 'Звук выключен' : `Громкость ${Math.round(volume * 100)}%`);
    if (video) {
      video.volume = volume;
      video.muted = muted;
    }
    try {
      localStorage.setItem('fsfy-volume', String(volume));
    } catch {
      /* приватный режим */
    }
  }
  /** Из значка звука: -1 - вернуть прежнюю громкость, остальное - как есть. */
  function onVolume(v: number) {
    setVolume(v < 0 ? lastVolume : v);
  }

  async function pip() {
    try {
      if (document.pictureInPictureElement) await document.exitPictureInPicture();
      else await video?.requestPictureInPicture();
    } catch {
      say('Картинка в картинке недоступна');
    }
  }

  function fullscreen() {
    if (document.fullscreenElement) document.exitFullscreen();
    else if (box?.requestFullscreen) box.requestFullscreen().catch(() => {});
    else {
      // iPhone: полный экран только у самого <video>; пока оно не готово, браузер бросает InvalidStateError.
      try {
        (video as HTMLVideoElement & { webkitEnterFullscreen?: () => void } | undefined)?.webkitEnterFullscreen?.();
      } catch {
        say('Полный экран пока недоступен');
      }
    }
  }

  // Браузер не даёт запустить видео со звуком без нажатия (зашёл по ссылке) - без звука, и кнопка
  // «Включить звук»: иначе у этого зрителя стоит, у остальных идёт.
  function playRemote() {
    video?.play().catch((e: DOMException) => {
      if (!video || e?.name !== 'NotAllowedError') return void (paused = true);
      video.muted = true;
      mutedByBrowser = true;
      video.play().catch(() => (paused = true));
    });
  }
  function unmute() {
    if (!video) return;
    video.muted = false;
    mutedByBrowser = false;
  }

  // ---------- совместный просмотр ----------
  // Команды комнаты в комнату не отправляем (seek(…, false)); свои нажатия уходят всегда.
  function applyRemote(s: { playing: boolean; position: number }) {
    remote = s;
    remoteAt = performance.now();
    desiredPlaying = s.playing;
    if (!video || !ready || !duration) return;
    // Большой разрыв - перемотка (у перепаковки это новый поток - не дёргаем по мелочи).
    if (needsSeek(time, s.position, mode, video.seeking)) seek(s.position, false);
    if (s.playing) {
      hlsEngine?.resumeBuffering();
      if (video.paused) playRemote();
    }
    if (!s.playing && !video.paused) video.pause();
  }
  $effect(() => {
    if (!sync) return;
    return untrack(() => sync.listen((s) => applyRemote(s)));
  });

  async function watchTogether() {
    if (roomBusy || sync || (!fileId && !src?.linkId)) return;
    roomBusy = true;
    try {
      const room = await api<{ id: string }>('/api/rooms', { method: 'POST', json: { ...(src?.linkId ? { linkId: src.linkId } : { fileId }), position: time } });
      video?.pause();
      if (document.fullscreenElement) await document.exitFullscreen().catch(() => {});
      await goto(`/room/${room.id}`);
    } catch {
      say('Не удалось начать совместный просмотр');
    } finally {
      roomBusy = false;
    }
  }

  // ---------- серии ----------
  let episodeFiles = $state<FileItem[]>([]);
  let episodeSeason = $state(1);
  let episodesBusy = $state(false);
  async function openEpisodes() {
    if (panel === 'episodes') {
      panel = '';
      return;
    }
    panel = 'episodes';
    episodeSeason = info?.season ?? 1;
    if (!info?.titleId || episodeFiles.length) return;
    episodesBusy = true;
    try {
      const detail = await api<TitleDetail>(`/api/titles/${info.titleId}`);
      episodeFiles = detail.files;
    } catch {
      say('Не удалось загрузить серии');
    } finally {
      episodesBusy = false;
    }
  }
  function pickEpisode(id: number) {
    void progress.stop();
    panel = '';
    if (id === fileId) return;
    if (sync) sync.local({ playing: true, position: 0, fileId: id });
    else goto(`/watch/${id}`, { replace: true });
  }

  function onPanel(name: string) {
    if (name === 'episodes') return void openEpisodes();
    panel = panel === name ? '' : name;
  }

  // ---------- управление: показ и скрытие, жесты, клавиши ----------
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  function poke() {
    shown = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      if (!panel && !dragging) shown = false;
    }, 4000);
  }

  // Касания: двойное - ±10 с у левого/правого края (как в приложениях), одно - показать или спрятать
  // управление. Мышь: щелчок - пауза, двойной - во весь экран.
  let lastTap = { at: 0, x: 0 };
  let tapTimer: ReturnType<typeof setTimeout> | undefined;
  function tap(e: PointerEvent) {
    if (e.pointerType === 'mouse') return toggle();
    const w = (e.currentTarget as HTMLElement).clientWidth;
    const side = e.clientX < w / 3 ? 'left' : e.clientX > (w * 2) / 3 ? 'right' : null;
    const now = performance.now();
    if (side && now - lastTap.at < 300 && Math.abs(e.clientX - lastTap.x) < 80) {
      clearTimeout(tapTimer);
      seek(time + (side === 'right' ? 10 : -10));
      jump = { side, n: jump?.side === side ? jump.n + 10 : 10, key: now };
      setTimeout(() => jump?.key === now && (jump = null), 650);
      lastTap = { at: now, x: e.clientX };
      return;
    }
    lastTap = { at: now, x: e.clientX };
    clearTimeout(tapTimer);
    tapTimer = setTimeout(() => {
      if (shown && !paused) {
        clearTimeout(hideTimer);
        shown = false;
      } else poke();
    }, side ? 260 : 0);
  }

  function key(e: KeyboardEvent) {
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.target instanceof HTMLElement && e.target.closest('input, textarea, [contenteditable="true"]')) {
      // Фокус остался в поле (окно ссылки закрылось, чат комнаты): Esc - убрать фокус; поле не
      // в плеере (от прошлой страницы) - Esc работает как обычно.
      if (e.code !== 'Escape') return;
      e.target.blur();
      if (box?.contains(e.target)) return;
    }
    const action = keyAction(e, { pipOk, canNext, hasSubs: !!info?.play.subs.length, inIntro: inIntro && !!intro, duration });
    if (!action) return;
    switch (action.type) {
      case 'swallow':
        e.preventDefault();
        return;
      case 'toggle':
        say(paused ? 'Воспроизведение' : 'Пауза');
        toggle();
        break;
      case 'seek':
        seek(time + action.delta);
        say(action.delta > 0 ? 'Вперёд на 10 секунд' : 'Назад на 10 секунд');
        break;
      case 'volume':
        setVolume(volume + action.delta);
        break;
      case 'fullscreen':
        fullscreen();
        break;
      case 'mute':
        setVolume(muted ? lastVolume : 0);
        break;
      case 'skipIntro':
        if (intro) seek(intro[1]);
        break;
      case 'next':
        goNext();
        break;
      case 'captions':
        void pickSubs(subId ? '' : (info?.play.subs[0]?.id ?? ''));
        break;
      case 'escape':
        // Esc не выводит окно из полноэкранного режима: закрывает меню, иначе - назад в сервис.
        if (panel) panel = '';
        else if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
        else void back();
        break;
      case 'rate':
        setRate(rate + action.delta);
        break;
      case 'pip':
        void pip();
        break;
      case 'keys':
        panel = panel === 'keys' ? '' : 'keys';
        break;
      case 'percent': {
        const target = (duration * action.tenth) / 10;
        seek(target);
        say(`Переход к ${clock(target)}`);
        break;
      }
    }
    poke();
    e.preventDefault();
  }

  $effect(() => {
    try {
      const saved = localStorage.getItem('fsfy-volume');
      const v = Number(saved);
      if (saved !== null && v >= 0 && v <= 1) {
        volume = v;
        muted = v === 0;
        if (v > 0) lastVolume = v;
      }
    } catch {
      /* нет хранилища */
    }
    const native = (window as Window & { __NOVA_DESKTOP__?: boolean }).__NOVA_DESKTOP__;
    const nativeFullscreen = (enabled: boolean) => {
      if (native) void api('/api/host/fullscreen', { method: 'POST', json: { enabled } }).catch(() => {});
    };
    const fs = () => {
      full = !!document.fullscreenElement;
      if (full) {
        (box?.querySelector(':focus') as HTMLElement | null)?.blur();
        poke();
      }
      nativeFullscreen(full);
    };
    document.addEventListener('fullscreenchange', fs);
    return () => {
      document.removeEventListener('fullscreenchange', fs);
      if (full) nativeFullscreen(false);
    };
  });

  $effect(() => {
    const title = src?.title || info?.title;
    if (!title || !('mediaSession' in navigator)) return;
    navigator.mediaSession.metadata = new MediaMetadata({
      title,
      artist: 'Nova',
      artwork: [{ src: new URL('/media-artwork.png', location.href).href, sizes: '512x512', type: 'image/png' }],
    });
    return () => {
      navigator.mediaSession.metadata = null;
    };
  });
</script>

<svelte:window onkeydown={key} onpointermove={poke} />

<div class="player" class:idle={!shown} class:with-sidebar={sidebar} bind:this={box} style:--top-h="{topH}px" style:--bottom-h="{bottomH}px" onpointermove={poke} role="application" aria-label="Видеоплеер">
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    bind:this={video}
    playsinline
    preload="auto"
    onloadedmetadata={onLoaded}
    ondurationchange={() => (mediaDuration = video && isFinite(video.duration) ? video.duration : 0)}
    ontimeupdate={onTime}
    onprogress={onTime}
    onplay={() => {
      paused = false;
      desiredPlaying = true;
      hlsEngine?.resumeBuffering();
      poke();
    }}
    onpause={() => {
      paused = true;
      if (!switching && session?.state !== 'loading') desiredPlaying = false;
      hlsEngine?.pauseBuffering();
      poke();
      void progress.save(true);
    }}
    onended={onEnded}
    onpointerup={tap}
    ondblclick={(e) => (e as MouseEvent).detail && matchMedia('(pointer: fine)').matches && fullscreen()}
  ></video>

  <Overlay
    {shown}
    {jump}
    {subtitle}
    {subLook}
    spinner={waiting}
    {toast}
    {mismatch}
    {elsewhere}
    {mutedByBrowser}
    {error}
    roomError={!!sync}
    failedFile={src ? 0 : fileId}
    failedLink={!!src}
    skipTo={inIntro && intro ? intro[1] : null}
    next={(inCredits || (nextEpisode.left !== null && !inCredits)) && canNext
      ? { title: nextSource?.title ?? info?.nextInfo?.label ?? '', still: info?.nextInfo?.still ?? null, countdown: nextEpisode.left, inCredits }
      : null}
    onback={back}
    onretry={() => {
      localError = '';
      if (session) session.retry();
      else if (info) begin(time);
    }}
    {onfail}
    {onwrong}
    ondismissmismatch={() => {
      mismatchOff = true;
    }}
    ongoelsewhere={() => {
      if (elsewhere === null) return;
      seek(elsewhere);
      elsewhere = null;
    }}
    onstay={() => (elsewhere = null)}
    onunmute={unmute}
    onskipintro={(to) => seek(to)}
    onnext={goNext}
    onstaycredits={() => {
      creditsSkipped = true;
      nextEpisode.cancel();
    }}
  />

  <!-- Совместный просмотр: чат и реакции поверх видео (видны и во весь экран). -->
  {@render children?.()}

  <Controls
    title={info?.title ?? ''}
    episode={info?.episode ?? ''}
    {shown}
    {paused}
    busy={waiting || !!error}
    {panel}
    {time}
    {duration}
    {bufferedPct}
    {intro}
    trickplay={info?.trickplay ?? null}
    {fileId}
    {volume}
    {muted}
    {qualityLabel}
    {rate}
    showEpisodes={info?.kind === 'series' && !src}
    showSubs={!!info?.play.subs.length}
    subsOn={!!subId}
    showQuality={!!info?.hls || (!!src && srcLevels.length > 1)}
    {canNext}
    canTogether={!sync && (!!fileId || !!src?.linkId)}
    {roomBusy}
    {pipOk}
    {full}
    onback={back}
    ontogether={watchTogether}
    ontoggle={toggle}
    onskip={(d) => seek(time + d)}
    onseek={seek}
    ondrag={(d) => (dragging = d)}
    onvolume={onVolume}
    onpanel={onPanel}
    onnext={goNext}
    onpip={pip}
    onfullscreen={fullscreen}
    onheight={(t, b) => {
      topH = t;
      bottomH = b;
    }}
  />

  {#if panel && info}
    <Panels
      {panel}
      {info}
      linked={!!src}
      {fileId}
      {episodeFiles}
      bind:episodeSeason
      {episodesBusy}
      {qualityLabel}
      {rate}
      {alts}
      linkId={src?.linkId ?? null}
      canReportWrong={!!onwrong}
      {audio}
      {subId}
      {subLook}
      {quality}
      {origOk}
      {ladder}
      {srcLevels}
      {srcLevel}
      onpanel={(n) => (panel = n)}
      onepisode={pickEpisode}
      onalt={(id) => {
        panel = '';
        if (id !== src?.linkId) onalt?.(id);
      }}
      onwrong={() => {
        panel = '';
        onwrong?.();
      }}
      onaudio={setAudio}
      onsubs={pickSubs}
      onlook={setLook}
      onrate={setRate}
      onquality={setQuality}
      onlevel={pickLevel}
    />
  {/if}
</div>

<style>
  .player {
    position: fixed;
    inset: 0;
    background: #000;
    color: #fff;
    overflow: hidden;
    -webkit-user-select: none;
    user-select: none;
  }
  @media (min-width: 801px) {
    .player.with-sidebar video {
      width: calc(100% - 360px);
    }
    .player.with-sidebar :global(.top),
    .player.with-sidebar :global(.bottom) {
      right: 360px;
    }
  }
  .player.idle {
    cursor: none;
  }
  video {
    width: 100%;
    height: 100%;
    object-fit: contain;
    background: #000;
  }
</style>
