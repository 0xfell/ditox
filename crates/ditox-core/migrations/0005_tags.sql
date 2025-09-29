-- Tags support: tags table and clip_tags mapping
CREATE TABLE IF NOT EXISTS tags (
  name TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS clip_tags (
  clip_id TEXT NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
  name TEXT NOT NULL REFERENCES tags(name) ON DELETE CASCADE,
  PRIMARY KEY(clip_id, name)
);

CREATE INDEX IF NOT EXISTS idx_clip_tags_name ON clip_tags(name);

