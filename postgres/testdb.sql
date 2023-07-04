CREATE TYPE role AS ENUM ('user', 'admin');

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NULL,
    role role NOT NULL DEFAULT 'user'
);

INSERT INTO users VALUES (DEFAULT, 'first', 'admin');
INSERT INTO users VALUES (DEFAULT, NULL, 'user');
