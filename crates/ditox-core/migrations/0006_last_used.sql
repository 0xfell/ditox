-- Add last_used_at to track when a clip was last selected/copied
ALTER TABLE clips ADD COLUMN last_used_at INTEGER NULL;

-- Future sort support on last_used_at
CREATE INDEX IF NOT EXISTS idx_clips_last_used_at ON clips(last_used_at DESC);

