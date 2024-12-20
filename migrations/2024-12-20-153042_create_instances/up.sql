CREATE TABLE instances
(
    id         INTEGER        NOT NULL PRIMARY KEY AUTOINCREMENT,
    actor_id   INTEGER UNIQUE NOT NULL,
    created_at TIMESTAMP      NOT NULL,
    updated_at TIMESTAMP,
    FOREIGN KEY (actor_id) REFERENCES actors (id) ON DELETE CASCADE ON UPDATE CASCADE
);
