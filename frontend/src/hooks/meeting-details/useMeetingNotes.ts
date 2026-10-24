import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { MeetingNote } from '@/lib/notes';
import { t } from '@/i18n/recording';

/** Notes of a saved meeting, with edit and delete (the backend also rewrites notes.json and the .md). */
export function useMeetingNotes(meetingId: string) {
  const [notes, setNotes] = useState<MeetingNote[]>([]);

  const refresh = useCallback(async () => {
    try {
      setNotes(await invoke<MeetingNote[]>('meeting_notes_list', { meetingId }));
    } catch (error) {
      console.error('Failed to load notes:', error);
    }
  }, [meetingId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const edit = useCallback(async (id: string, text: string) => {
    try {
      await invoke('meeting_note_update', { id, text });
      await refresh();
    } catch (error) {
      toast.error(t('noteSaveFailed'), { description: String(error) });
    }
  }, [refresh]);

  const remove = useCallback(async (id: string) => {
    try {
      await invoke('meeting_note_delete', { id });
      await refresh();
    } catch (error) {
      toast.error(t('noteSaveFailed'), { description: String(error) });
    }
  }, [refresh]);

  return { notes, edit, remove };
}
