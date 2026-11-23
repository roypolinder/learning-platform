-- Accounts that existed before verification keep NULL until they confirm their address.
ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMPTZ;
CREATE INDEX users_created_at ON users (created_at DESC);

-- Single-use links sent by email. Only a SHA-256 of the secret is stored. `email` is the
-- address the link was sent to: a link stops working once the account's address changes,
-- and for `change_email` it is the new address that becomes the account's email.
CREATE TABLE email_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('verify_email', 'reset_password', 'change_email')),
    email TEXT NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ
);
CREATE INDEX email_tokens_user_purpose ON email_tokens (user_id, purpose);
