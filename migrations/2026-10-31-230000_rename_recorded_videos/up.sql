-- Preserve existing rows and references if the initial video migration was applied.
-- PostgreSQL keeps the preview view's table dependency through this rename.
ALTER TABLE videos RENAME TO recorded_videos;
ALTER INDEX videos_owner RENAME TO recorded_videos_owner;
ALTER INDEX videos_channel RENAME TO recorded_videos_channel;
ALTER INDEX videos_source RENAME TO recorded_videos_source;
ALTER INDEX videos_one_active_recording RENAME TO recorded_videos_one_active_recording;
