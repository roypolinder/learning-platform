<script lang="ts">
	import { page } from '$app/state';
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import AuthForm from '#lib/features/authentication/AuthForm.svelte';

	const next = $derived(page.url.searchParams.get('next'));
</script>

<svelte:head><title>Sign in · Learning Platform</title></svelte:head>

<AuthLayout title="Welcome back" subtitle="Sign in to continue where you left off.">
	<AuthForm mode="login" />
	{#snippet footer()}
		New here? <a href={next ? `/register?next=${encodeURIComponent(next)}` : '/register'} class="font-medium text-fg hover:underline">Create an account</a>
	{/snippet}
</AuthLayout>
