-- One row per note typed during a meeting, with its instant (ms from recording start).
-- The older meeting_notes table (one markdown block per meeting) is not used by the app.
CREATE TABLE IF NOT EXISTS meeting_note_entries (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL,
    text TEXT NOT NULL,
    offset_ms INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_meeting_note_entries_meeting ON meeting_note_entries(meeting_id, offset_ms);
