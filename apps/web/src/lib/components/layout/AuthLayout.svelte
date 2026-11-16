<script lang="ts">
	import Logo from '#lib/components/ui/Logo.svelte';
	import type { Snippet } from 'svelte';

	let { title, subtitle, children, footer }: { title: string; subtitle: string; children: Snippet; footer: Snippet } = $props();
</script>

<div class="flex min-h-screen items-center justify-center px-4 py-12">
	<div class="w-full max-w-[360px]">
		<a href="/courses" class="mb-8 flex items-center gap-2">
			<Logo size={28} />
			<span class="text-[14px] font-semibold tracking-tight">Learning Platform</span>
		</a>
		<h1 class="text-[20px] font-semibold tracking-tight">{title}</h1>
		<p class="mt-1 text-[14px] text-fg-muted">{subtitle}</p>
		<div class="mt-6">{@render children()}</div>
		<div class="mt-6 border-t border-line pt-4 text-[13px] text-fg-muted">{@render footer()}</div>
		<p class="mt-4 text-[12px] text-fg-subtle"><a href="/privacy" class="hover:text-fg hover:underline">Privacy</a></p>
	</div>
</div>
