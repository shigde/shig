CREATE VIEW active_users AS
SELECT u.id,
       u.name,
       u.email,
       u.user_uuid,
       u.user_role_id,
       u.actor_id    as user_actor_id,
       chan.id       as channel_id,
       chan.uuid     as channel_uuid,
       chan.actor_id as channel_actor_id,
       img.file_url  as avatar
FROM users as u
         LEFT JOIN channels as chan
                   ON u.id = chan.user_id
         LEFT JOIN actor_images as img
                   ON u.actor_id = img.actor_id
WHERE u.active = true
