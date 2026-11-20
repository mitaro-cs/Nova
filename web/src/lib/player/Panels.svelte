<script lang="ts">
  import { clock } from '$lib/fmt';
  import { localized } from '$lib/localized.svelte';
  import type { FileItem, PlayInfo } from '$lib/types';
  import { artW } from '$lib/ui/art';
  import Icon from '$lib/ui/Icon.svelte';
  import { heightLabel } from './codecs';
  import type { Level } from './engine/hls';
  import type { Quality } from './plan';
  import { levelLabel } from './quality-label';
  import { SUB_LOOKS, SUB_SIZES, type SubLook } from './tracks';

  // Меню над органами управления: серии, настройки, источник, озвучка, субтитры, скорость, клавиши, качество.
  let {
    panel,
    info,
    linked = false,
    fileId = 0,
    // серии
    episodeFiles = [],
    episodeSeason = $bindable(1),
    episodesBusy = false,
    // настройки и источник
    qualityLabel = '',
    rate,
    volume=1,
    onvolume,
    onnext,
    alts = [],
    linkId = null,
    canReportWrong = false,
    // озвучка и субтитры
    audio = 0,
    subId = '',
    subLook,
    // качество: файл хоста
    quality = 'orig',
    origOk = false,
    ladder = [],
    // качество: видео по ссылке
    srcLevels = [],
    srcLevel = -1,
    onpanel,
    onepisode,
    onalt,
    onwrong,
    onaudio,
    onsubs,
    onlook,
    onrate,
    onquality,
    onlevel,
  }: {
    panel: string;
    info: PlayInfo;
    linked?: boolean;
    fileId?: number;
    episodeFiles?: FileItem[];
    episodeSeason?: number;
    episodesBusy?: boolean;
    qualityLabel?: string;
    rate: number;
    volume?:number;
    onvolume?:(value:number)=>void;
    onnext?:()=>void;
    alts?: { id: number; label: string }[];
    linkId?: number | null;
    canReportWrong?: boolean;
    audio?: number;
    subId?: string;
    subLook: SubLook;
    quality?: Quality;
    origOk?: boolean;
    ladder?: number[];
    srcLevels?: Level[];
    srcLevel?: number;
    onpanel: (name: string) => void;
    onepisode: (id: number) => void;
    onalt?: (id: number) => void;
    onwrong?: () => void;
    onaudio: (i: number) => void;
    onsubs: (id: string) => void;
    onlook: (p: Partial<SubLook>) => void;
    onrate: (r: number) => void;
    onquality: (q: Quality) => void;
    onlevel: (i: number) => void;
  } = $props();

  const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];
  const speedText = (r: number) => (r === 1 ? 'Обычная' : `×${String(r).replace('.', ',')}`);
  const seasons = $derived([...new Set(episodeFiles.map((f) => f.season ?? 1))]);
  const sourceLevelLabel = (level: Level) => levelLabel(level, srcLevels);
</script>

