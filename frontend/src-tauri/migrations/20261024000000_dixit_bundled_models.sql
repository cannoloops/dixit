-- Dixit: summaries on the bundled Gemma 4 E4B (llama-helper) instead of Ollama or a downloaded built-in model.
-- Cloud providers chosen with an API key are left alone.
UPDATE settings SET provider = 'builtin-ai', model = 'gemma4:e4b'
WHERE provider IN ('ollama', 'builtin-ai');

-- Onboarding used to set Parakeet only: move to Parakeet live + Whisper large-v3 final.
UPDATE transcript_settings SET provider = 'localWhisper', model = 'large-v3'
WHERE provider = 'parakeet';
