<script lang="ts">
	import Icon from '#lib/components/ui/Icon.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { auth } from '#lib/state/auth.svelte.ts';

	let status = $state<'idle' | 'sending' | 'sent'>('idle');
	let error = $state<string | null>(null);

	async function resend() {
		status = 'sending';
		error = null;
		try {
			await authApi.resendVerification();
			status = 'sent';
		} catch (caught) {
			status = 'idle';
			if (caught instanceof ApiError && caught.code === 'already_verified') await auth.load();
			else error = caught instanceof ApiError ? caught.message : t('common.error');
		}
	}
</script>

{#if auth.user && !auth.user.email_verified}
	<div class="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-line bg-warning-soft px-4 py-2 text-[13px] text-warning" role="status">
		<Icon name="mail" />
		<span>{t('verify.banner', { email: auth.user.email })}</span>
		{#if status === 'sent'}
			<span class="font-medium">{t('verify.resent')}</span>
		{:else}
			<button type="button" class="font-medium underline-offset-2 hover:underline disabled:opacity-60" disabled={status === 'sending'} onclick={resend}>
				{t('verify.resend')}
			</button>
		{/if}
		{#if error}<span>{error}</span>{/if}
	</div>
{/if}
