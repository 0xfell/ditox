-- Upgrade timestamp precision to nanoseconds for created_at/updated_at/last_used_at/deleted_at
-- Idempotent conversion: multiply seconds by 1_000_000_000 only when value < 10^12.

UPDATE clips SET created_at = CASE WHEN ABS(created_at) < 1000000000000 THEN created_at * 1000000000 ELSE created_at END;
UPDATE clips SET updated_at = CASE WHEN updated_at IS NOT NULL AND ABS(updated_at) < 1000000000000 THEN updated_at * 1000000000 ELSE updated_at END;
UPDATE clips SET last_used_at = CASE WHEN last_used_at IS NOT NULL AND ABS(last_used_at) < 1000000000000 THEN last_used_at * 1000000000 ELSE last_used_at END;
UPDATE clips SET deleted_at = CASE WHEN deleted_at IS NOT NULL AND ABS(deleted_at) < 1000000000000 THEN deleted_at * 1000000000 ELSE deleted_at END;

-- Refresh helpful indices
DROP INDEX IF EXISTS idx_clips_created_at;
CREATE INDEX IF NOT EXISTS idx_clips_created_at ON clips(created_at DESC);
DROP INDEX IF EXISTS idx_clips_updated_at;
CREATE INDEX IF NOT EXISTS idx_clips_updated_at ON clips(updated_at);
DROP INDEX IF EXISTS idx_clips_last_used_at;
CREATE INDEX IF NOT EXISTS idx_clips_last_used_at ON clips(last_used_at DESC);
DROP INDEX IF EXISTS idx_clips_recency;
CREATE INDEX IF NOT EXISTS idx_clips_recency
  ON clips(COALESCE(last_used_at, created_at) DESC)
  WHERE deleted_at IS NULL;

