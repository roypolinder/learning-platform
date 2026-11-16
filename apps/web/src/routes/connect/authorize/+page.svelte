<script lang="ts">
	import { page } from '$app/state';
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import { ApiError } from '#lib/services/api/client.ts';
	import { oauthApi } from '#lib/services/api/oauth.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { requireAuth } from '#lib/state/require-auth.svelte.ts';
	import type { AuthorizationPreview } from '#lib/types/api.ts';

	requireAuth();

	const params = Object.fromEntries(page.url.searchParams);
	let preview = $state<AuthorizationPreview | null>(null);
	let error = $state<string | null>(null);
	let deciding = $state<'allow' | 'deny' | null>(null);
	let redirected = $state(false);
	let requested = false;

	$effect(() => {
		if (!auth.user || requested) return;
		requested = true;
		oauthApi
			.preview(params)
			.then((result) => (preview = result))
			.catch((caught) => (error = message(caught)));
	});

	function message(caught: unknown): string {
		return caught instanceof ApiError ? caught.message : 'Something went wrong. Try again.';
	}

	async function decide(approve: boolean) {
		deciding = approve ? 'allow' : 'deny';
		error = null;
		try {
			const { redirect_to } = await oauthApi.decide(params, approve);
			redirected = approve;
			window.location.assign(redirect_to);
		} catch (caught) {
			error = message(caught);
			deciding = null;
		}
	}
</script>

<svelte:head><title>Connect an agent · Learning Platform</title></svelte:head>

<AuthLayout title="Connect an agent" subtitle="An application wants to use the learning platform as you.">
	<div class="flex flex-col gap-4">
		{#if error}<Notice tone="danger" title="Cannot connect">{error}</Notice>{/if}

		{#if redirected}
			<Notice tone="success" title="Access granted">
				Returning to {preview?.client_name}. You can close this tab once your editor confirms the sign-in.
			</Notice>
		{:else if preview}
			<p class="text-[14px]">
				<span class="font-semibold">{preview.client_name}</span> is asking to connect to your account.
			</p>
			<ul class="flex list-disc flex-col gap-1 pl-5 text-[13px] text-fg-muted">
				<li>See your courses, current step and progress</li>
				<li>Submit task results for your course sessions</li>
			</ul>
			<p class="text-[12px] text-fg-subtle">
				It cannot see your password or other learners' sessions. You can revoke access at any time in
				Settings. After you decide, your browser returns to <span class="font-mono">{preview.redirect_target}</span>.
			</p>
			<div class="flex gap-2">
				<Button variant="primary" class="h-9 flex-1" loading={deciding === 'allow'} disabled={deciding !== null} onclick={() => decide(true)}>
					Allow
				</Button>
				<Button class="h-9 flex-1" loading={deciding === 'deny'} disabled={deciding !== null} onclick={() => decide(false)}>
					Deny
				</Button>
			</div>
		{:else if !error}
			<Skeleton class="h-24" />
		{/if}
	</div>
	{#snippet footer()}
		{#if auth.user}
			Signed in as <span class="font-medium text-fg">{auth.user.email}</span>.
			<button type="button" class="font-medium text-fg hover:underline" onclick={() => auth.logout()}>Not you?</button>
		{/if}
	{/snippet}
</AuthLayout>
