<script lang="ts">
	import { page } from '$app/state';
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import AuthForm from '#lib/features/authentication/AuthForm.svelte';

	const next = $derived(page.url.searchParams.get('next'));
</script>

<svelte:head><title>Create account · Learning Platform</title></svelte:head>

<AuthLayout title="Create your account" subtitle="Courses are free. Progress is saved to your account.">
	<AuthForm mode="register" />
	{#snippet footer()}
		Already have an account? <a href={next ? `/login?next=${encodeURIComponent(next)}` : '/login'} class="font-medium text-fg hover:underline">Sign in</a>
	{/snippet}
</AuthLayout>
