ALTER TABLE notifications ADD COLUMN action_path TEXT;

-- Marked notifications are historical clutter; keep only unread entries.
DELETE FROM notifications WHERE read_at IS NOT NULL;
