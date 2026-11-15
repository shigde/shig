DROP VIEW recorded_video_previews;
DROP VIEW stream_previews;

CREATE VIEW stream_previews AS
SELECT str.id, str.title, img.file_url AS thumbnail, str.uuid,
       str.description, str.support, str.date, str.start_time, str.end_time,
       str.is_live, str.is_public, usr.name AS owner_name, usr.user_uuid AS owner_uuid,
       avt.file_url AS owner_avatar, chan.name AS channel_name, chan.uuid AS channel_uuid,
       chan.display_name AS channel_display_name, 'stream'::varchar AS content_type,
       NULL::varchar AS source_stream_uuid, NULL::bigint AS duration_ms, 0::bigint AS view_count,
       COALESCE(likes.count, 0)::bigint AS like_count
FROM streams str
JOIN channels chan ON str.channel_id = chan.id
JOIN users usr ON str.user_id = usr.id
LEFT JOIN stream_thumbnails img ON str.id = img.stream_id
LEFT JOIN LATERAL (
    SELECT file_url FROM actor_images
    WHERE actor_id = usr.actor_id AND image_type = 'actor' AND file_url IS NOT NULL
    ORDER BY width DESC NULLS LAST, id DESC LIMIT 1
) avt ON true
LEFT JOIN LATERAL (SELECT COUNT(*) AS count FROM stream_likes WHERE stream_id = str.id) likes ON true
WHERE usr.active = true
UNION ALL
SELECT -v.id, v.title, v.thumbnail, v.uuid, v.description, v.support,
       COALESCE(v.started_at, v.created_at), v.started_at, v.ended_at,
       false, v.is_public, usr.name, usr.user_uuid, avt.file_url,
       chan.name, chan.uuid, chan.display_name, 'video'::varchar,
       v.source_stream_uuid, v.duration_ms, v.view_count,
       COALESCE(likes.count, 0)::bigint
FROM recorded_videos v
JOIN channels chan ON v.channel_id = chan.id
JOIN users usr ON v.user_id = usr.id
LEFT JOIN LATERAL (
    SELECT file_url FROM actor_images
    WHERE actor_id = usr.actor_id AND image_type = 'actor' AND file_url IS NOT NULL
    ORDER BY width DESC NULLS LAST, id DESC LIMIT 1
) avt ON true
LEFT JOIN LATERAL (SELECT COUNT(*) AS count FROM recorded_video_likes WHERE recorded_video_id = v.id) likes ON true
WHERE usr.active = true AND v.status = 'ready' AND v.published_at IS NOT NULL AND v.is_public = true;

CREATE VIEW recorded_video_previews AS
SELECT * FROM stream_previews WHERE content_type = 'video';
