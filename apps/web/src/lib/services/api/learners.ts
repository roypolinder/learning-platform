import type { Note, Project, Stack, ThemeSummary } from '#lib/types/api.ts';
import { api } from './client.ts';

export const learnersApi = {
	themes: () => api.get<{ items: ThemeSummary[] }>('/themes').then((r) => r.items),
	project: () => api.get<{ project: Project | null }>('/project').then((r) => r.project),
	createProject: (input: { theme_id: string; stack: Stack }) =>
		api.post<{ project: Project | null }>('/project', input).then((r) => r.project),
	projectData: () => api.get<unknown>('/project/data')
};

export const notesApi = {
	/** Without `course`, returns the notes of every course. */
	list: (course?: string) =>
		api.get<{ items: Note[] }>(course ? `/notes?course=${encodeURIComponent(course)}` : '/notes').then((r) => r.items),
	create: (course: string, body: string) => api.post<Note>('/notes', { course, body }),
	update: (id: string, body: string) => api.patch<Note>(`/notes/${id}`, { body }),
	remove: (id: string) => api.delete<void>(`/notes/${id}`)
};
