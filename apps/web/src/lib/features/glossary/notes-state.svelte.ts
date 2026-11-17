import { t } from '#lib/i18n/index.svelte.ts';
import { ApiError } from '#lib/services/api/client.ts';
import { notesApi } from '#lib/services/api/learners.ts';
import type { Note } from '#lib/types/api.ts';

/** The learner's notes across all courses. Only the current course's notes are editable. */
export class NotesState {
	all = $state<Note[]>([]);
	loading = $state(true);
	error = $state<string | null>(null);
	saving = $state(false);

	readonly courseId: string;
	/** Course ids that have notes, the current course first. */
	readonly courses: string[];

	constructor(courseId: string) {
		this.courseId = courseId;
		this.courses = $derived([courseId, ...new Set(this.all.map((note) => note.course_slug).filter((id) => id !== courseId))]);
	}

	for(courseId: string): Note[] {
		return this.all.filter((note) => note.course_slug === courseId);
	}

	async load(): Promise<void> {
		this.loading = true;
		try {
			this.all = await notesApi.list();
			this.error = null;
		} catch (error) {
			this.error = error instanceof ApiError ? error.message : t('notes.load_error');
		} finally {
			this.loading = false;
		}
	}

	async add(body: string): Promise<boolean> {
		return this.save(async () => {
			const note = await notesApi.create(this.courseId, body);
			this.all = [...this.all, note];
		});
	}

	async update(id: string, body: string): Promise<boolean> {
		return this.save(async () => {
			const updated = await notesApi.update(id, body);
			this.all = this.all.map((note) => (note.id === id ? updated : note));
		});
	}

	async remove(id: string): Promise<boolean> {
		return this.save(async () => {
			await notesApi.remove(id);
			this.all = this.all.filter((note) => note.id !== id);
		});
	}

	private async save(action: () => Promise<void>): Promise<boolean> {
		this.saving = true;
		this.error = null;
		try {
			await action();
			return true;
		} catch (error) {
			this.error = error instanceof ApiError ? error.message : t('notes.error');
			return false;
		} finally {
			this.saving = false;
		}
	}
}
