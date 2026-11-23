import type { MessageKey } from '#lib/i18n/index.svelte.ts';

/** Coding agents with a known way to add the platform's MCP server. */
export type AgentId = 'claude-code' | 'vscode' | 'opencode' | 'cursor' | 'codex' | 'other';

export interface AgentSetup {
	id: AgentId;
	/** Product name; `null` uses the translated "Other". */
	name: string | null;
	file: string;
	language: 'json' | 'toml';
	code: string;
	caption: MessageKey;
	/** Optional terminal command that writes the same configuration. */
	command?: string;
	/** Agent-specific steps; the shared "approve in the browser" step follows them. */
	steps: MessageKey[];
}

export const SERVER_NAME = 'learning-platform';

const json = (value: unknown) => JSON.stringify(value, null, 2);

/** Every configuration names the server, the transport and the URL; none holds a secret. */
export function agentSetups(endpoint: string): AgentSetup[] {
	const url = endpoint || 'http://localhost:8080/mcp';
	return [
		{
			id: 'claude-code',
			name: 'Claude Code',
			file: '.mcp.json',
			language: 'json',
			code: json({ mcpServers: { [SERVER_NAME]: { type: 'http', url } } }),
			command: `claude mcp add --transport http --scope project ${SERVER_NAME} ${url}`,
			caption: 'mcp.claude.caption',
			steps: ['mcp.claude.step_1', 'mcp.claude.step_2']
		},
		{
			id: 'vscode',
			name: 'VS Code',
			file: '.mcp.json',
			language: 'json',
			code: json({ mcpServers: { [SERVER_NAME]: { type: 'http', url, tools: ['*'] } } }),
			caption: 'mcp.vscode.caption',
			steps: ['mcp.vscode.step_1', 'mcp.vscode.step_2']
		},
		{
			id: 'opencode',
			name: 'OpenCode',
			file: 'opencode.json',
			language: 'json',
			code: json({
				$schema: 'https://opencode.ai/config.json',
				mcp: { [SERVER_NAME]: { type: 'remote', url, enabled: true } }
			}),
			caption: 'mcp.opencode.caption',
			steps: ['mcp.opencode.step_1', 'mcp.opencode.step_2']
		},
		{
			id: 'cursor',
			name: 'Cursor',
			file: '.cursor/mcp.json',
			language: 'json',
			code: json({ mcpServers: { [SERVER_NAME]: { url } } }),
			caption: 'mcp.cursor.caption',
			steps: ['mcp.cursor.step_1', 'mcp.cursor.step_2']
		},
		{
			id: 'codex',
			name: 'Codex',
			file: '~/.codex/config.toml',
			language: 'toml',
			code: `[mcp_servers.${SERVER_NAME}]\nurl = "${url}"\n`,
			command: `codex mcp add ${SERVER_NAME} --url ${url}`,
			caption: 'mcp.codex.caption',
			steps: ['mcp.codex.step_1', 'mcp.codex.step_2']
		},
		{
			id: 'other',
			name: null,
			file: '.mcp.json',
			language: 'json',
			code: json({ mcpServers: { [SERVER_NAME]: { type: 'http', url } } }),
			caption: 'mcp.other.caption',
			steps: ['mcp.other.step_1', 'mcp.other.step_2']
		}
	];
}

const STORAGE_KEY = 'lp-agent';
const ids: AgentId[] = ['claude-code', 'vscode', 'opencode', 'cursor', 'codex', 'other'];

/** The agent the learner picked, remembered in this browser so every setup panel agrees. */
class AgentChoice {
	current = $state<AgentId>(stored());

	set(id: AgentId): void {
		this.current = id;
		try {
			localStorage.setItem(STORAGE_KEY, id);
		} catch {
			// Private mode: the choice lasts for this page only.
		}
	}
}

function stored(): AgentId {
	try {
		const value = localStorage.getItem(STORAGE_KEY);
		if (ids.includes(value as AgentId)) return value as AgentId;
	} catch {
		// Fall back to the default below.
	}
	return 'claude-code';
}

export const agentChoice = new AgentChoice();
