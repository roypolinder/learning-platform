export type StepStatus = 'locked' | 'available' | 'in_progress' | 'completed' | 'failed' | 'skipped';
export type SessionStatus = 'active' | 'completed' | 'abandoned';
export type Difficulty = 'beginner' | 'intermediate' | 'advanced';
export type StepCategory = 'content' | 'assessment' | 'practical' | 'workflow';
export type CompletionStrategy = 'acknowledge' | 'submission' | 'event' | 'automatic' | 'derived';
export type OutcomeStatus = 'passed' | 'failed' | 'pending';

export interface Page<T> {
	items: T[];
	page: number;
	per_page: number;
	total: number;
}

export interface ApiErrorBody {
	error: { code: string; message: string; details?: unknown };
}

export type UserRole = 'learner' | 'super_admin';

export interface User {
	id: string;
	email: string;
	display_name: string;
	role: UserRole;
	locale: 'en' | 'nl';
	created_at?: string;
}

export interface CourseSummary {
	id: string;
	version: string;
	title: string;
	summary: string;
	category: string;
	difficulty: Difficulty;
	estimated_minutes: number;
	tags: string[];
	section_count: number;
	step_count: number;
}

export interface Prerequisite {
	title: string;
	course?: string | null;
}

export interface CourseStep {
	id: string;
	section_id: string;
	type: string;
	category: StepCategory;
	completion: CompletionStrategy;
	title: string;
	description: string | null;
	body: string | null;
	extension: string | null;
	config: Record<string, unknown>;
	submission_schema: Record<string, unknown> | null;
	/** Requests the learner's browser sends to their own service; see service-runner.ts. */
	client_task?: unknown;
	depends_on: string[];
	optional: boolean;
	estimated_minutes: number | null;
	help: string | null;
}

export interface CourseSection {
	id: string;
	title: string;
	description: string | null;
	steps: CourseStep[];
}

export interface GlossaryEntry {
	term: string;
	definition: string;
}

export interface CourseDetail extends CourseSummary {
	description: string;
	help: string | null;
	objectives: string[];
	prerequisites: Prerequisite[];
	glossary: GlossaryEntry[];
	extensions: string[];
	sections: CourseSection[];
}

export interface CourseProgress {
	state: 'in_progress' | 'completed';
	percent: number;
	session_id: string;
}

export interface CourseList extends Page<CourseSummary> {
	categories: string[];
	/** The signed-in learner's progress keyed by course id. */
	progress: Record<string, CourseProgress>;
}

export interface CheckResult {
	id: string;
	label: string;
	passed: boolean;
	detail?: string;
}

export interface ValidationOutcome {
	status: OutcomeStatus;
	summary: string;
	score?: number;
	checks: CheckResult[];
	feedback?: unknown;
}

export interface StepProgress {
	step_id: string;
	status: StepStatus;
	attempts: number;
	score: number | null;
	outcome: ValidationOutcome | null;
}

export interface Progress {
	session_id: string;
	status: SessionStatus;
	current_step_id: string | null;
	completed_required: number;
	total_required: number;
	percent: number;
	steps: StepProgress[];
}

export interface Session {
	id: string;
	course_id: string;
	course_version_id: string;
	status: SessionStatus;
	current_step_id: string | null;
	started_at: string;
	last_accessed_at: string;
	completed_at: string | null;
}

export interface CourseDetailResponse {
	course: CourseDetail;
	session: Session | null;
}

export interface SessionDetail {
	session: Session;
	course: CourseDetail;
	progress: Progress;
}

export interface StepActionResult {
	progress: Progress;
	outcome: ValidationOutcome | null;
}

export interface SessionSummary {
	session: Session;
	course: CourseSummary;
	completed_required: number;
	total_required: number;
	percent: number;
	current_step: { id: string; title: string } | null;
}

export interface ProgressOverview {
	stats: { active_courses: number; completed_courses: number; steps_completed: number };
	sessions: SessionSummary[];
}

export interface IntegrationConnection {
	id: string;
	kind: 'mcp';
	auth_method: 'token' | 'oauth';
	label: string;
	token_hint: string;
	status: 'pending' | 'connected' | 'revoked';
	client_name: string | null;
	last_session_id: string | null;
	last_seen_at: string | null;
	created_at: string;
	expires_at: string;
	revoked_at: string | null;
}

export interface IntegrationsResponse {
	mcp_endpoint: string;
	mcp_config: unknown;
	connections: IntegrationConnection[];
}

export interface McpConnectionCreated {
	connection: IntegrationConnection;
	token: string;
	endpoint: string;
}

/** Query parameters of an OAuth authorization request, passed through untouched. */
export type AuthorizationParams = Record<string, string>;

export interface AuthorizationPreview {
	client_name: string;
	redirect_target: string;
	scope: string;
}

export interface AdminCourse {
	id: string;
	title: string;
	version: string | null;
	category: string | null;
	source: 'file' | 'authored';
	editable: boolean;
	published: boolean;
	step_count: number;
	session_count: number;
	completed_count: number;
	updated_at: string;
}

export interface AdminCourseDetail extends AdminCourse {
	definition: string;
}

export interface CourseIssue {
	path: string;
	message: string;
}

export interface CourseValidationReport {
	valid: boolean;
	issues: CourseIssue[];
	course: CourseSummary | null;
}

export interface PublishResult {
	status: 'created' | 'new_version' | 'updated_in_place' | 'unchanged';
	course: CourseSummary;
}

export interface StepTypeInfo {
	name: string;
	category: StepCategory;
	completion: CompletionStrategy;
	description: string;
	provider: string | null;
}

export interface AuthoringGuide {
	format: string;
	rules: string[];
	guide_markdown: string;
	step_types: StepTypeInfo[];
}

export interface TrafficReport {
	generated_at: string;
	/** Self-contained HTML document; render only inside a sandboxed iframe. */
	html: string;
}

export type Stack = 'javascript' | 'typescript' | 'python' | 'other';

export interface ThemeSummary {
	id: string;
	name: string;
	tagline: string;
	icon: string | null;
}

export interface Project {
	theme_id: string;
	stack: Stack;
	created_at: string;
	theme: ThemeSummary | null;
}

export interface Note {
	id: string;
	course_slug: string;
	body: string;
	created_at: string;
	updated_at: string;
}
