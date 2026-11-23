import type {
    AdminCourse,
    AdminUsersResponse,
    LogLevel,
    LogsResponse,
    TestEmailResult,
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
    trafficReport: () => api.get<TrafficReport>('/admin/traffic-report'),
    logs: (params: { level?: LogLevel; category?: string; q?: string; page?: number; per_page?: number }) => {
        const query = new URLSearchParams();
        for (const [key, value] of Object.entries(params)) if (value) query.set(key, String(value));
        return api.get<LogsResponse>(`/admin/logs?${query}`);
    },
    sendTestEmail: () => api.post<TestEmailResult>('/admin/mail/test'),
    users: (params: { q?: string; status?: 'verified' | 'unverified'; page?: number; per_page?: number }) => {
        const query = new URLSearchParams();
        for (const [key, value] of Object.entries(params)) if (value) query.set(key, String(value));
        return api.get<AdminUsersResponse>(`/admin/users?${query}`);
    }
};
