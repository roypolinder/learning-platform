import { ApiError } from '#lib/services/api/client.ts';
import { coursesApi } from '#lib/services/api/courses.ts';
import type { CourseProgress, CourseSummary, Difficulty } from '#lib/types/api.ts';

export type ProgressFilter = 'all' | 'not_started' | 'in_progress' | 'completed';

export class CourseCatalog {
	query = $state('');
	category = $state<string | null>(null);
	difficulty = $state<Difficulty | null>(null);
	progressFilter = $state<ProgressFilter>('all');
	courses = $state<CourseSummary[]>([]);
	progress = $state<Record<string, CourseProgress>>({});
	categories = $state<string[]>([]);
	loading = $state(true);
	error = $state<string | null>(null);
	private requestId = 0;

	readonly visible = $derived(
		this.courses.filter((course) => {
			if (this.progressFilter === 'all') return true;
			const state = this.progress[course.id]?.state ?? 'not_started';
			return state === this.progressFilter;
		})
	);

	count(filter: ProgressFilter): number {
		if (filter === 'all') return this.courses.length;
		return this.courses.filter((course) => (this.progress[course.id]?.state ?? 'not_started') === filter).length;
	}

	async load(): Promise<void> {
		const id = ++this.requestId;
		this.loading = true;
		try {
			const result = await coursesApi.list({
				q: this.query.trim() || undefined,
				category: this.category ?? undefined,
				difficulty: this.difficulty ?? undefined
			});
			// Ignore responses that arrive after a newer request was issued.
			if (id !== this.requestId) return;
			this.courses = result.items;
			this.progress = result.progress ?? {};
			if (!this.categories.length) this.categories = result.categories;
			this.error = null;
		} catch (error) {
			if (id === this.requestId) this.error = error instanceof ApiError ? error.message : 'Could not load courses.';
		} finally {
			if (id === this.requestId) this.loading = false;
		}
	}
}
