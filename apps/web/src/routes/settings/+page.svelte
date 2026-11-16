<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import McpSetup from '#lib/features/integrations/McpSetup.svelte';
	import { ApiError } from '#lib/services/api/client.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { requireAuth } from '#lib/state/require-auth.svelte.ts';

	requireAuth();

	let displayName = $state('');
	let saving = $state(false);
	let message = $state<{ tone: 'success' | 'danger'; text: string } | null>(null);

	$effect(() => {
		if (auth.user && !displayName) displayName = auth.user.display_name;
	});

	async function save(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		message = null;
		try {
			auth.user = await authApi.updateProfile(displayName);
			message = { tone: 'success', text: 'Profile saved.' };
		} catch (error) {
			message = { tone: 'danger', text: error instanceof ApiError ? (Object.values(error.fieldErrors())[0] ?? error.message) : 'Could not save.' };
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head><title>Settings · Learning Platform</title></svelte:head>

<div class="mx-auto max-w-3xl px-6 py-8">
	<PageHeader title="Settings" />

	{#if auth.user}
		<section class="mt-6 grid gap-6 border-b border-line pb-8 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium">Account</h2>
				<p class="mt-1 text-[13px] text-fg-muted">How you appear on the platform.</p>
			</div>
			<form class="flex max-w-sm flex-col gap-4" onsubmit={save}>
				<TextField label="Display name" bind:value={displayName} maxlength={80} />
				<TextField label="Email" value={auth.user.email} disabled hint="Email changes are not available yet." />
				{#if message}<Notice tone={message.tone}>{message.text}</Notice>{/if}
				<div><Button type="submit" variant="primary" loading={saving}>Save changes</Button></div>
			</form>
		</section>

		<section class="mt-8 grid gap-6 md:grid-cols-[200px_1fr]">
			<div>
				<h2 class="text-[14px] font-medium">Connected tools</h2>
				<p class="mt-1 text-[13px] text-fg-muted">AI agents that can read your course context and report results.</p>
			</div>
			<McpSetup />
		</section>
	{/if}
</div>
