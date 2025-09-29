-- Add sync-related columns to clips for local-first LWW
ALTER TABLE clips ADD COLUMN updated_at INTEGER;
ALTER TABLE clips ADD COLUMN lamport INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clips ADD COLUMN device_id TEXT;
CREATE INDEX IF NOT EXISTS idx_clips_updated_at ON clips(updated_at);
