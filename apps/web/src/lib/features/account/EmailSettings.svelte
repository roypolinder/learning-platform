<script lang="ts">
	import Badge from '#lib/components/ui/Badge.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { User } from '#lib/types/api.ts';
	import { accountErrorText } from './errors.ts';

	let { user }: { user: User } = $props();

	let editing = $state(false);
	let email = $state('');
	let password = $state('');
	let busy = $state(false);
	let message = $state<{ tone: 'success' | 'danger'; text: string } | null>(null);

	async function run(action: () => Promise<void>) {
		busy = true;
		message = null;
		try {
			await action();
		} catch (error) {
			message = { tone: 'danger', text: accountErrorText(error) };
		} finally {
			busy = false;
		}
	}

	const resend = () =>
		run(async () => {
			await authApi.resendVerification();
			message = { tone: 'success', text: t('settings.verification_sent', { email: user.email }) };
		});

	function submit(event: SubmitEvent) {
		event.preventDefault();
		void run(async () => {
			auth.user = await authApi.changeEmail(email.trim(), password);
			message = { tone: 'success', text: t('settings.change_email_sent', { email: auth.user.pending_email ?? email.trim() }) };
			editing = false;
			email = '';
			password = '';
		});
	}

	const cancelChange = () =>
		run(async () => {
			auth.user = await authApi.cancelEmailChange();
		});
</script>

<div class="flex max-w-sm flex-col gap-3">
	<div class="flex flex-wrap items-center gap-2">
		<span class="text-[14px] font-medium">{user.email}</span>
		{#if user.email_verified}
			<Badge tone="success">{t('settings.verified')}</Badge>
		{:else}
			<Badge tone="warning">{t('settings.unverified')}</Badge>
		{/if}
	</div>

	{#if user.pending_email}
		<Notice tone="info">
			{t('settings.pending_email', { email: user.pending_email })}
			<button type="button" class="ml-1 font-medium text-fg underline-offset-2 hover:underline" disabled={busy} onclick={cancelChange}>
				{t('settings.cancel_change')}
			</button>
		</Notice>
	{/if}

	{#if message}<Notice tone={message.tone}>{message.text}</Notice>{/if}

	{#if editing}
		<form class="flex flex-col gap-3" onsubmit={submit}>
			<TextField label={t('settings.new_email')} type="email" bind:value={email} autocomplete="email" required />
			<TextField label={t('settings.current_password')} type="password" bind:value={password} autocomplete="current-password" required />
			<div class="flex gap-2">
				<Button type="submit" variant="primary" loading={busy}>{t('settings.change_email')}</Button>
				<Button variant="ghost" onclick={() => (editing = false)}>{t('common.cancel')}</Button>
			</div>
		</form>
	{:else}
		<div class="flex flex-wrap gap-2">
			<Button onclick={() => ((editing = true), (message = null))}>{t('settings.change_email')}</Button>
			{#if !user.email_verified}
				<Button variant="ghost" loading={busy} onclick={resend}>{t('settings.resend_verification')}</Button>
			{/if}
		</div>
	{/if}
</div>
