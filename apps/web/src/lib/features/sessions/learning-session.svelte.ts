import { ApiError } from '#lib/services/api/client.ts';
import { sessionsApi } from '#lib/services/api/sessions.ts';
import type {
	CourseSection,
	CourseStep,
	Progress,
	SessionDetail,
	StepProgress,
	ValidationOutcome
} from '#lib/types/api.ts';

/** Client-side state of one course session, kept in sync with the server via SSE. */
export class LearningSession {
	detail = $state<SessionDetail | null>(null);
	selectedStepId = $state<string | null>(null);
	pending = $state(false);
	/** Server updates right after the learner's own action are echoes, not agent activity. */
	private lastLocalAction = 0;
	error = $state<string | null>(null);
	loadError = $state<ApiError | null>(null);
	/** Set when a step completed without the learner acting in the browser (e.g. via MCP). */
	remoteCompletion = $state<string | null>(null);

	private unsubscribe: (() => void) | null = null;

	readonly progress = $derived(this.detail?.progress ?? null);
	readonly steps: CourseStep[] = $derived(this.detail?.course.sections.flatMap((s) => s.steps) ?? []);
	readonly selectedStep = $derived(this.steps.find((s) => s.id === this.selectedStepId) ?? null);
	readonly selectedSection: CourseSection | null = $derived(
		this.detail?.course.sections.find((s) => s.id === this.selectedStep?.section_id) ?? null
	);
	readonly selectedProgress: StepProgress | null = $derived(
		this.progressFor(this.selectedStepId)
	);

	constructor(readonly sessionId: string) {}

	async load(): Promise<void> {
		try {
			this.detail = await sessionsApi.get(this.sessionId);
			this.loadError = null;
			const target = this.detail.progress.current_step_id ?? this.steps[0]?.id ?? null;
			if (target) await this.select(target);
			this.unsubscribe = sessionsApi.subscribe(this.sessionId, () => void this.syncFromServer());
		} catch (error) {
			this.loadError = error instanceof ApiError ? error : new ApiError(0, 'unknown', String(error));
		}
	}

	dispose(): void {
		this.unsubscribe?.();
	}

	progressFor(stepId: string | null): StepProgress | null {
		return this.progress?.steps.find((step) => step.step_id === stepId) ?? null;
	}

	neighbour(offset: -1 | 1): CourseStep | null {
		const index = this.steps.findIndex((step) => step.id === this.selectedStepId);
		return index < 0 ? null : (this.steps[index + offset] ?? null);
	}

	async select(stepId: string): Promise<void> {
		this.selectedStepId = stepId;
		this.error = null;
		this.remoteCompletion = null;
		if (this.progressFor(stepId)?.status === 'available') {
			await this.run(() => sessionsApi.openStep(this.sessionId, stepId));
		}
	}

	async complete(stepId: string): Promise<void> {
		const result = await this.run(() => sessionsApi.completeStep(this.sessionId, stepId));
		if (result) this.advance(stepId);
	}

	async submit(stepId: string, payload: unknown): Promise<ValidationOutcome | null> {
		const result = await this.run(() => sessionsApi.submit(this.sessionId, stepId, payload));
		return result?.outcome ?? null;
	}

	async recheck(): Promise<void> {
		await this.run(async () => ({ progress: await sessionsApi.refresh(this.sessionId), outcome: null }));
	}

	/** Moves to the next step the learner can act on. */
	advance(fromStepId: string): void {
		const progress = this.progress;
		if (!progress) return;
		const index = this.steps.findIndex((step) => step.id === fromStepId);
		const next = this.steps
			.slice(index + 1)
			.find((step) => ['available', 'in_progress', 'failed'].includes(this.progressFor(step.id)?.status ?? ''));
		if (next) void this.select(next.id);
	}

	private async run(
		action: () => Promise<{ progress: Progress; outcome: ValidationOutcome | null }>
	): Promise<{ progress: Progress; outcome: ValidationOutcome | null } | null> {
		this.pending = true;
		this.error = null;
		try {
			const result = await action();
			this.applyProgress(result.progress);
			return result;
		} catch (error) {
			this.error = error instanceof ApiError ? error.message : 'Something went wrong.';
			return null;
		} finally {
			this.pending = false;
			this.lastLocalAction = Date.now();
		}
	}

	private async syncFromServer(): Promise<void> {
		const before = this.selectedProgress?.status;
		const local = this.pending;
		try {
			const progress = await sessionsApi.progress(this.sessionId);
			this.applyProgress(progress);
			if (before && before !== 'completed' && this.selectedProgress?.status === 'completed' && !local && !this.pending && Date.now() - this.lastLocalAction > 3000) {
				this.remoteCompletion = this.selectedStepId;
			}
		} catch {
			// Transient; the next event or user action resyncs.
		}
	}

	private applyProgress(progress: Progress): void {
		if (!this.detail) return;
		this.detail = {
			...this.detail,
			progress,
			session: { ...this.detail.session, status: progress.status, current_step_id: progress.current_step_id }
		};
	}
}
