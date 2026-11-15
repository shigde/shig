-- Repair databases where the streaming-policy migration was recorded before
-- public_content_enabled was added to its original migration file.
ALTER TABLE instance_streaming_policy
    ADD COLUMN IF NOT EXISTS public_content_enabled BOOLEAN NOT NULL DEFAULT TRUE;
