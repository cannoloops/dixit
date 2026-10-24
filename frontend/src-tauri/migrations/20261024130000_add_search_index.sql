-- Search index over meetings (rebuilt by search::sync, never the source of truth).
-- A chunk is a slice of transcript, one note, or a slice of the summary.
CREATE TABLE IF NOT EXISTS search_chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    kind TEXT NOT NULL,          -- transcript | note | summary
    start_ms INTEGER,            -- offset from recording start (NULL for the summary)
    text TEXT NOT NULL,
    embedding BLOB               -- f32 little-endian, unit length; NULL until embedded
);
CREATE INDEX IF NOT EXISTS idx_search_chunks_meeting ON search_chunks(meeting_id);

-- Exact search: rowid = search_chunks.id; accents folded ("perche" finds "perché").
CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(text, tokenize = 'unicode61 remove_diacritics 2');
CREATE TRIGGER IF NOT EXISTS search_chunks_ai AFTER INSERT ON search_chunks BEGIN
    INSERT INTO search_fts(rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER IF NOT EXISTS search_chunks_ad AFTER DELETE ON search_chunks BEGIN
    DELETE FROM search_fts WHERE rowid = old.id;
END;

-- What each meeting looked like when it was indexed: a different stamp means reindex.
CREATE TABLE IF NOT EXISTS search_index_state (
    meeting_id TEXT PRIMARY KEY NOT NULL,
    stamp TEXT NOT NULL
);
