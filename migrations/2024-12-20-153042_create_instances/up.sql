CREATE TABLE instances
(
    id         INTEGER        NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    actor_id   INTEGER UNIQUE NOT NULL
        REFERENCES actors (id)
            ON UPDATE CASCADE ON DELETE CASCADE,
    is_home    BOOLEAN        NOT NULL DEFAULT false,
    domain     VARCHAR(255)   NOT NULL,
    tls        BOOLEAN        NOT NULL DEFAULT true,
    token      VARCHAR(500),
    created_at TIMESTAMP      NOT NULL,
    updated_at TIMESTAMP      NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX instance_domain ON instances (domain);

