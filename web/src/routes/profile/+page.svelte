<script lang="ts">
  import { CHARACTERS } from '$lib/ui/characters';
  import EmailSettings from '$lib/account/EmailSettings.svelte';
  import Security from '$lib/account/Security.svelte';
  import PasswordInput from '$lib/ui/PasswordInput.svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { api, del, errorText, patch, post, put } from '$lib/api';
  import { loadMe, session } from '$lib/session.svelte';
  import DecorEditor from '$lib/profile/DecorEditor.svelte';
  import { motionOn, setMotion } from '$lib/motion';
  import Avatar from '$lib/ui/Avatar.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Availability from '$lib/account/Availability.svelte';
  import { toast } from '$lib/ui/toast.svelte';

  // Мой аккаунт: у каждого один - имя, фото или рисованный аватар, оформление страницы, пароль.
  const me = $derived(session.me);
  const profile = $derived(me?.user.profile);

  let name = $state('');
  let login = $state('');
  let old = $state('');
  let fresh = $state('');
  let err = $state('');
  let uploading = $state(false);
  let tab = $state<'style' | 'account' | 'playback'>('style');
  let motion = $state(motionOn());
  let trailers = $state((() => { try { return localStorage.getItem('nova-trailers') !== 'off'; } catch { return true; } })());
  $effect(()=>{const requested=page.url.searchParams.get('tab');if(requested==='account'||requested==='style'||requested==='playback')tab=requested;});

  $effect(() => {
    name = session.me?.user.name ?? '';
    login = session.me?.user.login ?? '';
  });

  async function saveName(e: SubmitEvent) {
    e.preventDefault();
    err = '';
    try {
      await patch('/api/me', { name });
      await loadMe();
      toast('Имя сохранено');
    } catch (x) {
      err = errorText(x);
    }
  }

  async function saveLogin(e: SubmitEvent) {
    e.preventDefault();
    err = '';
    try {
      await patch('/api/me', { login });
      await loadMe();
      toast('Логин сохранён');
    } catch (x) {
      err = errorText(x);
    }
  }

  async function setAvatar(i: number) {
    try {
      await patch('/api/me', { avatar: i });
      await loadMe();
    } catch (e) { toast(errorText(e)); }
  }

  // Своё фото: сервер обрежет до квадрата 256 px.
  async function pick(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const f = input.files?.[0];
    input.value = '';
    if (!f || !profile) return;
    if (f.size > 15 * 1024 * 1024) {
      err = 'Картинка больше 15 МБ';
      return;
    }
    uploading = true;
    err = '';
    try {
      await api(`/api/profiles/${profile.id}/photo`, { method: 'PUT', body: f });
      await loadMe();
    } catch (x) {
      err = errorText(x);
    } finally {
      uploading = false;
    }
  }

  async function dropPhoto() {
    if (!profile) return;
    await del(`/api/profiles/${profile.id}/photo`).catch(() => {});
    await loadMe();
  }

  async function savePassword(e: SubmitEvent) {
    e.preventDefault();
    err = '';
    try {
      await put('/api/me/password', { old: old || null, new: fresh });
      old = fresh = '';
      await loadMe();
      toast(session.me?.window ? `Пароль задан. С телефона входите с логином «${session.me.user.login}»` : 'Пароль изменён');
    } catch (x) {
      err = errorText(x);
    }
  }

  async function logout() {
    await post('/api/auth/logout').catch(() => {});
    session.me = null;
    goto('/login');
  }
</script>

