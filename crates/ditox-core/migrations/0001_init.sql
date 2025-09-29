-- Ditox schema v1: core tables
CREATE TABLE IF NOT EXISTS clips (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL DEFAULT 'text', -- 'text' | 'image'
  text TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  is_favorite INTEGER NOT NULL DEFAULT 0,
  deleted_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_clips_created_at ON clips(created_at DESC);

-- Image metadata (scaffold for v0.2)
CREATE TABLE IF NOT EXISTS images (
  clip_id TEXT PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
  format TEXT NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  size_bytes INTEGER NOT NULL,
  sha256 TEXT NOT NULL,
  thumb_path TEXT
);
