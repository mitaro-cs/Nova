<script lang="ts">
  import PasswordInput from '$lib/ui/PasswordInput.svelte';
  import { goto } from '$app/navigation';
  import { api, errorText, post } from '$lib/api';
  import { toast } from '$lib/ui/toast.svelte';
  import { loadMe } from '$lib/session.svelte';
  import Availability from '$lib/account/Availability.svelte';
  import { LEGAL_VERSION } from '$lib/legal';
  import type { Site } from '$lib/types';

  let { invite = '', askInvite = true }: { invite?: string; askInvite?: boolean } = $props();

  let code = $state('');
  let name = $state('');
  let login = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);
  let email=$state(''),emailEnabled=$state(false);
  // Согласие с политикой и правилами: без галочки аккаунт не создаётся ни кнопкой, ни запросом (сервер проверяет сам).
  let agreed=$state(false);
  let legalVersion=$state(LEGAL_VERSION);
  // Места беты: «Осталось N из 100», а когда мест нет - форма не нужна.
  let seats=$state<Site['seats']>(null);
  $effect(()=>{api<Pick<Site,'seats'>&{emailRecovery?:boolean;legalVersion?:number}>('/api/site').then(s=>{emailEnabled=!!s.emailRecovery;if(s.legalVersion)legalVersion=s.legalVersion;seats=s.seats??null;},()=>{});});

  $effect(() => {
    code = invite;
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!agreed) {
      error = 'Чтобы создать аккаунт, согласитесь с политикой конфиденциальности и правилами';
      return;
    }
    busy = true;
    error = '';
    try {
      await post('/api/auth/register', { invite: code || null, name, login, password, consent: legalVersion });
      if(emailEnabled&&email.trim()) {try{await post('/api/me/email',{email,password});}catch{toast('Аккаунт создан, но письмо не отправлено. Повторите подтверждение в настройках аккаунта.');}}
      const me = await loadMe();
      if (me?.user.profile?.id) sessionStorage.setItem(`nova-onboarding:${me.user.profile.id}`, '1');
      goto(email.trim()?'/profile?tab=account':'/', { replace: true });
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }
</script>

{#if seats && seats.left <= 0}
  <div class="seats full" role="status"><b>Все {seats.total} мест беты заняты</b><span>Nova пока не принимает новых зрителей. Если у вас уже есть аккаунт - войдите.</span></div>
  <a class="btn primary block" href="/login">Войти</a>
{:else}
{#if seats}<p class="seats" role="status"><b>Бета Nova - бесплатно для первых {seats.total}.</b> Осталось мест: {seats.left}</p>{/if}
<form onsubmit={submit}>
  {#if askInvite}
    <label class="field">
      <span>Код приглашения</span>
      <input class="input" bind:value={code} autocapitalize="characters" autocomplete="off" required />
    </label>
  {/if}
  <label class="field"><span>Как вас зовут</span><input class="input" bind:value={name} autocomplete="name" required maxlength="60" /><Availability value={name} kind="name" /></label>
  <label class="field">
    <span>Логин (латиница, цифры)</span><input class="input" bind:value={login} autocomplete="username" required minlength="3" maxlength="32" />
    <Availability value={login} kind="login" />
  </label>
  <div class="field"><PasswordInput caption="Пароль (от 6 символов)" bind:value={password} autocomplete="new-password" required minlength={6} />
  </div>
  {#if emailEnabled}<label class="field"><span>Почта для восстановления (необязательно)</span><input class="input" type="email" bind:value={email} autocomplete="email"/><small class="muted">После регистрации отправим ссылку для подтверждения.</small></label>{/if}
  <label class="agree"><input type="checkbox" bind:checked={agreed} /><span>Я прочитал(а) <a href="/legal/privacy" target="_blank" rel="noopener">политику конфиденциальности</a> и <a href="/legal/terms" target="_blank" rel="noopener">правила использования</a> и согласен(на) с ними</span></label>
  {#if error}<p class="error">{error}</p>{/if}
  <button class="btn primary block" disabled={busy || !agreed}>{busy ? 'Создаём…' : 'Создать аккаунт'}</button>
  <a class="btn ghost block" href="/login">У меня уже есть аккаунт</a>
</form>
{/if}

<style>
  .seats { margin: 0 0 14px; padding: 12px 14px; border-radius: 14px; background: #ffffff0f; font-size: 13.5px; line-height: 1.45; color: var(--muted); }
  .seats b { color: var(--text); }
  .seats.full { display: grid; gap: 4px; margin-bottom: 14px; }
  .agree { display: flex; align-items: flex-start; gap: 10px; margin: 6px 0 4px; font-size: 13px; line-height: 1.45; color: var(--muted); cursor: pointer; }
  .agree input { flex: none; width: 18px; height: 18px; margin: 1px 0 0; accent-color: #fff; }
  .agree a { color: var(--text); text-decoration: underline; text-underline-offset: 3px; }
</style>
