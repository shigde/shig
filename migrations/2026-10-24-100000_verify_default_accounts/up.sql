-- Backfill only the two accounts provisioned by create_standard_users on the home instance.
-- Keep active/password/roles unchanged: verification must not undo an admin's block.
INSERT INTO verification_tokens (user_id, kind, token, verified, created_at, updated_at)
SELECT u.id, 'sign_up', 'default-account-verified:' || u.user_uuid, TRUE, now(), now()
FROM users u
JOIN actors a ON a.id = u.actor_id
JOIN instances i ON i.id = a.instance_id AND i.is_home = TRUE
WHERE (
    (u.name = 'admin@' || i.domain AND u.user_role_id = 1
        AND u.email = 'admin@' || CASE WHEN i.domain LIKE 'localhost%' THEN 'localhost.de' ELSE i.domain END)
    OR
    (u.name = 'user@' || i.domain AND u.user_role_id = 2
        AND u.email = 'user@' || CASE WHEN i.domain LIKE 'localhost%' THEN 'localhost.de' ELSE i.domain END)
)
AND NOT EXISTS (
    SELECT 1 FROM verification_tokens v
    WHERE v.user_id = u.id AND v.kind = 'sign_up' AND v.verified = TRUE
)
ON CONFLICT (token) DO NOTHING;
