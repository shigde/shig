-- Remove only evidence inserted by this migration, retaining actual email verifications.
DELETE FROM verification_tokens v
USING users u
WHERE v.user_id = u.id
  AND v.kind = 'sign_up'
  AND v.token = 'default-account-verified:' || u.user_uuid;
