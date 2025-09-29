-- Ditox schema v2: FTS5 virtual table for text search
CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
  text,
  content='clips',
  content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS clips_fts_ai AFTER INSERT ON clips BEGIN
  INSERT INTO clips_fts(rowid, text) VALUES (new.rowid, new.text);
END;
CREATE TRIGGER IF NOT EXISTS clips_fts_ad AFTER DELETE ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
END;
CREATE TRIGGER IF NOT EXISTS clips_fts_au AFTER UPDATE OF text ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
  INSERT INTO clips_fts(rowid, text) VALUES (new.rowid, new.text);
END;
-- Rebuild in case data existed prior to FTS
INSERT INTO clips_fts(clips_fts) VALUES('rebuild');
