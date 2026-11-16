import type {
    AdminCourse,
    AdminCourseDetail,
    AuthoringGuide,
    CourseValidationReport,
    PublishResult,
    TrafficReport
} from '#lib/types/api.ts';
import { api } from './client.ts';

export const adminApi = {
    courses: () => api.get<AdminCourse[]>('/admin/courses'),
    course: (courseId: string) => api.get<AdminCourseDetail>(`/admin/courses/${encodeURIComponent(courseId)}`),
    validate: (source: string) => api.post<CourseValidationReport>('/admin/courses/validate', { source }),
    publish: (source: string) => api.post<PublishResult>('/admin/courses', { source }),
    guide: () => api.get<AuthoringGuide>('/admin/authoring-guide'),
    trafficReport: () => api.get<TrafficReport>('/admin/traffic-report')
};
