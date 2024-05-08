-- The database initialization script is used for defining your local schema as well as postgres
-- running within a docker container, where we'll copy this file over and run on startup

DO
$$
    BEGIN
        IF
            NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'blogs') THEN
            CREATE DATABASE blogs;
        END IF;
    END
$$;

\c blogs;

DROP TABLE IF EXISTS posts;
CREATE TABLE posts
(
    id SERIAL PRIMARY KEY,
    slug VARCHAR(255) UNIQUE NOT NULL,
    title VARCHAR(255) NOT NULL,
    content VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO posts (slug, title, content)
VALUES ('first-post', 'First Post', 'This is the content of the first post.'),
       ('second-post', 'Second Post', 'Here is some more content for another post.'),
       ('hello-world', 'Hello World', 'Yet another post to add to our collection.'),
       ('tech-talk', 'Tech Talk', 'Discussing the latest in technology.'),
       ('travel-diaries', 'Travel Diaries', 'Sharing my experiences traveling around the world.');