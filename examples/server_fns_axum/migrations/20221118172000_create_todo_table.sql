
CREATE TABLE IF NOT EXISTS todos
(
  id          INTEGER NOT NULL PRIMARY KEY,
  title       VARCHAR,
  completed   BOOLEAN
);