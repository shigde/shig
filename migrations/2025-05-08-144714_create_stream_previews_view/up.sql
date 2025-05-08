CREATE VIEW stream_previews AS
SELECT str.id,
       str.title,
       img.file_url  as thumbnail,
       str.uuid,
       str.description,
       str.support,
       str.date,
       str.start_time,
       str.end_time,
       str.is_live,
       str.is_public,
       usr.name      as owner_name,
       usr.user_uuid as owner_uuid,
       avt.file_url  as owner_avatar,
       chan.name     as channel_name,
       chan.uuid     as channel_uuid
FROM streams as str
         LEFT JOIN channels as chan
                   ON str.user_id = chan.user_id
         LEFT JOIN users as usr
                   ON str.user_id = usr.id
         LEFT JOIN stream_thumbnails as img
                   ON str.id = img.stream_id
         LEFT JOIN actor_images as avt
                   ON usr.actor_id = avt.actor_id
WHERE usr.active = true;
