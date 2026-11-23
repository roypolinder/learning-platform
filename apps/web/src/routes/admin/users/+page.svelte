<script lang="ts">
	import { untrack } from 'svelte';
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import AdminGate from '#lib/features/admin/AdminGate.svelte';
	import SignupChart from '#lib/features/admin/SignupChart.svelte';
	import { adminApi } from '#lib/services/api/admin.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { AdminUsersResponse } from '#lib/types/api.ts';
	import { formatRelative } from '#lib/utils/format.ts';

	const PER_PAGE = 50;

	let data = $state<AdminUsersResponse | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let query = $state('');
	let status = $state<'' | 'verified' | 'unverified'>('');
	let page = $state(1);

	async function load() {
		loading = true;
		error = null;
		try {
			data = await adminApi.users({ q: query.trim() || undefined, status: status || undefined, page, per_page: PER_PAGE });
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Could not load users.';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (!auth.isSuperAdmin) return;
		void [status, page];
		// Typing in the search box waits for Search instead of querying on every key.
		untrack(() => void load());
	});

	function search(event: SubmitEvent) {
		event.preventDefault();
		page = 1;
		void load();
	}

	const pages = $derived(data ? Math.max(1, Math.ceil(data.users.total / PER_PAGE)) : 1);
	const verifiedShare = $derived(data && data.summary.total ? Math.round((data.summary.verified / data.summary.total) * 100) : 0);
	const formatDateTime = (iso: string) =>
		new Date(iso).toLocaleString('en-GB', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
</script>

<svelte:head><title>Users · Learning Platform</title></svelte:head>

<AdminGate>
	<div class="mx-auto max-w-6xl px-6 py-8">
		<PageHeader title="Users" description="Who signed up and with which email. Unverified accounts that never started a course are often bots.">
			{#snippet actions()}
				<Button onclick={load} {loading}><Icon name="refresh" size={14} /> Reload</Button>
			{/snippet}
		</PageHeader>

		{#if error}
			<div class="mt-6"><Notice tone="danger">{error}</Notice></div>
		{:else if !data}
			<div class="mt-6 flex flex-col gap-3"><Skeleton class="h-20" /><Skeleton class="h-40" /><Skeleton class="h-64" /></div>
		{:else}
			<div class="mt-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
				{#each [{ label: 'Accounts', value: data.summary.total, note: `${data.summary.verified} verified (${verifiedShare}%)` }, { label: 'Last 24 hours', value: data.summary.last_24_hours, note: 'new sign-ups' }, { label: 'Last 7 days', value: data.summary.last_7_days, note: 'new sign-ups' }, { label: 'Last 30 days', value: data.summary.last_30_days, note: 'new sign-ups' }] as tile (tile.label)}
					<div class="rounded-lg border border-line bg-surface px-4 py-3">
						<p class="text-[12px] text-fg-muted">{tile.label}</p>
						<p class="mt-1 text-[22px] font-semibold tracking-tight tabular-nums">{tile.value}</p>
						<p class="text-[12px] text-fg-subtle">{tile.note}</p>
					</div>
				{/each}
			</div>

			<div class="mt-4"><SignupChart days={data.summary.signups_per_day} /></div>

			<div class="mt-6 flex flex-wrap items-center gap-2">
				<form class="flex flex-1 gap-2" onsubmit={search}>
					<input
						type="search"
						bind:value={query}
						placeholder="Search email or name"
						aria-label="Search email or name"
						class="h-8 w-full max-w-xs rounded-md border border-line-strong bg-surface px-3 text-[13px] focus:border-accent focus:outline-none"
					/>
					<Button type="submit" size="sm"><Icon name="search" size={13} /> Search</Button>
				</form>
				<select
					bind:value={status}
					onchange={() => (page = 1)}
					aria-label="Email status"
					class="h-8 rounded-md border border-line-strong bg-surface px-2 text-[13px]"
				>
					<option value="">All accounts</option>
					<option value="verified">Verified email</option>
					<option value="unverified">Unverified email</option>
				</select>
			</div>

			<div class="mt-3 overflow-x-auto rounded-lg border border-line bg-surface">
				<table class="w-full text-left text-[13px]">
					<thead class="border-b border-line text-[12px] text-fg-muted">
						<tr>
							<th class="px-3 py-2 font-medium">Email</th>
							<th class="px-3 py-2 font-medium">Name</th>
							<th class="px-3 py-2 font-medium">Signed up</th>
							<th class="px-3 py-2 font-medium">Last sign-in</th>
							<th class="px-3 py-2 text-right font-medium">Courses</th>
							<th class="px-3 py-2 font-medium">Activity</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-line">
						{#each data.users.items as user (user.id)}
							<tr class="hover:bg-surface-hover">
								<td class="px-3 py-2">
									<div class="flex flex-wrap items-center gap-1.5">
										<span class="font-mono text-[12px]">{user.email}</span>
										{#if user.email_verified_at}
											<Badge tone="success">Verified</Badge>
										{:else}
											<Badge tone="warning">Unverified</Badge>
										{/if}
										{#if user.role === 'super_admin'}<Badge tone="accent">Admin</Badge>{/if}
									</div>
								</td>
								<td class="max-w-[180px] truncate px-3 py-2" title={user.display_name}>{user.display_name}</td>
								<td class="px-3 py-2 whitespace-nowrap text-fg-muted" title={formatDateTime(user.created_at)}>{formatRelative(user.created_at)}</td>
								<td class="px-3 py-2 whitespace-nowrap text-fg-muted">
									{#if user.last_sign_in_at}<span title={formatDateTime(user.last_sign_in_at)}>{formatRelative(user.last_sign_in_at)}</span> · {user.sign_ins}×{:else}—{/if}
								</td>
								<td class="px-3 py-2 text-right tabular-nums">{user.courses_completed}/{user.courses_started}</td>
								<td class="px-3 py-2">
									<div class="flex flex-wrap gap-1.5 text-[12px] text-fg-muted">
										<span>{user.locale.toUpperCase()}</span>
										{#if user.theme_id}<span>· {user.theme_id}</span>{/if}
										{#if user.agent_connected}<span class="text-success">· agent connected</span>{/if}
									</div>
								</td>
							</tr>
						{:else}
							<tr><td colspan="6" class="px-3 py-8 text-center text-fg-muted">No accounts match.</td></tr>
						{/each}
					</tbody>
				</table>
			</div>
			<div class="mt-3 flex items-center justify-between text-[12px] text-fg-muted">
				<span>{data.users.total} {data.users.total === 1 ? 'account' : 'accounts'} · courses shown as completed/started</span>
				{#if pages > 1}
					<div class="flex items-center gap-2">
						<Button size="sm" variant="ghost" disabled={page <= 1} onclick={() => (page -= 1)}><Icon name="arrow-left" size={13} /></Button>
						<span>Page {page} of {pages}</span>
						<Button size="sm" variant="ghost" disabled={page >= pages} onclick={() => (page += 1)}><Icon name="arrow-right" size={13} /></Button>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</AdminGate>
