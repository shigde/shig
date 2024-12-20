CREATE TABLE users
(
    id         INTEGER        NOT NULL PRIMARY KEY AUTOINCREMENT,
    name       TEXT           NOT NULL,
    email      TEXT UNIQUE    NOT NULL,
    uuid       TEXT UNIQUE    NOT NULL,
    role_id    INTEGER        NOT NULL,
    password   TEXT           NOT NULL,
    active     BOOLEAN        NOT NULL DEFAULT 0,
    actor_id   INTEGER UNIQUE NOT NULL,
    created_at TIMESTAMP      NOT NULL,
    updated_at TIMESTAMP,
    FOREIGN KEY (actor_id) REFERENCES actors (id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (role_id) REFERENCES user_roles (id)
)
