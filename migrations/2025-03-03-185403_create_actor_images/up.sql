-- auto-generated definition
create table actor_images
(
    id         INTEGER      NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    filename   VARCHAR(255) NOT NULL,
    height     INTEGER,
    width      INTEGER,
    file_url   VARCHAR(255),
    on_disk    BOOLEAN      NOT NULL,
    image_type VARCHAR(40)  NOT NULL,
    actor_id   INTEGER      NOT NULL
        REFERENCES actors (id)
            ON UPDATE CASCADE ON DELETE CASCADE,
    created_at timestamp    NOT NULL,
    updated_at timestamp    NOT NULL DEFAULT now()
);



create index actor_image_filename on actor_images (filename);

create index actor_image_actor_id_type_width on actor_images (actor_id, image_type, width);


