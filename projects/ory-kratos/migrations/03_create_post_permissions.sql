CREATE TABLE IF NOT EXISTS post_permissions (
        post_id TEXT NOT NULL,
        user_id TEXT NOT NULL,
        read BOOL NOT NULL,
        write BOOL NOT NULL,
        `delete` BOOL NOT NULL,
        FOREIGN KEY (user_id) REFERENCES users(user_id),
        FOREIGN KEY (post_id) REFERENCES posts(post_id),
        PRIMARY KEY (post_id, user_id)
    );