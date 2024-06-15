CREATE TABLE IF NOT EXISTS users (
  id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  username   TEXT NOT NULL UNIQUE,
  password   TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS user_permissions (
    user_id  INTEGER NOT NULL,
    token    TEXT NOT NULL
);

-- INSERT INTO users (id, anonymous, username, password) 
-- SELECT 0, true, 'Guest', ''
-- ON CONFLICT(id) DO UPDATE SET
--     anonymous = EXCLUDED.anonymous,
--     username = EXCLUDED.username;


CREATE TABLE IF NOT EXISTS todos (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id    INTEGER NOT NULL,
  title      TEXT NOT NULL,
  completed  BOOLEAN,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
  -- FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);
