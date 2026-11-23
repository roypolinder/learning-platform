-- Warnings, errors and notable events (emails sent, account changes) for super admins.
-- May contain email addresses; rows older than 30 days are deleted by the API.
CREATE TABLE system_logs (
    id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    level TEXT NOT NULL CHECK (level IN ('error', 'warn', 'info')),
    target TEXT NOT NULL,
    category TEXT,
    message TEXT NOT NULL,
    fields JSONB NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX system_logs_created_at ON system_logs (created_at DESC);
CREATE INDEX system_logs_level_created_at ON system_logs (level, created_at DESC);
