-- Dixit: meeting type (sal, analisi, mix, altro) that picks the summary template.
-- meeting_type_reason is the classifier's motivation; NULL when the user chose the type.
ALTER TABLE meetings ADD COLUMN meeting_type TEXT;
ALTER TABLE meetings ADD COLUMN meeting_type_reason TEXT;
