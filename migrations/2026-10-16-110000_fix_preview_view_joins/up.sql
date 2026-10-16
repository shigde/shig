CREATE OR REPLACE VIEW active_users AS
SELECT u.id,
       u.name,
       u.email,
       u.user_uuid,
       u.user_role_id,
       u.actor_id AS user_actor_id,
       chan.id AS channel_id,
       chan.uuid AS channel_uuid,
       chan.actor_id AS channel_actor_id,
       img.file_url AS avatar
FROM users AS u
LEFT JOIN channels AS chan ON u.id = chan.user_id
LEFT JOIN LATERAL (
    SELECT file_url
    FROM actor_images
    WHERE actor_id = u.actor_id AND image_type = 'actor' AND file_url IS NOT NULL
    ORDER BY width DESC NULLS LAST, id DESC
    LIMIT 1
) AS img ON true
WHERE u.active = true;

CREATE OR REPLACE VIEW stream_previews AS
SELECT str.id,
       str.title,
       img.file_url AS thumbnail,
       str.uuid,
       str.description,
       str.support,
       str.date,
       str.start_time,
       str.end_time,
       str.is_live,
       str.is_public,
       usr.name AS owner_name,
       usr.user_uuid AS owner_uuid,
       avt.file_url AS owner_avatar,
       chan.name AS channel_name,
       chan.uuid AS channel_uuid
FROM streams AS str
LEFT JOIN channels AS chan ON str.channel_id = chan.id
LEFT JOIN users AS usr ON str.user_id = usr.id
LEFT JOIN stream_thumbnails AS img ON str.id = img.stream_id
LEFT JOIN LATERAL (
    SELECT file_url
    FROM actor_images
    WHERE actor_id = usr.actor_id AND image_type = 'actor' AND file_url IS NOT NULL
    ORDER BY width DESC NULLS LAST, id DESC
    LIMIT 1
) AS avt ON true
WHERE usr.active = true;
