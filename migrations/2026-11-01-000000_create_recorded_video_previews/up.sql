-- Reuse the shared visibility contract: only public, ready, published videos.
CREATE VIEW recorded_video_previews AS
SELECT * FROM stream_previews WHERE content_type = 'video';
