<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import LanguagePicker from '#lib/components/ui/LanguagePicker.svelte';
	import Logo from '#lib/components/ui/Logo.svelte';
	import { i18n, preferredLocale, translate, type Locale, type MessageKey } from '#lib/i18n/index.svelte.ts';
	import { auth } from '#lib/state/auth.svelte.ts';

	/** The URL decides the language, so every language has its own indexable page. */
	let { locale }: { locale: Locale } = $props();

	const tr = (key: MessageKey, params?: Record<string, string | number>) => translate(locale, key, params);
	const homes: Record<Locale, string> = { en: '/', nl: '/nl/' };
	const siteUrl = (import.meta.env.VITE_SITE_URL ?? '').replace(/\/$/, '');

	// Mirrors courses/themes/*.yaml; this page is prerendered without the API.
	const themes = [
		'bike-rental',
		'bakery',
		'bookshop',
		'plant-shop',
		'recipes',
		'holiday-homes',
		'gym',
		'vintage-clothing',
		'animal-shelter',
		'concert-venue'
	];
	const phases: { title: MessageKey; courses: string[] }[] = [
		{ title: 'landing.phase_1', courses: ['project-setup', 'mcp-first-connection', 'agent-skill-basics', 'first-web-page'] },
		{
			title: 'landing.phase_2',
			courses: ['tool-calling-fundamentals', 'semantic-matching', 'vector-databases', 'multimodal-ai', 'rag-fundamentals']
		},
		{
			title: 'landing.phase_3',
			courses: ['agent-skills-advanced', 'build-mcp-server', 'ai-agents', 'knowledge-graphs', 'agentic-workflows']
		}
	];
	const faqs = [1, 2, 3, 4, 5] as const;

	let courseNumbers = new Map(phases.flatMap((phase) => phase.courses).map((id, index) => [id, index + 1]));

	async function switchLanguage(next: Locale) {
		await auth.setLocale(next).catch(() => i18n.set(next));
		if (next !== locale) await goto(homes[next]);
	}

	onMount(() => {
		// Send first-time visitors to the version in their language; an explicit choice wins.
		const preferred = preferredLocale();
		if (preferred !== locale) void goto(homes[preferred], { replaceState: true });
	});
</script>

<svelte:head>
	<title>{tr('landing.title')} · {tr('app.name')}</title>
	<meta name="description" content={tr('landing.lead')} />
	<meta property="og:title" content={tr('landing.headline')} />
	<meta property="og:description" content={tr('landing.lead')} />
	<meta property="og:type" content="website" />
	<meta property="og:locale" content={locale === 'nl' ? 'nl_NL' : 'en_GB'} />
	<link rel="canonical" href="{siteUrl}{homes[locale]}" />
	<link rel="alternate" hreflang="en" href="{siteUrl}/" />
	<link rel="alternate" hreflang="nl" href="{siteUrl}/nl/" />
	<link rel="alternate" hreflang="x-default" href="{siteUrl}/" />
</svelte:head>