<div class="panel" role="menu">
  {#if panel === 'episodes'}
    <h4>Сезоны и серии</h4>
    <div class="episode-seasons">
      {#each seasons as season (season)}<button role="menuitemradio" aria-checked={episodeSeason === season} onclick={() => (episodeSeason = season)}>{season} сезон</button>{/each}
    </div>
    {#if episodesBusy}<p class="hint">Загружаем серии…</p>{/if}
    {#each episodeFiles.filter((f) => (f.season ?? 1) === episodeSeason) as file (file.id)}
      <button class="episode-option" class:on={file.id === fileId} role="menuitem" onclick={() => onepisode(file.id)}>
        {#if file.still}<img src={artW(file.still, 342)} alt="" loading="lazy" />{/if}
        <span>
          <b>{file.episode}. {localized(file.name, 'Серия')}</b>
          {#if localized(file.overview)}<small>{localized(file.overview)}</small>{/if}
          <small>{file.duration ? `${Math.round(file.duration / 60)} мин` : ''}{file.progress?.position ? ` · ${clock(file.progress.position)}` : ''}</small>
        </span>
      </button>
    {/each}
  {:else if panel === 'settings'}
    <h4>Настройки просмотра</h4>
    {#if onnext}<button role="menuitem" onclick={onnext}><Icon name="next" size={18} /> Следующая серия</button>{/if}
    {#if info.titleId && info.kind === 'series'}<button role="menuitem" onclick={() => onpanel('episodes')}><Icon name="tv" size={18} /> Сезоны и серии</button>{/if}
    <button role="menuitem" onclick={() => onpanel('quality')}><Icon name="sliders" size={18} /> Качество <small>{qualityLabel}</small></button>
    <button role="menuitem" onclick={() => onpanel('speed')}><Icon name="play" size={18} /> Скорость <small>{speedText(rate)}</small></button>
    {#if onvolume}<label class="boost">Громкость · {Math.round(volume*100)}%<input aria-label="Громкость до 200%" type="range" min="0" max="2" step="0.02" value={volume} oninput={e=>onvolume?.(Number(e.currentTarget.value))} /><span>0% <b>100%</b> 200%</span></label>{/if}
    {#if info.play.subs.length}<button role="menuitem" onclick={() => onpanel('subs')}><Icon name="captions" size={18} /> Субтитры и размер</button>{/if}
    {#if info.play.audio.length > 1}<button role="menuitem" onclick={() => onpanel('audio')}><Icon name="audio" size={18} /> Озвучка</button>{/if}
    {#if linked && alts.length > 1}<button role="menuitem" onclick={() => onpanel('sources')}><Icon name="link" size={18} /> Источник видео</button>{/if}
    {#if linked && canReportWrong}<button role="menuitem" onclick={() => onwrong?.()}><Icon name="flag" size={18} /> Это не тот фильм</button>{/if}
    <button role="menuitem" onclick={() => onpanel('keys')}><Icon name="info" size={18} /> Горячие клавиши</button>
  {:else if panel === 'sources'}
    <h4>Источник видео</h4>
    {#each alts as a (a.id)}
      <button role="menuitemradio" aria-checked={a.id === linkId} class:on={a.id === linkId} onclick={() => onalt?.(a.id)}>
        {a.label}
      </button>
    {/each}
  {:else if panel === 'audio'}
    <h4>Озвучка</h4>
    {#each info.play.audio as a (a.index)}
      <button role="menuitemradio" aria-checked={audio === a.index} class:on={audio === a.index} onclick={() => onaudio(a.index)}>
        {a.label}
      </button>
    {/each}
  {:else if panel === 'subs'}
    <h4>Субтитры</h4>
    <button role="menuitemradio" aria-checked={!subId} class:on={!subId} onclick={() => onsubs('')}>Выключены</button>
    {#each info.play.subs as s (s.id)}
      <button role="menuitemradio" aria-checked={subId === s.id} class:on={subId === s.id} onclick={() => onsubs(s.id)}>
        {s.label}
      </button>
    {/each}
    {#if !linked}
    <h4 class="sep">Вид</h4>
    <div class="looks" role="radiogroup" aria-label="Вид субтитров">
      {#each SUB_LOOKS as l (l.id)}
        <button role="radio" aria-checked={subLook.look === l.id} class="look {l.id}" class:on={subLook.look === l.id} onclick={() => onlook({ look: l.id })}>
          <span>Аа</span>{l.label}
        </button>
      {/each}
    </div>
    <div class="seg" role="radiogroup" aria-label="Размер субтитров">
      {#each SUB_SIZES as z (z.id)}
        <button role="radio" aria-checked={subLook.size === z.id} class:on={subLook.size === z.id} onclick={() => onlook({ size: z.id })}>{z.label}</button>
      {/each}
    </div>
    <div class="seg" role="radiogroup" aria-label="Положение субтитров">
      <button role="radio" aria-checked={!subLook.high} class:on={!subLook.high} onclick={() => onlook({ high: false })}>Внизу</button>
      <button role="radio" aria-checked={subLook.high} class:on={subLook.high} onclick={() => onlook({ high: true })}>Выше</button>
    </div>
    {/if}
  {:else if panel === 'speed'}
    <h4>Скорость</h4>
    {#each SPEEDS as r (r)}
      <button role="menuitemradio" aria-checked={rate === r} class:on={rate === r} onclick={() => onrate(r)}>
        {speedText(r)}
      </button>
    {/each}
  {:else if panel === 'keys'}
    <h4>Горячие клавиши</h4>
    <dl class="keys">
      <dt>Пробел, K</dt><dd>пауза</dd>
      <dt>← →, J L</dt><dd>±10 секунд</dd>
      <dt>1…9</dt><dd>к 10…90 % фильма</dd>
      <dt>↑ ↓</dt><dd>громкость</dd>
      <dt>M</dt><dd>без звука</dd>
      <dt>F</dt><dd>во весь экран</dd>
      <dt>Shift + &lt; &gt;</dt><dd>скорость</dd>
      <dt>C</dt><dd>субтитры</dd>
      <dt>S</dt><dd>пропустить заставку</dd>
      <dt>N</dt><dd>следующая серия</dd>
      <dt>P</dt><dd>картинка в картинке</dd>
    </dl>
    <p class="hint">На телефоне: два касания у края - ±10 секунд, одно - показать управление.</p>
  {:else if panel === 'quality' && linked}
    <h4>Качество</h4>
    <button role="menuitemradio" aria-checked={srcLevel === -1} class:on={srcLevel === -1} onclick={() => onlevel(-1)}>
      Авто
    </button>
    {#each srcLevels as l (l.i)}
      <button role="menuitemradio" aria-checked={srcLevel === l.i} class:on={srcLevel === l.i} onclick={() => onlevel(l.i)}>
        {sourceLevelLabel(l)}
      </button>
    {/each}
  {:else if panel === 'quality'}
    <h4>Качество</h4>
    {#if origOk}
      <button role="menuitemradio" aria-checked={quality === 'orig'} class:on={quality === 'orig'} onclick={() => onquality('orig')}>
        Оригинал
      </button>
    {/if}
    <button role="menuitemradio" aria-checked={quality === 'auto'} class:on={quality === 'auto'} onclick={() => onquality('auto')}>
      Авто
    </button>
    {#each ladder as l (l)}
      <button role="menuitemradio" aria-checked={quality === l} class:on={quality === l} onclick={() => onquality(l)}>
        {heightLabel(l)}
      </button>
    {/each}
  {/if}
</div>

<style>
  .boost{display:grid;gap:10px;padding:12px;font-size:13px}.boost input{width:100%;accent-color:white}.boost span{display:flex;justify-content:space-between;color:#aaa;font-size:11px}.boost b{font-weight:400}
  .panel {
    position: absolute;
    right: clamp(14px, 3vw, 36px);
    bottom: 116px;
    display: grid;
    gap: 2px;
    width: min(360px, calc(100% - 28px));
    max-height: min(70%, 560px);
    overflow-y: auto;
    padding: 10px;
    border-radius: 18px;
    background: rgba(24, 24, 24, 0.96);
    z-index: 5;
  }
  h4 {
    margin: 4px 8px 6px;
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.65);
  }
  h4.sep {
    margin-top: 12px;
  }
  .panel button {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 0;
    border-radius: 12px;
    background: none;
    color: #fff;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .panel button:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .panel button.on {
    background: rgba(255, 255, 255, 0.16);
  }
  .panel button small {
    margin-left: auto;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
  }
  .hint {
    margin: 6px 8px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
  }
  .keys {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 14px;
    margin: 4px 8px;
    font-size: 13px;
  }
  .keys dt {
    font-weight: 600;
  }
  .keys dd {
    margin: 0;
    color: rgba(255, 255, 255, 0.7);
  }
  .episode-seasons {
    display: flex;
    gap: 4px;
    overflow-x: auto;
  }
  .episode-seasons button {
    flex: none;
  }
  .panel .episode-option {
    align-items: flex-start;
  }
  .episode-option img {
    width: 105px;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    border-radius: 6px;
  }
  .episode-option span {
    display: grid;
    gap: 5px;
    min-width: 0;
  }
  .episode-option small {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    margin: 0;
    font-size: 11px;
    color: #bbb;
  }
  .looks,
  .seg {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px 6px;
  }
  .look span {
    padding: 0 6px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.5);
    font-weight: 700;
  }
  .look.yellow span {
    color: #ffe14d;
  }
  .seg button {
    flex: 1;
    justify-content: center;
  }
</style>
