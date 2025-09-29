-- Add local-only image columns on clips
ALTER TABLE clips ADD COLUMN is_image INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clips ADD COLUMN image_path TEXT;
-- Index for quick image queries
CREATE INDEX IF NOT EXISTS idx_clips_is_image ON clips(is_image);
-- Backfill existing image rows
UPDATE clips SET is_image = 1 WHERE kind = 'image' AND is_image = 0;
