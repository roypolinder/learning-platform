const labels: Record<string, string> = {
	explanation: 'Reading',
	interactive_content: 'Interactive',
	video: 'Video',
	code_example: 'Example',
	quiz: 'Quiz',
	multiple_choice: 'Question',
	free_text: 'Written answer',
	interactive_exercise: 'Exercise',
	external_task: 'Task',
	mcp_connection_check: 'MCP connection',
	http_endpoint_check: 'HTTP check',
	service_check: 'Service check',
	mcp_server_check: 'MCP server check',
	git_repository_check: 'Git check',
	filesystem_validation: 'File check',
	configuration_validation: 'Config check',
	extension_validation: 'Validation',
	conditional: 'Review',
	prerequisite: 'Prerequisite',
	manual_completion: 'Checkpoint',
	automatic_completion: 'Milestone'
};

export function stepTypeLabel(type: string): string {
	return labels[type] ?? type.replaceAll('_', ' ');
}
