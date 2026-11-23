<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon, { isIconName } from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import DeleteAccount from '#lib/features/account/DeleteAccount.svelte';
	import EmailSettings from '#lib/features/account/EmailSettings.svelte';
	import PasswordSettings from '#lib/features/account/PasswordSettings.svelte';
	import McpSetup from '#lib/features/integrations/McpSetup.svelte';
	import { i18n, locales, t, type Locale } from '#lib/i18n/index.svelte.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { learnersApi } from '#lib/services/api/learners.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { requireAuth } from '#lib/state/require-auth.svelte.ts';
	import type { Project } from '#lib/types/api.ts';

	requireAuth();

	let displayName = $state('');
	let saving = $state(false);
	let message = $state<{ tone: 'success' | 'danger'; text: string } | null>(null);
	let languageMessage = $state<{ tone: 'success' | 'danger'; text: string } | null>(null);
	let project = $state<Project | null>(null);
	let projectLoaded = $state(false);

	$effect(() => {
		if (auth.user && !displayName) displayName = auth.user.display_name;
	});

	$effect(() => {
		if (!auth.user) return;
		void i18n.locale; // theme names are translated
		learnersApi
			.project()
			.then((result) => (project = result))
			.catch(() => (project = null))
			.finally(() => (projectLoaded = true));
	});

	function errorText(error: unknown): string {
		return error instanceof ApiError ? (Object.values(error.fieldErrors())[0] ?? error.message) : t('settings.save_error');
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		message = null;
		try {
			auth.user = await authApi.updateProfile({ display_name: displayName });
			message = { tone: 'success', text: t('settings.saved') };
		} catch (error) {
			message = { tone: 'danger', text: errorText(error) };
		} finally {
			saving = false;
		}
	}

	async function changeLanguage(locale: Locale) {
		languageMessage = null;
		try {
			await auth.setLocale(locale);
			languageMessage = { tone: 'success', text: t('settings.language_saved') };
		} catch (error) {
			languageMessage = { tone: 'danger', text: errorText(error) };
		}
	}
</script>

<svelte:head><title>{t('settings.title')} · {t('app.name')}</title></svelte:head>

<div class="mx-auto max-w-3xl px-6 py-8">
	<PageHeader title={t('settings.title')} />

	{#if auth.user}
		<section class="mt-6 grid gap-6 border-b border-line pb-8 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium">{t('settings.account')}</h2>
				<p class="mt-1 text-[13px] text-fg-muted">{t('settings.account_body')}</p>
			</div>
			<form class="flex max-w-sm flex-col gap-4" onsubmit={save}>
				<TextField label={t('settings.display_name')} bind:value={displayName} maxlength={80} />
				{#if message}<Notice tone={message.tone}>{message.text}</Notice>{/if}
				<div><Button type="submit" variant="primary" loading={saving}>{t('settings.save')}</Button></div>
			</form>
		</section>

		<section class="mt-8 grid gap-6 border-b border-line pb-8 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium">{t('settings.email')}</h2>
				<p class="mt-1 text-[13px] text-fg-muted">{t('settings.email_body')}</p>
			</div>
			<EmailSettings user={auth.user} />
		</section>

		<section class="mt-8 grid gap-6 border-b border-line pb-8 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium">{t('settings.password')}</h2>
				<p class="mt-1 text-[13px] text-fg-muted">{t('settings.password_body')}</p>
			</div>
			<PasswordSettings />
		</section>

		<section class="mt-8 grid gap-6 border-b border-line pb-8 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium">{t('settings.language')}</h2>
				<p class="mt-1 text-[13px] text-fg-muted">{t('settings.language_body')}</p>
			</div>
			<div class="flex max-w-sm flex-col gap-3">
				<div class="grid grid-cols-2 gap-2" role="radiogroup" aria-label={t('settings.language')}>
					{#each locales as option (option.code)}
						<label
							class="flex cursor-pointer items-center gap-2 rounded-md border px-3 py-2 text-[13px] {i18n.locale === option.code
								? 'border-accent bg-accent-soft'
								: 'border-line-strong bg-surface hover:border-fg-subtle'}"
						>
							<input
								type="radio"
								name="locale"
								value={option.code}
								checked={i18n.locale === option.code}
								onchange={() => changeLanguage(option.code)}
								class="accent-[var(--accent)]"
							/>
							<span>{option.flag} {option.label}</span>
						</label>
					{/each}
				</div>
				{#if languageMessage}<Notice tone={languageMessage.tone}>{languageMessage.text}</Notice>{/if}
			</div>
		</section>

		<section class="mt-8 grid gap-6 border-b border-line pb-8 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium">{t('settings.project')}</h2>
				<p class="mt-1 text-[13px] text-fg-muted">{t('settings.project_body')}</p>
			</div>
			{#if project}
				<div class="flex max-w-sm items-center gap-3 rounded-lg border border-line bg-surface px-4 py-3">
					<span class="grid size-8 shrink-0 place-items-center rounded-md bg-accent-soft text-accent">
						<Icon name={isIconName(project.theme?.icon) ? (project.theme?.icon ?? 'globe') : 'globe'} />
					</span>
					<div>
						<p class="text-[14px] font-medium">{project.theme?.name ?? project.theme_id}</p>
						<p class="text-[12px] text-fg-muted">{t(`project.stack.${project.stack}`)}</p>
					</div>
				</div>
			{:else if projectLoaded}
				<p class="text-[13px] text-fg-muted">{t('settings.project_none')}</p>
			{/if}
		</section>

		<section class="mt-8 grid gap-6 border-b border-line pb-8 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium">{t('settings.tools')}</h2>
				<p class="mt-1 text-[13px] text-fg-muted">{t('settings.tools_body')}</p>
			</div>
			<McpSetup />
		</section>

		<section class="mt-8 grid gap-6 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium text-danger">{t('settings.delete')}</h2>
				<p class="mt-1 text-[13px] text-fg-muted">{t('settings.delete_body')}</p>
			</div>
			<DeleteAccount />
		</section>
	{/if}
</div>
