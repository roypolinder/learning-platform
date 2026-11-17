<script lang="ts">
	import { locales, translate, type Locale } from '#lib/i18n/index.svelte.ts';

	interface Props {
		value: Locale;
		onchange: (locale: Locale) => void;
		class?: string;
	}

	let { value, onchange, class: className = '' }: Props = $props();
</script>

<label class="relative inline-flex items-center {className}">
	<span class="sr-only">{translate(value, 'language.picker')}</span>
	<select
		{value}
		onchange={(event) => onchange(event.currentTarget.value as Locale)}
		class="h-8 cursor-pointer appearance-none rounded-md border border-line bg-surface py-0 pr-7 pl-2.5 text-[13px] text-fg hover:border-line-strong focus:border-accent focus:outline-none"
	>
		{#each locales as locale (locale.code)}
			<option value={locale.code}>{locale.flag} {locale.label}</option>
		{/each}
	</select>
	<svg class="pointer-events-none absolute right-2 text-fg-subtle" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
		<path d="M4 6l4 4 4-4" />
	</svg>
</label>
