import { ApiError } from '#lib/services/api/client.ts';
import { integrationsApi } from '#lib/services/api/integrations.ts';
import type { IntegrationConnection } from '#lib/types/api.ts';

export class McpConnections {
	connections = $state<IntegrationConnection[]>([]);
	endpoint = $state('');
	loading = $state(true);
	busy = $state(false);
	error = $state<string | null>(null);

	readonly active = $derived(this.connections.filter((c) => !c.revoked_at));
	readonly connected = $derived(this.active.find((c) => c.status === 'connected') ?? null);

	async load(): Promise<void> {
		try {
			const response = await integrationsApi.list();
			this.connections = response.connections;
			this.endpoint = response.mcp_endpoint;
		} catch (error) {
			this.error = error instanceof ApiError ? error.message : 'Could not load connections.';
		} finally {
			this.loading = false;
		}
	}

	async revoke(id: string): Promise<void> {
		this.busy = true;
		try {
			await integrationsApi.revoke(id);
			await this.load();
		} finally {
			this.busy = false;
		}
	}
}
