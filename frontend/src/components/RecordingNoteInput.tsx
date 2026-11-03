'use client';

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Input } from '@/components/ui/input';
import { t } from '@/i18n/recording';

/** Always-visible note field while recording: Enter saves the note at the current instant. */
export function RecordingNoteInput() {
  const [text, setText] = useState('');

  const save = async () => {
    const note = text.trim();
    if (!note) return;
    setText('');
    try {
      await invoke('recording_note_add', { text: note });
    } catch (error) {
      setText(note);
      toast.error(t('noteSaveFailed'), { description: String(error) });
    }
  };

  return (
    <Input
      value={text}
      onChange={(e) => setText(e.target.value)}
      onKeyDown={(e) => {
        // Enter confirming an IME composition must not save.
        if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
          e.preventDefault();
          void save();
        }
      }}
      aria-label={t('noteInputLabel')}
      placeholder={t('notePlaceholder')}
      className="w-[min(32rem,calc(100vw-3rem))] shadow-(--shadow-float)"
    />
  );
}
