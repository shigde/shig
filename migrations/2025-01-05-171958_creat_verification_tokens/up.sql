CREATE TABLE verification_tokens
(
    id         INTEGER        NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER UNIQUE NOT NULL,
    kind       VARCHAR        NOT NULL,
    token      VARCHAR UNIQUE NOT NULL,
    verified   BOOLEAN        NOT NULL DEFAULT 0,
    created_at TIMESTAMP      NOT NULL,
    updated_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX index_verification_tokens_token ON verification_tokens (token);