<div class="rw-page">
  {#if me && profile}
    <header class="rw-head who-head">
      <div class="who">
        <div class="pic">
          <Avatar avatar={profile.avatar} photo={profile.photo} size={96} />
        </div>
        <div class="name">
          <h1>{me.user.name}</h1>
          <p>@{me.user.login}</p>
        </div>
      </div>
      <div class="row">
        <label class="btn sm" class:busy={uploading}>
          <Icon name="image" size={15} />
          {uploading ? 'Загружаем…' : profile.photo ? 'Другое фото' : 'Загрузить фото'}
          <input type="file" accept="image/jpeg,image/png,image/webp" onchange={pick} hidden />
        </label>
        {#if profile.photo}<button class="btn sm ghost" onclick={dropPhoto}>Убрать фото</button>{/if}
        <a class="btn sm ghost" href="/people/{profile.id}"><Icon name="user" size={15} /> Моя страница</a>
      </div>
    </header>

    <nav class="profile-tabs" aria-label="Настройки профиля">
      <button class:active={tab === 'style'} onclick={() => tab = 'style'}>Оформление</button>
      <button class:active={tab === 'account'} onclick={() => tab = 'account'}>Аккаунт и безопасность</button>
      <button class:active={tab === 'playback'} onclick={() => tab = 'playback'}>Интерфейс</button>
    </nav>
    <section class="rw-plate" aria-label="Профиль">
    {#if tab === 'style'}<DecorEditor profileId={profile.id} />{/if}

    {#if tab === 'style' && !profile.photo}
      <section class="rw-block pane">
        <h2 class="rw-h2">Аватар</h2>
        <p class="muted small">При регистрации Nova выбирает вам случайного персонажа. Найдите своего — или загрузите фото.</p>
        <div class="avatars" role="radiogroup" aria-label="Аватар">
          {#each CHARACTERS as character, i (character.name)}
            <button type="button" role="radio" aria-checked={profile.avatar === i} aria-label={character.name} title={character.name} class:on={profile.avatar === i} onclick={() => setAvatar(i)}>
              <Avatar avatar={i} size={52} /><span>{character.name}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if tab === 'playback'}
    <section class="rw-block pane">
      <h2 class="rw-h2">Анимации</h2>
      <label class="switch">
        <input type="checkbox" bind:checked={motion} onchange={() => setMotion(motion)} />
        <span>Плавные переходы, всплывающие ряды, плывущие постеры. Выключите, если компьютер слабый.</span>
      </label>
    </section>
    <section class="rw-block pane">
      <h2 class="rw-h2">Трейлеры в герое</h2>
      <label class="switch">
        <input type="checkbox" bind:checked={trailers} onchange={() => { try { localStorage.setItem('nova-trailers', trailers ? 'on' : 'off'); } catch { /* приватный режим */ } }} />
        <span>Трейлер сам играет без звука в герое главной и в ленте «Что смотреть». Выключите, чтобы экономить трафик.</span>
      </label>
    </section>
    <section class="rw-block pane">
      <h2 class="rw-h2">Экскурсия по Nova</h2>
      <p class="muted">Короткий показ, где что лежит: разделы, Обзор, подборки, совместный просмотр, плеер и телевизор.</p>
      <button class="btn" onclick={() => window.dispatchEvent(new Event('nova-tour'))}><Icon name="play" size={15} /> Пройти экскурсию</button>
    </section>

    {/if}
    {#if tab === 'account'}
    <div class="account-grid">
    <form class="rw-block pane" onsubmit={saveName}>
      <h2 class="rw-h2">Имя</h2>
      <label class="field"><span>Как вас видят другие</span><input class="input" bind:value={name} maxlength="60" /></label>
      <Availability value={name} kind="name" original={me.user.name} />
      <button class="btn">Сохранить</button>
    </form>

    <form class="rw-block pane" onsubmit={saveLogin}>
      <h2 class="rw-h2">Логин</h2>
      <label class="field"><span>Для входа по паролю с других устройств</span><input class="input" bind:value={login} maxlength="32" autocomplete="username" /></label>
      <Availability value={login} kind="login" original={me.user.login} />
      <p class="muted small">3-32 символа: буквы, цифры, «_», «-», «.».</p>
      <button class="btn">Сохранить</button>
    </form>

    <form class="rw-block pane" onsubmit={savePassword}>
      <h2 class="rw-h2">{me.hasPassword ? 'Сменить пароль' : 'Пароль для входа с других устройств'}</h2>
      {#if !me.hasPassword}
        <p class="muted small">Сейчас вы входите только из приложения на этом компьютере. Задайте пароль, чтобы управлять сайтом с телефона.</p>
      {:else}
        <div class="field"><PasswordInput caption="Текущий пароль" bind:value={old} autocomplete="current-password" /></div>
      {/if}
      <div class="field"><PasswordInput caption="Новый пароль" bind:value={fresh} minlength={6} autocomplete="new-password" required /></div>
      <button class="btn">Сохранить пароль</button>
    </form>
    <EmailSettings />
    <Security />
    </div>
    {/if}
    {#if err}<p class="error">{err}</p>{/if}
    {#if !me.window}
      <div class="rw-block"><button class="btn danger" onclick={logout}><Icon name="logout" size={16} /> Выйти</button></div>
    {/if}
    </section>
  {/if}
</div>

<style>
  .who-head {
    align-items: center;
    flex-wrap: wrap;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 20px;
    min-width: 0;
  }
  .pic {
    flex: none;
    border-radius: 50%;
    overflow: hidden;
  }
  .name {
    min-width: 0;
  }
  .name h1 {
    overflow-wrap: anywhere;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .busy {
    opacity: 0.6;
    pointer-events: none;
  }
  .pane {
    display: grid;
    gap: 14px;
  }
  .pane .btn {
    justify-self: start;
  }
  .avatars {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(78px, 1fr));
    gap: 10px;
  }
  .avatars button {
    display: grid;
    place-items: center;
    gap:8px;
    padding: 12px 4px;
    border: 2px solid transparent;
    border-radius: 16px;
    background: var(--surface);
    color:var(--text);
    cursor: pointer;
    transition: transform 0.15s;
  }
  .avatars button:hover {
    transform: scale(1.08);
  }
  .avatars .on {
    border-color: var(--text);
  }
  .avatars span { font-size:11px;line-height:1.3; }
  .switch {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-size: 14px;
    color: var(--muted);
    cursor: pointer;
  }
  .switch input {
    margin-top: 3px;
    width: 18px;
    height: 18px;
    accent-color: #fff;
    flex: none;
  }
  .small {
    font-size: 13px;
    margin: 0;
  }
  @media (max-width: 480px) {
    .who {
      flex-direction: column;
      text-align: center;
    }
    .who-head,
    .row {
      justify-content: center;
    }
  }

  /* Вкладки - капсулы над плитой, выбранная - белая (как чипы остальных экранов). */
  .profile-tabs {
    display: flex;
    gap: 8px;
    margin-bottom: calc(20 * var(--rw));
    overflow-x: auto;
  }
  .profile-tabs button {
    flex: none;
    border: 1px solid var(--line-strong);
    border-radius: 999px;
    padding: 10px 20px;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
  }
  .profile-tabs button.active {
    background: #fff;
    border-color: #fff;
    color: #0a0a0a;
  }
  .account-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }</style>
