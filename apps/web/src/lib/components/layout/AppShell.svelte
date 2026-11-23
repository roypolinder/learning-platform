<script lang="ts">
	import type { Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Icon, { type IconName } from '#lib/components/ui/Icon.svelte';
	import Logo from '#lib/components/ui/Logo.svelte';
	import LanguagePicker from '#lib/components/ui/LanguagePicker.svelte';
	import VerifyEmailBanner from '#lib/features/authentication/VerifyEmailBanner.svelte';
	import { i18n, t, type MessageKey } from '#lib/i18n/index.svelte.ts';
	import { auth } from '#lib/state/auth.svelte.ts';

	let { children }: { children: Snippet } = $props();

	const navigation: { href: string; label: MessageKey; icon: IconName; requiresAuth?: boolean; requiresAdmin?: boolean }[] = [
		{ href: '/dashboard', label: 'nav.dashboard', icon: 'home', requiresAuth: true },
		{ href: '/courses', label: 'nav.courses', icon: 'book' },
		{ href: '/admin/courses', label: 'nav.studio', icon: 'edit', requiresAdmin: true },
		{ href: '/admin/users', label: 'nav.users', icon: 'users', requiresAdmin: true },
		{ href: '/admin/logs', label: 'nav.logs', icon: 'terminal', requiresAdmin: true },
		{ href: '/admin/traffic', label: 'nav.traffic', icon: 'chart', requiresAdmin: true },
		{ href: '/settings', label: 'nav.settings', icon: 'settings', requiresAuth: true }
	];

	// Signed-in learners start from their dashboard; visitors from the landing page.
	let home = $derived(auth.user ? '/dashboard' : i18n.locale === 'nl' ? '/nl/' : '/');

	function isActive(href: string): boolean {
		return page.url.pathname.startsWith(href);
	}

	async function logout() {
		await auth.logout();
		await goto('/login');
	}
</script>

<div class="flex min-h-screen">
	<aside class="sticky top-0 hidden h-screen w-56 shrink-0 flex-col border-r border-line bg-surface md:flex">
		<a href={home} class="flex h-12 items-center gap-2 px-4">
			<Logo size={20} />
			<span class="text-[13px] font-semibold tracking-tight">{t('app.name')}</span>
		</a>
		<nav class="flex flex-col gap-px px-2 pt-2" aria-label={t('nav.main')}>
			{#each navigation.filter((item) => (!item.requiresAuth || auth.user) && (!item.requiresAdmin || auth.isSuperAdmin)) as item (item.href)}
				<a
					href={item.href}
					aria-current={isActive(item.href) ? 'page' : undefined}
					class="flex h-8 items-center gap-2.5 rounded-md px-2 text-[13px] text-fg-muted hover:bg-surface-hover hover:text-fg aria-[current=page]:bg-surface-muted aria-[current=page]:font-medium aria-[current=page]:text-fg"
				>
					<Icon name={item.icon} />
					{t(item.label)}
				</a>
			{/each}
		</nav>
		<div class="mt-auto px-3 pb-2">
			<LanguagePicker value={i18n.locale} onchange={(locale) => void auth.setLocale(locale)} class="w-full [&_select]:w-full" />
		</div>
		<div class="border-t border-line p-2">
			{#if auth.user}
				<div class="flex items-center gap-2 rounded-md px-2 py-1.5">
					<span class="grid size-6 place-items-center rounded-full bg-surface-muted text-[11px] font-medium text-fg-muted">
						{auth.user.display_name.slice(0, 1).toUpperCase()}
					</span>
					<div class="min-w-0 flex-1">
						<p class="truncate text-[13px] font-medium">{auth.user.display_name}</p>
						<p class="truncate text-[11px] text-fg-subtle">{auth.user.email}</p>
					</div>
					<button type="button" onclick={logout} class="rounded p-1 text-fg-subtle hover:bg-surface-hover hover:text-fg" title={t('nav.sign_out')} aria-label={t('nav.sign_out')}>
						<Icon name="log-out" size={14} />
					</button>
				</div>
			{:else if auth.ready}
				<a href="/login" class="flex h-8 items-center gap-2 rounded-md px-2 text-[13px] text-fg-muted hover:bg-surface-hover hover:text-fg">
					<Icon name="arrow-right" /> {t('nav.sign_in')}
				</a>
			{/if}
		</div>
	</aside>

	<div class="flex min-w-0 flex-1 flex-col">
		<header class="flex h-12 items-center gap-4 border-b border-line bg-surface px-4 md:hidden">
			<a href={home} class="flex items-center gap-2 text-[13px] font-semibold"><Logo size={18} /><span class="sr-only sm:not-sr-only">{t('app.name')}</span></a>
			<nav class="flex flex-wrap gap-3 text-[13px] text-fg-muted">
				{#if auth.user}<a href="/dashboard">{t('nav.dashboard')}</a>{/if}
				<a href="/courses">{t('nav.courses')}</a>
				{#if auth.isSuperAdmin}<a href="/admin/courses">{t('nav.studio_short')}</a><a href="/admin/users">{t('nav.users')}</a><a href="/admin/logs">{t('nav.logs')}</a><a href="/admin/traffic">{t('nav.traffic')}</a>{/if}
				{#if auth.user}<a href="/settings">{t('nav.settings')}</a>{:else}<a href="/login">{t('nav.sign_in')}</a>{/if}
			</nav>
		</header>
		<VerifyEmailBanner />
		<main class="flex-1">{@render children()}</main>
		<footer class="border-t border-line px-6 py-3 text-[12px] text-fg-subtle">
			<a href="/privacy" class="hover:text-fg hover:underline">{t('common.privacy')}</a>
		</footer>
	</div>
</div>
