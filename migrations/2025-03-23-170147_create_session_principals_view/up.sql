CREATE VIEW session_principals_view AS
SELECT u.id,
       u.name,
       u.email,
       u.user_uuid,
       u.user_role_id,
       a.actor_iri    as user_actor,
       chan.actor_iri as channel_actor
FROM users as u
         LEFT JOIN actors as a
                   ON u.actor_id = a.id
         LEFT JOIN (SELECT c.id, c.user_id, ac.actor_iri
                    FROM channels as c
                             LEFT JOIN actors ac
                                       ON c.actor_id = ac.id) as chan
                   ON u.id = chan.user_id
WHERE u.active = true
