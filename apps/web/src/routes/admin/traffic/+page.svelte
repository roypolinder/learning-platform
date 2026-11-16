<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import AdminGate from '#lib/features/admin/AdminGate.svelte';
	import { adminApi } from '#lib/services/api/admin.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { TrafficReport } from '#lib/types/api.ts';
	import { formatRelative } from '#lib/utils/format.ts';

	let report = $state<TrafficReport | null>(null);
	let unavailable = $state(false);
	let error = $state<string | null>(null);
	let loading = $state(false);

	async function load() {
		loading = true;
		error = null;
		try {
			report = await adminApi.trafficReport();
			unavailable = false;
		} catch (e) {
			if (e instanceof ApiError && e.status === 404) unavailable = true;
			else error = e instanceof ApiError ? e.message : 'Could not load the traffic report.';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (auth.isSuperAdmin) void load();
	});
</script>

<svelte:head><title>Traffic · Learning Platform</title></svelte:head>

<AdminGate>
	<div class="mx-auto max-w-7xl px-6 py-8">
		<PageHeader
			title="Traffic"
			description={report ? `Generated ${formatRelative(report.generated_at)} from the web server access log.` : 'Visitors and requests from the web server access log.'}
		>
			{#snippet actions()}
				<Button onclick={load} {loading}><Icon name="refresh" size={14} /> Reload</Button>
			{/snippet}
		</PageHeader>

		<div class="mt-6">
			{#if error}
				<Notice tone="danger">{error}</Notice>
			{:else if unavailable}
				<Notice tone="warning" title="No report yet">
					The report is generated on the server every 15 minutes by <span class="font-mono">setup.sh</span>. If this stays empty, run
					<span class="font-mono">sudo ./setup.sh</span> again and check that <span class="font-mono">TRAFFIC_REPORT_PATH</span> is set in the configuration.
				</Notice>
			{:else if !report}
				<Skeleton class="h-96" />
			{:else}
				<!-- Opaque origin: the report cannot read the session or call the API. -->
				<iframe
					title="Traffic report"
					sandbox="allow-scripts"
					srcdoc={report.html}
					class="h-[calc(100vh-12rem)] min-h-[520px] w-full rounded-lg border border-line bg-white"
				></iframe>
			{/if}
		</div>
	</div>
</AdminGate>
