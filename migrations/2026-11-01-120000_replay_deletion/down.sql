UPDATE recorded_videos SET status='failed', error='Permanent deletion incomplete' WHERE status='deleting';
ALTER TABLE recorded_videos DROP CONSTRAINT recorded_videos_status_check;
ALTER TABLE recorded_videos ADD CONSTRAINT videos_status_check
    CHECK (status IN ('pending', 'recording', 'processing', 'ready', 'failed'));
