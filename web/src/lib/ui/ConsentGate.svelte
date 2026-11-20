<script lang="ts">
  import { goto } from '$app/navigation';
  import { errorText, post } from '$lib/api';
  import { LEGAL_DATE } from '$lib/legal';
  import { loadMe, session } from '$lib/session.svelte';
  import Logo from '$lib/ui/Logo.svelte';

  // Закрывает весь сервис, пока человек не согласился с текущей редакцией документов: прежние аккаунты
  // (созданные до появления политики) и все, кто не принимал новую редакцию. Документы открываются в новой
  // вкладке, чтобы согласие не терялось; принять можно только со включённой галочкой.
  const version = $derived(session.me?.legal?.version ?? 1);
  const first = $derived((session.me?.legal?.accepted ?? 0) === 0);
  let agreed = $state(false);
  let busy = $state(false);
  let error = $state('');

  async function accept() {
    if (!agreed || busy) return;
    busy = true;
    error = '';
    try {
      await post('/api/me/consent', { version });
      await loadMe();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function leave() {
    await post('/api/auth/logout').catch(() => {});
    session.me = null;
    goto('/login');
  }
</script>

<main class="gate" aria-labelledby="gate-title">
  <div class="card">
    <a href="/" aria-label="Nova" tabindex="-1"><Logo /></a>
    <h1 id="gate-title">{first ? 'Согласие с политикой конфиденциальности' : 'Мы обновили документы'}</h1>
    <p class="lead">
      {first
        ? 'Чтобы продолжить пользоваться Nova, прочитайте и примите политику конфиденциальности и правила использования.'
        : 'Изменилась политика конфиденциальности или правила. Прочитайте новую редакцию и примите её, чтобы продолжить.'}
    </p>
    <ul>
      <li>Сервер хранит ваше имя, историю просмотра, оценки, отзывы, друзей и сообщения.</li>
      <li>Видеть их могут другие зрители в пределах ваших настроек и хост сервера.</li>
      <li>Данные не продаются и не используются для рекламы. Удалить аккаунт можно через хоста.</li>
    </ul>
    <p class="docs">
      <a href="/legal/privacy" target="_blank" rel="noopener">Политика конфиденциальности</a>
      <a href="/legal/terms" target="_blank" rel="noopener">Правила использования</a>
      <span>редакция от {LEGAL_DATE}</span>
    </p>
    <label class="ok">
      <input type="checkbox" bind:checked={agreed} />
      <span>Я прочитал(а) политику конфиденциальности и правила использования и согласен(на) с ними</span>
    </label>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="btns">
      <button class="btn primary" type="button" disabled={!agreed || busy} onclick={accept}>{busy ? 'Сохраняем…' : 'Согласен и продолжить'}</button>
      <button class="btn ghost" type="button" onclick={leave}>Выйти</button>
    </div>
  </div>
</main>

<style>
  .gate { display: grid; place-items: center; min-height: 100dvh; padding: 24px 16px; background: #000; }
  .card { width: min(560px, 100%); padding: clamp(22px, 4vw, 40px); border: 1px solid #ffffff1a; border-radius: 32px; background: #101012; }
  h1 { margin: 22px 0 0; font-size: clamp(24px, 4vw, 32px); font-weight: 800; letter-spacing: -0.03em; line-height: 1.15; }
  .lead { margin: 12px 0 0; color: #c4c4cc; line-height: 1.55; }
  ul { margin: 18px 0 0; padding-left: 20px; color: #d4d4d8; line-height: 1.6; font-size: 14px; }
  .docs { display: flex; flex-wrap: wrap; gap: 8px 18px; margin: 18px 0 0; font-size: 14px; }
  .docs a { text-decoration: underline; text-underline-offset: 3px; }
  .docs span { color: var(--muted); }
  .ok { display: flex; align-items: flex-start; gap: 12px; margin-top: 22px; padding: 14px 16px; border: 1px solid #ffffff24; border-radius: 18px; background: #17171a; font-size: 14px; line-height: 1.45; cursor: pointer; }
  .ok input { flex: none; width: 20px; height: 20px; margin: 1px 0 0; accent-color: #fff; }
  .btns { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 20px; }
  .btns .btn { min-height: 46px; padding-inline: 26px; border-radius: 999px; }
  .error { margin: 12px 0 0; }
</style>
