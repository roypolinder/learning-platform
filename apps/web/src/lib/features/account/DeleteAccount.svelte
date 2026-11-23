<script lang="ts">
	import { goto } from '$app/navigation';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import { i18n, t } from '#lib/i18n/index.svelte.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { accountErrorText } from './errors.ts';

	let confirming = $state(false);
	let password = $state('');
	let busy = $state(false);
	let error = $state<string | null>(null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		error = null;
		try {
			// The API only ever deletes the account of the session making the request.
			await authApi.deleteAccount(password);
			auth.user = null;
			await goto(i18n.locale === 'nl' ? '/nl/' : '/');
		} catch (caught) {
			error = accountErrorText(caught);
		} finally {
			busy = false;
		}
	}
</script>

{#if confirming}
	<form class="flex max-w-sm flex-col gap-3 rounded-lg border border-danger/40 p-4" onsubmit={submit}>
		<Notice tone="danger">{t('settings.delete_warning')}</Notice>
		<TextField label={t('auth.password')} type="password" bind:value={password} autocomplete="current-password" required />
		{#if error}<Notice tone="danger">{error}</Notice>{/if}
		<div class="flex flex-wrap gap-2">
			<Button type="submit" variant="danger" loading={busy} disabled={!password}>{t('settings.delete_confirm')}</Button>
			<Button variant="ghost" onclick={() => ((confirming = false), (password = ''), (error = null))}>{t('common.cancel')}</Button>
		</div>
	</form>
{:else}
	<div><Button variant="danger" onclick={() => (confirming = true)}>{t('settings.delete_start')}</Button></div>
{/if}