<div class="min-h-screen bg-bg">
	<header class="sticky top-0 z-20 border-b border-line bg-bg/90 backdrop-blur" style="padding-top: env(safe-area-inset-top, 0px)">
		<div class="mx-auto flex h-14 max-w-4xl items-center gap-3 px-4 sm:px-6">
			<a href={homes[locale]} class="flex items-center gap-2">
				<Logo size={20} />
				<span class="text-[13px] font-semibold tracking-tight">{tr('app.name')}</span>
			</a>
			<nav class="ml-auto flex items-center gap-1 sm:gap-2" aria-label={tr('nav.main')}>
				<a href="/courses" class="hidden h-8 items-center rounded-md px-2.5 text-[13px] text-fg-muted hover:bg-surface-hover hover:text-fg sm:flex">
					{tr('nav.courses')}
				</a>
				<LanguagePicker value={locale} onchange={switchLanguage} />
				{#if auth.user}
					<Button variant="primary" size="sm" href="/dashboard">{tr('nav.dashboard')}</Button>
				{:else}
					<a href="/login" class="hidden h-8 items-center rounded-md px-2.5 text-[13px] text-fg-muted hover:bg-surface-hover hover:text-fg sm:flex">
						{tr('nav.sign_in')}
					</a>
					<Button variant="primary" size="sm" href="/register">{tr('nav.get_started')}</Button>
				{/if}
			</nav>
		</div>
	</header>

	<main class="mx-auto max-w-4xl px-4 sm:px-6">
		<section class="grid gap-10 pt-16 pb-14 md:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)] md:items-center md:pt-24">
			<div>
				<h1 class="text-[32px] leading-tight font-semibold tracking-tight text-balance sm:text-[38px]">{tr('landing.headline')}</h1>
				<p class="mt-4 max-w-lg text-[15px] leading-relaxed text-fg-muted">{tr('landing.lead')}</p>
				<div class="mt-7 flex flex-wrap gap-2">
					{#if auth.user}
						<Button variant="primary" href="/dashboard" class="h-9 px-4">{tr('landing.cta_dashboard')}</Button>
					{:else}
						<Button variant="primary" href="/register" class="h-9 px-4">{tr('landing.cta_primary')}</Button>
					{/if}
					<Button href="/courses" class="h-9 px-4">{tr('landing.cta_secondary')}</Button>
				</div>
			</div>

			<figure class="rounded-lg border border-line bg-surface p-3">
				<div class="flex items-center gap-2 rounded-md border border-line-strong px-3 py-2">
					<Icon name="search" size={14} class="text-fg-subtle" />
					<span class="text-[14px]">{tr('landing.example_query')}</span>
				</div>
				<ul class="mt-2 divide-y divide-line">
					{#each [['landing.example_result_1', 'landing.example_result_1_meta', '0.82'], ['landing.example_result_2', 'landing.example_result_2_meta', '0.71']] as const as [title, meta, score] (title)}
						<li class="flex items-baseline gap-3 px-1 py-2.5">
							<span class="min-w-0 flex-1">
								<span class="block text-[13px] font-medium">{tr(title)}</span>
								<span class="block text-[12px] text-fg-muted">{tr(meta)}</span>
							</span>
							<span class="font-mono text-[11px] text-fg-subtle">{score}</span>
						</li>
					{/each}
				</ul>
				<figcaption class="mt-1 px-1 text-[12px] text-fg-subtle">{tr('landing.example_caption')}</figcaption>
			</figure>
		</section>

		<section class="border-t border-line py-12">
			<h2 class="text-[15px] font-semibold">{tr('landing.intro_title')}</h2>
			<p class="mt-2 max-w-2xl text-[14px] leading-relaxed text-fg-muted">{tr('landing.intro')}</p>
			<h3 class="mt-8 text-[13px] font-medium text-fg-muted">{tr('landing.themes_title')}</h3>
			<ul class="mt-2 flex max-w-2xl flex-wrap gap-x-4 gap-y-1 text-[14px]">
				{#each themes as theme (theme)}
					<li>{tr(`theme.${theme}` as MessageKey)}</li>
				{/each}
			</ul>
		</section>

		<section class="border-t border-line py-12">
			<h2 class="text-[15px] font-semibold">{tr('landing.track_title')}</h2>
			<p class="mt-2 max-w-2xl text-[14px] text-fg-muted">{tr('landing.track_lead')}</p>
			<div class="mt-6 grid gap-8 md:grid-cols-3">
				{#each phases as phase (phase.title)}
					<div>
						<h3 class="text-[12px] font-medium tracking-wide text-fg-subtle uppercase">{tr(phase.title)}</h3>
						<ol class="mt-3 flex flex-col gap-2">
							{#each phase.courses as course (course)}
								<li>
									<a href="/courses/{course}" class="flex items-center gap-3 text-[14px] hover:text-accent">
										<span class="w-5 shrink-0 font-mono text-[12px] text-fg-subtle">{String(courseNumbers.get(course)).padStart(2, '0')}</span>
										{tr(`track.${course}` as MessageKey)}
									</a>
								</li>
							{/each}
						</ol>
					</div>
				{/each}
			</div>
		</section>

		<section class="border-t border-line py-12">
			<h2 class="text-[15px] font-semibold">{tr('landing.faq_title')}</h2>
			<dl class="mt-4 divide-y divide-line rounded-lg border border-line bg-surface">
				{#each faqs as number (number)}
					<div class="px-4 py-4">
						<dt class="text-[14px] font-medium">{tr(`landing.faq_${number}_q`)}</dt>
						<dd class="mt-1 text-[14px] leading-relaxed text-fg-muted">{tr(`landing.faq_${number}_a`)}</dd>
					</div>
				{/each}
			</dl>
		</section>
	</main>

	<footer class="border-t border-line px-4 py-4 text-[12px] text-fg-subtle sm:px-6" style="padding-bottom: calc(1rem + env(safe-area-inset-bottom, 0px))">
		<div class="mx-auto flex max-w-4xl gap-4">
			<span>{tr('app.name')}</span>
			<a href="/courses" class="hover:text-fg hover:underline">{tr('nav.courses')}</a>
			<a href="/privacy" class="hover:text-fg hover:underline">{tr('common.privacy')}</a>
		</div>
	</footer>
</div>
