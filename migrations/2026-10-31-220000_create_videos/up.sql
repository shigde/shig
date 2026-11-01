CREATE TABLE videos (
    id INTEGER NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    uuid VARCHAR(255) UNIQUE NOT NULL,
    source_stream_id INTEGER REFERENCES streams(id) ON DELETE SET NULL,
    source_stream_uuid VARCHAR(255) NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    title VARCHAR NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    support TEXT NOT NULL DEFAULT '',
    thumbnail VARCHAR NOT NULL DEFAULT '',
    licence INTEGER NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    published_at TIMESTAMP,
    status VARCHAR NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'recording', 'processing', 'ready', 'failed')),
    duration_ms BIGINT NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
    archive_prefix VARCHAR,
    error TEXT,
    started_at TIMESTAMP,
    ended_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    CHECK (published_at IS NULL OR (status = 'ready' AND archive_prefix IS NOT NULL)),
    CHECK (status <> 'ready' OR archive_prefix IS NOT NULL),
    CHECK (ended_at IS NULL OR (started_at IS NOT NULL AND ended_at >= started_at))
);
CREATE INDEX videos_owner ON videos(user_id);
CREATE INDEX videos_channel ON videos(channel_id);
CREATE INDEX videos_source ON videos(source_stream_uuid);
CREATE UNIQUE INDEX videos_one_active_recording ON videos(source_stream_id) WHERE status = 'recording';
SELECT diesel_manage_updated_at('videos');

DROP VIEW stream_previews;
CREATE VIEW stream_previews AS
SELECT str.id, str.title, img.file_url AS thumbnail, str.uuid,
       str.description, str.support, str.date, str.start_time, str.end_time,
       str.is_live, str.is_public, usr.name AS owner_name, usr.user_uuid AS owner_uuid,
       avt.file_url AS owner_avatar, chan.name AS channel_name, chan.uuid AS channel_uuid,
       chan.display_name AS channel_display_name, 'stream'::varchar AS content_type,
       NULL::varchar AS source_stream_uuid, NULL::bigint AS duration_ms
FROM streams str
JOIN channels chan ON str.channel_id = chan.id
JOIN users usr ON str.user_id = usr.id
LEFT JOIN stream_thumbnails img ON str.id = img.stream_id
LEFT JOIN LATERAL (
    SELECT file_url FROM actor_images
    WHERE actor_id = usr.actor_id AND image_type = 'actor' AND file_url IS NOT NULL
    ORDER BY width DESC NULLS LAST, id DESC LIMIT 1
) avt ON true
WHERE usr.active = true
UNION ALL
SELECT -v.id, v.title, v.thumbnail, v.uuid, v.description, v.support,
       COALESCE(v.started_at, v.created_at), v.started_at, v.ended_at,
       false, v.is_public, usr.name, usr.user_uuid, avt.file_url,
       chan.name, chan.uuid, chan.display_name, 'video'::varchar,
       v.source_stream_uuid, v.duration_ms
FROM videos v
JOIN channels chan ON v.channel_id = chan.id
JOIN users usr ON v.user_id = usr.id
LEFT JOIN LATERAL (
    SELECT file_url FROM actor_images
    WHERE actor_id = usr.actor_id AND image_type = 'actor' AND file_url IS NOT NULL
    ORDER BY width DESC NULLS LAST, id DESC LIMIT 1
) avt ON true
WHERE usr.active = true AND v.status = 'ready' AND v.published_at IS NOT NULL AND v.is_public = true;
