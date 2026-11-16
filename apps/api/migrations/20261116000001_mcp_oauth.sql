-- Clients created through OAuth dynamic client registration (RFC 7591). All are public
-- clients using PKCE, so no client secret exists.
CREATE TABLE oauth_clients (
    id UUID PRIMARY KEY,
    client_name TEXT NOT NULL,
    redirect_uris TEXT[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT oauth_clients_redirect_uris_present CHECK (cardinality(redirect_uris) > 0)
);

CREATE TABLE oauth_authorization_codes (
    id UUID PRIMARY KEY,
    code_hash BYTEA NOT NULL UNIQUE,
    client_id UUID NOT NULL REFERENCES oauth_clients (id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    redirect_uri TEXT NOT NULL,
    code_challenge TEXT NOT NULL,
    scope TEXT NOT NULL,
    -- Set on exchange; a replayed code revokes the connection it produced.
    connection_id UUID REFERENCES integration_connections (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ
);
CREATE INDEX oauth_authorization_codes_expiry ON oauth_authorization_codes (expires_at);

-- OAuth grants are stored as MCP connections so they share listing, revocation and
-- activity tracking with manually created tokens. `expires_at` is the grant lifetime;
-- `access_expires_at` limits the current access token.
ALTER TABLE integration_connections
    ADD COLUMN auth_method TEXT NOT NULL DEFAULT 'token'
        CHECK (auth_method IN ('token', 'oauth')),
    ADD COLUMN oauth_client_id UUID REFERENCES oauth_clients (id) ON DELETE CASCADE,
    ADD COLUMN refresh_token_hash BYTEA UNIQUE,
    ADD COLUMN access_expires_at TIMESTAMPTZ,
    ADD CONSTRAINT integration_connections_oauth_client
        CHECK ((auth_method = 'oauth') = (oauth_client_id IS NOT NULL));
