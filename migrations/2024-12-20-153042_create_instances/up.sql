CREATE TABLE instances
(
    id         INTEGER        NOT NULL PRIMARY KEY AUTOINCREMENT,
    actor_id   INTEGER UNIQUE NOT NULL,
    is_home    BOOLEAN        NOT NULL DEFAULT 0,
    domain     VARCHAR        NOT NULL,
    tls        BOOLEAN        NOT NULL DEFAULT 1,
    token      VARCHAR,
    created_at TIMESTAMP      NOT NULL,
    updated_at TIMESTAMP,
    FOREIGN KEY (actor_id) REFERENCES actors (id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX index_instances_actor_id ON instances (actor_id);
