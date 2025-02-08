CREATE TABLE users
(
    id           INTEGER             NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name         varchar(255)        NOT NULL,
    email        varchar(255) UNIQUE NOT NULL,
    user_uuid    varchar(255) UNIQUE NOT NULL,
    user_role_id INTEGER             NOT NULL
        REFERENCES user_roles (id),
    password     varchar(255)        NOT NULL,
    active       BOOLEAN             NOT NULL DEFAULT FALSE,
    actor_id     INTEGER UNIQUE      NOT NULL
        REFERENCES actors (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    created_at   TIMESTAMP           NOT NULL,
    updated_at   TIMESTAMP           NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX index_user_name ON users (name);
CREATE UNIQUE INDEX index_user_email ON users (email);
CREATE UNIQUE INDEX index_user_user_uuid ON users (user_uuid);
CREATE UNIQUE INDEX index_user_actor_id ON users (actor_id);
