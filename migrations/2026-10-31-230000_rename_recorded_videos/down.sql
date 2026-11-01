ALTER INDEX recorded_videos_one_active_recording RENAME TO videos_one_active_recording;
ALTER INDEX recorded_videos_source RENAME TO videos_source;
ALTER INDEX recorded_videos_channel RENAME TO videos_channel;
ALTER INDEX recorded_videos_owner RENAME TO videos_owner;
ALTER TABLE recorded_videos RENAME TO videos;
