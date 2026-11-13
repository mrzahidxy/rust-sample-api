CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_hash TEXT NOT NULL UNIQUE,
    refresh_expires_at BIGINT NOT NULL,
    access_jti UUID NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX auth_sessions_user_idx ON auth_sessions(user_id);
