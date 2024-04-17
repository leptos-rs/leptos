CREATE TABLE users (
        user_id TEXT PRIMARY KEY,
        identity_id TEXT NOT NULL,
        email TEXT NOT NULL
    );
 CREATE INDEX IF NOT EXISTS idx_identity_id ON users (identity_id);