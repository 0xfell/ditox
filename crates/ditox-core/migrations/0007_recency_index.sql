-- Optimize recency ordering by indexing the expression used in ORDER BY
-- This helps queries that sort by COALESCE(last_used_at, created_at) DESC
CREATE INDEX IF NOT EXISTS idx_clips_recency
  ON clips(COALESCE(last_used_at, created_at) DESC)
  WHERE deleted_at IS NULL;

