CREATE TABLE users
(
    id           INTEGER        NOT NULL PRIMARY KEY,
    name         VARCHAR        NOT NULL,
    email        VARCHAR UNIQUE NOT NULL,
    uuid         VARCHAR UNIQUE NOT NULL,
    user_role_id INTEGER        NOT NULL,
    password     VARCHAR        NOT NULL,
    active       BOOLEAN        NOT NULL DEFAULT 0,
    actor_id     INTEGER UNIQUE NOT NULL,
    created_at   TIMESTAMP      NOT NULL,
    updated_at   TIMESTAMP,
    FOREIGN KEY (actor_id) REFERENCES actors (id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (user_role_id) REFERENCES user_roles (id)
)
