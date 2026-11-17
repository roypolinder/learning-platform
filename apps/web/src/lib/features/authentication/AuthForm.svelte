<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import { ApiError } from '#lib/services/api/client.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { i18n, locales, t, type Locale } from '#lib/i18n/index.svelte.ts';
	import { auth } from '#lib/state/auth.svelte.ts';

	let { mode }: { mode: 'login' | 'register' } = $props();

	let email = $state('');
	let password = $state('');
	let displayName = $state('');
	// Defaults to the language the visitor is browsing in (English unless they chose otherwise).
	let locale = $state<Locale>(i18n.locale);
	let submitting = $state(false);
	let error = $state<string | null>(null);
	let fieldErrors = $state<Record<string, string>>({});

	function destination(): string {
		const next = page.url.searchParams.get('next');
		return next && next.startsWith('/') && !next.startsWith('//') ? next : '/dashboard';
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		submitting = true;
		error = null;
		fieldErrors = {};
		try {
			auth.set(
				mode === 'login'
					? await authApi.login({ email, password })
					: await authApi.register({ email, password, display_name: displayName, locale })
			);
			await goto(destination());
		} catch (caught) {
			if (caught instanceof ApiError) {
				fieldErrors = caught.fieldErrors();
				error = Object.keys(fieldErrors).length ? null : caught.message;
			} else {
				error = t('common.error');
			}
		} finally {
			submitting = false;
		}
	}
</script>

<form class="flex flex-col gap-4" onsubmit={submit} novalidate>
	{#if error}<Notice tone="danger">{error}</Notice>{/if}
	{#if mode === 'register'}
		<TextField label={t('auth.name')} bind:value={displayName} autocomplete="name" required error={fieldErrors.display_name} />
	{/if}
	<TextField label={t('auth.email')} type="email" bind:value={email} autocomplete="email" required error={fieldErrors.email} />
	<TextField
		label={t('auth.password')}
		type="password"
		bind:value={password}
		autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
		required
		error={fieldErrors.password}
		hint={mode === 'register' ? t('auth.password_hint') : undefined}
	/>
	{#if mode === 'register'}
		<fieldset class="flex flex-col gap-1.5">
			<legend class="mb-1.5 text-[13px] font-medium">{t('common.language')}</legend>
			<div class="grid grid-cols-2 gap-2">
				{#each locales as option (option.code)}
					<label
						class="flex cursor-pointer items-center gap-2 rounded-md border px-3 py-2 text-[13px] {locale === option.code
							? 'border-accent bg-accent-soft'
							: 'border-line-strong bg-surface hover:border-fg-subtle'}"
					>
						<input
							type="radio"
							name="locale"
							value={option.code}
							bind:group={locale}
							onchange={() => i18n.set(option.code)}
							class="accent-[var(--accent)]"
						/>
						<span>{option.flag} {option.label}</span>
					</label>
				{/each}
			</div>
			<p class="text-[12px] text-fg-subtle">{t('auth.language_hint')}</p>
		</fieldset>
	{/if}
	<Button type="submit" variant="primary" loading={submitting} class="mt-1 h-9">
		{t(mode === 'login' ? 'auth.sign_in' : 'auth.submit_register')}
	</Button>
</form>
