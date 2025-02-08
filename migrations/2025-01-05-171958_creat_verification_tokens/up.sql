CREATE TABLE verification_tokens
(
    id         INTEGER             NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    user_id    INTEGER             NOT NULL
        REFERENCES users (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    kind       VARCHAR(255)        NOT NULL,
    token      VARCHAR(255) UNIQUE NOT NULL,
    verified   BOOLEAN             NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP           NOT NULL,
    updated_at TIMESTAMP           NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX index_verification_tokens_token ON verification_tokens (token);
