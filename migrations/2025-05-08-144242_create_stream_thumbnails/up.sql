-- auto-generated definition
create table stream_thumbnails
(
    id         INTEGER        NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    filename   VARCHAR(255)   NOT NULL,
    height     INTEGER,
    width      INTEGER,
    file_url   VARCHAR(255),
    on_disk    BOOLEAN        NOT NULL,
    stream_id  INTEGER UNIQUE NOT NULL
        REFERENCES streams (id)
            ON UPDATE CASCADE ON DELETE CASCADE,
    created_at timestamp      NOT NULL,
    updated_at timestamp      NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX index_stream_thumbnails_stream ON stream_thumbnails (stream_id);


