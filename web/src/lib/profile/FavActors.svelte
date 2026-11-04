<script lang="ts">
  import { errorText, patch } from '$lib/api';
  import { fadeIn } from '$lib/ui/fade';
  import { placeholder } from '$lib/ui/art';
  import Icon from '$lib/ui/Icon.svelte';
  import { toast } from '$lib/ui/toast.svelte';

  // Любимые актёры и актрисы: до пяти круглых фото с именами. Владелец жмёт «Изменить» - раскрывается редактор:
  // убрать, добавить из актёров просмотренного или вписать любое имя.
  type A = { name: string; photo: string | null };
  let {
    actors = $bindable(),
    suggest = [],
    editable = false,
  }: { actors: A[]; suggest?: string[]; editable?: boolean } = $props();

  const MAX = 5;
  let open = $state(false);
  let name = $state('');
  let busy = $state(false);

  const initials = (n: string) => n.split(/\s+/).slice(0, 2).map((w) => w[0]).join('');
  const more = $derived(suggest.filter((s) => !actors.some((a) => a.name.toLowerCase() === s.toLowerCase())).slice(0, 4));

  async function save(next: A[]) {
    if (busy) return;
    busy = true;
    const before = actors;
    actors = next;
    try {
      await patch('/api/me/profile', { actors: next.map((a) => a.name) });
    } catch (x) {
      actors = before;
      toast(errorText(x));
    } finally {
      busy = false;
    }
  }
  function add(n: string) {
    const v = n.trim();
    if (!v) return;
    if (actors.length >= MAX) return void toast(`Любимых актёров и актрис - не больше ${MAX}: сначала уберите кого-то`);
    if (actors.some((a) => a.name.toLowerCase() === v.toLowerCase())) return void toast('Он уже в списке');
    name = '';
    void save([...actors, { name: v, photo: null }]);
  }
  const remove = (n: string) => save(actors.filter((a) => a.name !== n));
</script>

<section class="fa" aria-labelledby="fa-h">
  <div class="sh">
    <h2 id="fa-h">Любимые актёры и актрисы</h2>
    {#if editable}<button type="button" class="edit" aria-expanded={open} onclick={() => (open = !open)}>{open ? 'Готово' : 'Изменить'}</button>{/if}
  </div>

  {#if editable}
    <div class="edwrap" class:open>
      <div class="inner">
        <div class="ed">
          <p class="hint">Любимых {actors.length} из {MAX}. Чтобы добавить другого, сначала уберите кого-то.</p>
          <div class="chips">
            {#each actors as a (a.name)}
              <span class="chip">{a.name}<button type="button" aria-label="Убрать {a.name}" onclick={() => remove(a.name)}><Icon name="close" size={12} /></button></span>
            {/each}
          </div>
          {#if more.length}
            <p class="hint">Из ваших просмотренных</p>
            <div class="chips">{#each more as s (s)}<button type="button" class="sug" onclick={() => add(s)}>+ {s}</button>{/each}</div>
          {/if}
          <form class="add" onsubmit={(e) => (e.preventDefault(), add(name))}>
            <input class="input" bind:value={name} maxlength="80" placeholder="Имя актёра или актрисы" aria-label="Имя актёра или актрисы" autocomplete="off" />
            <button class="btn primary" disabled={!name.trim() || busy}>Добавить</button>
          </form>
        </div>
      </div>
    </div>
  {/if}

  {#if actors.length}
    <ul class="act">
      {#each actors as a, i (a.name)}
        <li style:--i={i}>
          <a href="/person?name={encodeURIComponent(a.name)}&role=actor" title={a.name}>
            <span class="face" style:background={a.photo ? undefined : placeholder(a.name)}>
              {#if a.photo}<img use:fadeIn src={a.photo} alt="" loading="lazy" decoding="async" width="160" height="160" />{:else}<b>{initials(a.name)}</b>{/if}
            </span>
            <span class="n">{a.name}</span>
          </a>
        </li>
      {/each}
    </ul>
  {:else if !editable}
    <p class="muted">Пока никого.</p>
  {:else}
    <p class="muted">Добавьте до пяти любимых актёров и актрис - кнопка «Изменить».</p>
  {/if}
</section>

<style>
  .sh {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .edit {
    flex: none;
    padding: 4px 13px;
    border: 1px solid var(--line-strong, #ffffff40);
    border-radius: 999px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    transition:
      background 0.2s,
      color 0.2s;
  }
  .edit:hover {
    background: #ffffff14;
    color: var(--text);
  }
  .edwrap {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows 0.45s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .edwrap.open {
    grid-template-rows: 1fr;
  }
  .inner {
    overflow: hidden;
    min-height: 0;
  }
  .ed {
    margin-bottom: 14px;
    padding: 14px;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: #ffffff08;
  }
  .hint {
    margin: 0 0 8px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px 5px 13px;
    border-radius: 999px;
    background: #ffffff1f;
    font-size: 13px;
    animation: pop 0.45s cubic-bezier(0.2, 0.9, 0.3, 1.4) both;
  }
  .chip button {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: none;
    cursor: pointer;
    transition: background 0.2s;
  }
  .chip button:hover {
    background: #ffffff38;
  }
  .sug {
    padding: 5px 13px;
    border: 1px dashed var(--line-strong, #ffffff40);
    border-radius: 999px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    transition:
      color 0.2s,
      border-color 0.2s;
  }
  .sug:hover {
    border-color: #fff;
    color: var(--text);
  }
  .add {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }
  .add .input {
    flex: 1;
    min-width: 0;
  }
  .act {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 14px;
    margin: 0;
    padding: 0;
    list-style: none;
    text-align: center;
  }
  .act li {
    min-width: 0;
    animation: float 3.6s ease-in-out infinite alternate;
    animation-delay: calc(var(--i) * -0.8s);
    will-change: transform;
  }
  .act a {
    display: grid;
    justify-items: center;
    align-content: start;
    gap: 8px;
  }
  .face {
    display: grid;
    place-items: center;
    width: 100%;
    max-width: 84px;
    aspect-ratio: 1;
    border-radius: 50%;
    overflow: hidden;
    box-shadow: inset 0 0 0 1.5px #ffffff33;
    transition: transform 0.3s cubic-bezier(0.2, 0.9, 0.3, 1.4);
  }
  .act a:hover .face {
    transform: scale(1.1);
  }
  .face img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .face b {
    font-size: clamp(14px, 3.4vw, 22px);
  }
  .n {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }
  @keyframes float {
    to {
      transform: translate3d(0, -8px, 0);
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.3);
    }
  }
  @media (max-width: 900px) {
    .act {
      gap: 8px;
    }
    .n {
      font-size: 12px;
    }
  }
</style>
