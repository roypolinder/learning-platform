<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
	type Size = 'sm' | 'md';

	interface Props extends HTMLButtonAttributes {
		variant?: Variant;
		size?: Size;
		href?: string;
		loading?: boolean;
		children: Snippet;
	}

	let {
		variant = 'secondary',
		size = 'md',
		href,
		loading = false,
		disabled,
		type = 'button',
		class: className = '',
		children,
		...rest
	}: Props = $props();

	const variants: Record<Variant, string> = {
		primary: 'bg-accent text-accent-fg hover:bg-accent-hover border-transparent',
		secondary: 'bg-surface text-fg border-line-strong hover:bg-surface-hover',
		ghost: 'bg-transparent text-fg-muted border-transparent hover:bg-surface-hover hover:text-fg',
		danger: 'bg-surface text-danger border-line-strong hover:bg-danger-soft'
	};
	const sizes: Record<Size, string> = {
		sm: 'h-7 px-2.5 text-[13px] gap-1.5',
		md: 'h-8 px-3 text-[13px] gap-2'
	};

	let classes = $derived(
		`inline-flex items-center justify-center rounded-md border font-medium whitespace-nowrap transition-colors disabled:opacity-50 disabled:pointer-events-none ${variants[variant]} ${sizes[size]} ${className}`
	);
</script>

{#if href}
	<a {href} class={classes}>{@render children()}</a>
{:else}
	<button {type} class={classes} disabled={disabled || loading} aria-busy={loading} {...rest}>
		{#if loading}
			<span class="size-3 animate-spin rounded-full border-[1.5px] border-current border-r-transparent"></span>
		{/if}
		{@render children()}
	</button>
{/if}
