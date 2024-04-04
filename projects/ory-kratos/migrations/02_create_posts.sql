CREATE TABLE IF NOT EXISTS posts (
        post_id TEXT PRIMARY KEY NOT NULL,
        user_id TEXT NOT NULL,
        content TEXT NOT NULL,
        FOREIGN KEY (user_id) REFERENCES users(user_id)
    );