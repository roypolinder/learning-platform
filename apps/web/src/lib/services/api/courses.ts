import type { CourseDetailResponse, CourseList, Difficulty } from '#lib/types/api.ts';
import { api } from './client.ts';

export interface CourseFilters {
	q?: string;
	category?: string;
	difficulty?: Difficulty;
}

export const coursesApi = {
	list: (filters: CourseFilters = {}) => {
		const params = new URLSearchParams();
		for (const [key, value] of Object.entries(filters)) {
			if (value) params.set(key, value);
		}
		params.set('per_page', '50');
		return api.get<CourseList>(`/courses?${params}`);
	},
	get: (courseId: string) => api.get<CourseDetailResponse>(`/courses/${encodeURIComponent(courseId)}`)
};
