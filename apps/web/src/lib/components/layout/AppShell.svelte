<script lang="ts">
	import type { Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Icon, { type IconName } from '#lib/components/ui/Icon.svelte';
	import Logo from '#lib/components/ui/Logo.svelte';
	import { auth } from '#lib/state/auth.svelte.ts';

	let { children }: { children: Snippet } = $props();

	const navigation: { href: string; label: string; icon: IconName; requiresAuth?: boolean; requiresAdmin?: boolean }[] = [
		{ href: '/', label: 'Home', icon: 'home', requiresAuth: true },
		{ href: '/courses', label: 'Courses', icon: 'book' },
		{ href: '/admin/courses', label: 'Course studio', icon: 'edit', requiresAdmin: true },
		{ href: '/admin/traffic', label: 'Traffic', icon: 'chart', requiresAdmin: true },
		{ href: '/settings', label: 'Settings', icon: 'settings', requiresAuth: true }
	];

	function isActive(href: string): boolean {
		const path = page.url.pathname;
		return href === '/' ? path === '/' : path.startsWith(href);
	}

	async function logout() {
		await auth.logout();
		await goto('/login');
	}
</script>

<div class="flex min-h-screen">
	<aside class="sticky top-0 hidden h-screen w-56 shrink-0 flex-col border-r border-line bg-surface md:flex">
		<a href="/" class="flex h-12 items-center gap-2 px-4">
			<Logo size={20} />
			<span class="text-[13px] font-semibold tracking-tight">Learning Platform</span>
		</a>
		<nav class="flex flex-col gap-px px-2 pt-2" aria-label="Main">
			{#each navigation.filter((item) => (!item.requiresAuth || auth.user) && (!item.requiresAdmin || auth.isSuperAdmin)) as item (item.href)}
				<a
					href={item.href}
					aria-current={isActive(item.href) ? 'page' : undefined}
					class="flex h-8 items-center gap-2.5 rounded-md px-2 text-[13px] text-fg-muted hover:bg-surface-hover hover:text-fg aria-[current=page]:bg-surface-muted aria-[current=page]:font-medium aria-[current=page]:text-fg"
				>
					<Icon name={item.icon} />
					{item.label}
				</a>
			{/each}
		</nav>
		<div class="mt-auto border-t border-line p-2">
			{#if auth.user}
				<div class="flex items-center gap-2 rounded-md px-2 py-1.5">
					<span class="grid size-6 place-items-center rounded-full bg-surface-muted text-[11px] font-medium text-fg-muted">
						{auth.user.display_name.slice(0, 1).toUpperCase()}
					</span>
					<div class="min-w-0 flex-1">
						<p class="truncate text-[13px] font-medium">{auth.user.display_name}</p>
						<p class="truncate text-[11px] text-fg-subtle">{auth.user.email}</p>
					</div>
					<button type="button" onclick={logout} class="rounded p-1 text-fg-subtle hover:bg-surface-hover hover:text-fg" title="Sign out" aria-label="Sign out">
						<Icon name="log-out" size={14} />
					</button>
				</div>
			{:else if auth.ready}
				<a href="/login" class="flex h-8 items-center gap-2 rounded-md px-2 text-[13px] text-fg-muted hover:bg-surface-hover hover:text-fg">
					<Icon name="arrow-right" /> Sign in
				</a>
			{/if}
		</div>
	</aside>

	<div class="flex min-w-0 flex-1 flex-col">
		<header class="flex h-12 items-center gap-4 border-b border-line bg-surface px-4 md:hidden">
			<a href="/" class="flex items-center gap-2 text-[13px] font-semibold"><Logo size={18} />Learning Platform</a>
			<nav class="flex gap-3 text-[13px] text-fg-muted">
				<a href="/courses">Courses</a>
				{#if auth.isSuperAdmin}<a href="/admin/courses">Studio</a><a href="/admin/traffic">Traffic</a>{/if}
				{#if auth.user}<a href="/settings">Settings</a>{:else}<a href="/login">Sign in</a>{/if}
			</nav>
		</header>
		<main class="flex-1">{@render children()}</main>
		<footer class="border-t border-line px-6 py-3 text-[12px] text-fg-subtle">
			<a href="/privacy" class="hover:text-fg hover:underline">Privacy</a>
		</footer>
	</div>
</div>
