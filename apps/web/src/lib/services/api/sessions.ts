import type { Progress, SessionDetail, StepActionResult } from '#lib/types/api.ts';
import { api } from './client.ts';

const step = (sessionId: string, stepId: string) =>
	`/sessions/${sessionId}/steps/${encodeURIComponent(stepId)}`;

export const sessionsApi = {
	start: (courseId: string) =>
		api.post<SessionDetail>(`/courses/${encodeURIComponent(courseId)}/sessions`),
	get: (sessionId: string) => api.get<SessionDetail>(`/sessions/${sessionId}`),
	progress: (sessionId: string) => api.get<Progress>(`/sessions/${sessionId}/progress`),
	refresh: (sessionId: string) => api.post<Progress>(`/sessions/${sessionId}/refresh`),
	openStep: (sessionId: string, stepId: string) =>
		api.post<StepActionResult>(`${step(sessionId, stepId)}/open`),
	completeStep: (sessionId: string, stepId: string) =>
		api.post<StepActionResult>(`${step(sessionId, stepId)}/complete`),
	submit: (sessionId: string, stepId: string, payload: unknown) =>
		api.post<StepActionResult>(`${step(sessionId, stepId)}/submissions`, { payload }),
	/** Server-sent events fired whenever the session changes, including via MCP. */
	subscribe(sessionId: string, onChange: () => void): () => void {
		const source = new EventSource(`/api/v1/sessions/${sessionId}/stream`);
		// `ready` fires on every (re)connect, so changes missed while disconnected are picked up.
		source.addEventListener('ready', onChange);
		source.addEventListener('progress', onChange);
		source.addEventListener('resync', onChange);
		return () => source.close();
	}
};
