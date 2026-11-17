<script lang="ts">
	import { onMount } from 'svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon, { isIconName } from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { learnersApi } from '#lib/services/api/learners.ts';
	import type { CourseStep, Project, Stack, StepProgress, ThemeSummary } from '#lib/types/api.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';

	/** The learner chooses the website they build in every course. Chosen once, never changed. */
	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();

	const stacks: Stack[] = ['javascript', 'typescript', 'python', 'other'];

	let themes = $state<ThemeSummary[]>([]);
	let project = $state<Project | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let themeId = $state<string | null>(null);
	let stack = $state<Stack>('javascript');
	let saving = $state(false);

	onMount(async () => {
		try {
			[themes, project] = await Promise.all([learnersApi.themes(), learnersApi.project()]);
		} catch (caught) {
			error = caught instanceof ApiError ? caught.message : t('project.load_error');
		} finally {
			loading = false;
		}
	});

	let chosenTheme = $derived(themes.find((theme) => theme.id === themeId) ?? null);

	async function confirm() {
		if (!themeId) return;
		saving = true;
		error = null;
		try {
			project = await learnersApi.createProject({ theme_id: themeId, stack });
			// The course text now refers to the learner's own website.
			await session.reload();
			await session.recheck();
			if (session.progressFor(step.id)?.status === 'completed') session.advance(step.id);
		} catch (caught) {
			error = caught instanceof ApiError ? caught.message : t('common.error');
		} finally {
			saving = false;
		}
	}
</script>

<div class="flex flex-col gap-6">
	{#if error}<Notice tone="danger">{error}</Notice>{/if}

	{#if loading}
		<div class="grid gap-2 sm:grid-cols-2"><Skeleton class="h-16" /><Skeleton class="h-16" /><Skeleton class="h-16" /><Skeleton class="h-16" /></div>
	{:else if project}
		<div class="flex items-center gap-3 rounded-lg border border-transparent bg-success-soft px-4 py-3">
			<span class="grid size-9 shrink-0 place-items-center rounded-md bg-surface text-success">
				<Icon name={isIconName(project.theme?.icon) ? (project.theme?.icon ?? 'globe') : 'globe'} />
			</span>
			<p class="text-[14px]">
				{t('project.chosen', { theme: project.theme?.name ?? project.theme_id, stack: t(`project.stack.${project.stack}`) })}
			</p>
		</div>
		<section class="rounded-lg border border-line bg-surface px-4 py-3">
			<h3 class="text-[13px] font-medium">{t('project.data')}</h3>
			<p class="mt-1 text-[13px] text-fg-muted">{t('project.data_body')}</p>
		</section>
		{#if progress?.status === 'completed'}
			<div class="flex justify-end">
				<Button onclick={() => session.advance(step.id)}>{t('common.next_step')} <Icon name="arrow-right" size={14} /></Button>
			</div>
		{:else}
			<div class="flex justify-end">
				<Button loading={session.pending} onclick={() => session.recheck()}><Icon name="refresh" size={14} /> {t('common.check_again')}</Button>
			</div>
		{/if}
	{:else}
		<fieldset>
			<legend class="text-[15px] font-semibold">{t('project.choose_title')}</legend>
			<p class="mt-1 text-[13px] text-fg-muted">{t('project.choose_body')}</p>
			<div class="mt-3 grid gap-2 sm:grid-cols-2">
				{#each themes as theme (theme.id)}
					<label
						class="flex cursor-pointer items-start gap-3 rounded-lg border px-3 py-2.5 transition-colors {themeId === theme.id
							? 'border-accent bg-accent-soft'
							: 'border-line bg-surface hover:border-line-strong'}"
					>
						<input type="radio" name="theme" value={theme.id} bind:group={themeId} class="sr-only" />
						<span class="grid size-8 shrink-0 place-items-center rounded-md bg-surface-muted text-fg-muted">
							<Icon name={isIconName(theme.icon) ? theme.icon : 'globe'} />
						</span>
						<span class="min-w-0">
							<span class="block text-[14px] font-medium">{theme.name}</span>
							<span class="block text-[12px] leading-snug text-fg-muted">{theme.tagline}</span>
						</span>
					</label>
				{/each}
			</div>
		</fieldset>

		<fieldset>
			<legend class="text-[15px] font-semibold">{t('project.stack_title')}</legend>
			<p class="mt-1 text-[13px] text-fg-muted">{t('project.stack_body')}</p>
			<div class="mt-3 flex flex-col gap-2">
				{#each stacks as option (option)}
					<label
						class="flex cursor-pointer items-center gap-3 rounded-md border px-3 py-2.5 text-[14px] transition-colors {stack === option
							? 'border-accent bg-accent-soft'
							: 'border-line bg-surface hover:border-line-strong'}"
					>
						<input type="radio" name="stack" value={option} bind:group={stack} class="accent-[var(--accent)]" />
						<span class="min-w-0 flex-1">
							<span class="flex items-center gap-2 font-medium">
								{t(`project.stack.${option}`)}
								{#if option === 'javascript'}<Badge tone="accent">{t('project.recommended')}</Badge>{/if}
							</span>
							<span class="block text-[12px] text-fg-muted">{t(`project.stack.${option}_note`)}</span>
						</span>
					</label>
				{/each}
			</div>
		</fieldset>

		<div class="flex flex-wrap items-center justify-end gap-3 border-t border-line pt-4">
			<p class="flex items-center gap-1.5 text-[12px] text-fg-subtle"><Icon name="lock" size={13} /> {t('project.final')}</p>
			<Button variant="primary" disabled={!themeId} loading={saving} onclick={confirm}>
				{chosenTheme ? t('project.confirm_short', { theme: chosenTheme.name }) : t('project.confirm')}
				<Icon name="arrow-right" size={14} />
			</Button>
		</div>
	{/if}
</div>
