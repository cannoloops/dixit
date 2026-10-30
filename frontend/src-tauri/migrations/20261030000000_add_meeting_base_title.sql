-- Dixit: the meeting's name before the AI title was appended, so a new summary replaces it instead of piling up.
-- NULL until the first AI title, and again after a manual rename (the new name becomes the base).
ALTER TABLE meetings ADD COLUMN base_title TEXT;
