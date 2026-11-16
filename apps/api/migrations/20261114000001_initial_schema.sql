CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT users_email_length CHECK (char_length(email) BETWEEN 3 AND 254)
);
CREATE UNIQUE INDEX users_email_unique ON users (lower(email));

-- Browser sessions. Only a SHA-256 of the bearer secret is stored.
CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);
CREATE INDEX auth_sessions_user ON auth_sessions (user_id);

CREATE TABLE courses (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    category TEXT NOT NULL,
    difficulty TEXT NOT NULL,
    current_version_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Definitions are immutable per version so active sessions can always be replayed
-- against the exact content they started with.
CREATE TABLE course_versions (
    id UUID PRIMARY KEY,
    course_id UUID NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    checksum TEXT NOT NULL,
    definition JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (course_id, version)
);

ALTER TABLE courses
    ADD CONSTRAINT courses_current_version_fk
    FOREIGN KEY (current_version_id) REFERENCES course_versions (id);

CREATE TABLE course_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    course_id UUID NOT NULL REFERENCES courses (id),
    course_version_id UUID NOT NULL REFERENCES course_versions (id),
    status TEXT NOT NULL CHECK (status IN ('active', 'completed', 'abandoned')),
    current_step_id TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX course_sessions_single_active
    ON course_sessions (user_id, course_id) WHERE status = 'active';
CREATE INDEX course_sessions_user_recent ON course_sessions (user_id, last_accessed_at DESC);

CREATE TABLE step_progress (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES course_sessions (id) ON DELETE CASCADE,
    step_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('locked', 'available', 'in_progress', 'completed', 'failed', 'skipped')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    score DOUBLE PRECISION,
    last_result JSONB,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, step_id)
);

-- Append-only audit trail of state transitions, validations and integration activity.
CREATE TABLE session_events (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES course_sessions (id) ON DELETE CASCADE,
    step_id TEXT,
    kind TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX session_events_session_time ON session_events (session_id, created_at DESC);

CREATE TABLE extension_registrations (
    id UUID PRIMARY KEY,
    extension_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    manifest JSONB NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE integration_connections (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('mcp')),
    label TEXT NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE,
    token_hint TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'connected', 'revoked')),
    client_name TEXT,
    last_session_id UUID REFERENCES course_sessions (id) ON DELETE SET NULL,
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);
CREATE INDEX integration_connections_user ON integration_connections (user_id, created_at DESC);
