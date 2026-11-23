<script lang="ts">
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { accountErrorText } from './errors.ts';

	let current = $state('');
	let next = $state('');
	let busy = $state(false);
	let message = $state<{ tone: 'success' | 'danger'; text: string } | null>(null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		message = null;
		try {
			await authApi.changePassword(current, next);
			current = '';
			next = '';
			message = { tone: 'success', text: t('settings.password_changed') };
		} catch (error) {
			message = { tone: 'danger', text: accountErrorText(error) };
		} finally {
			busy = false;
		}
	}
</script>

<form class="flex max-w-sm flex-col gap-3" onsubmit={submit}>
	<TextField label={t('settings.current_password')} type="password" bind:value={current} autocomplete="current-password" required />
	<TextField
		label={t('auth.new_password')}
		type="password"
		bind:value={next}
		autocomplete="new-password"
		required
		minlength={10}
		hint={t('auth.password_hint')}
	/>
	{#if message}<Notice tone={message.tone}>{message.text}</Notice>{/if}
	<div><Button type="submit" loading={busy}>{t('settings.change_password')}</Button></div>
</form>
