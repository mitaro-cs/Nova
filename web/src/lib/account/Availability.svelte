<script lang="ts">
  import { api } from '$lib/api';
  let {value,kind,original=''}:{value:string;kind:'login'|'name';original?:string}=$props();
  let taken=$state(false),checking=$state(false);
  $effect(()=>{
    const text=value.trim();taken=false;checking=false;
    if(text.toLocaleLowerCase('ru')===original.trim().toLocaleLowerCase('ru')||text.length<(kind==='login'?3:1))return;
    let alive=true;const timer=setTimeout(()=>{checking=true;void api<{loginTaken:boolean;nameTaken:boolean}>(`/api/auth/availability?${kind}=${encodeURIComponent(text)}`).then(r=>{if(alive)taken=kind==='login'?r.loginTaken:r.nameTaken;},()=>{}).finally(()=>{if(alive)checking=false;});},300);
    return()=>{alive=false;clearTimeout(timer);};
  });
</script>
{#if checking}<small class="muted" role="status">Проверяем…</small>{:else if taken}<small class="error" role="alert">{kind==='login'?'Этот логин уже занят':'Это имя уже используется'}</small>{/if}
