import { useCallback } from 'react';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { t } from '@/i18n/meeting';

interface UseMeetingOperationsProps {
  meeting: any;
}

export function useMeetingOperations({
  meeting,
}: UseMeetingOperationsProps) {

  // Open meeting folder in file explorer
  const handleOpenMeetingFolder = useCallback(async () => {
    try {
      await invokeTauri('open_meeting_folder', { meetingId: meeting.id });
    } catch (error) {
      console.error('Failed to open meeting folder:', error);
      toast.error(t('openFolderFailed'), { description: error ? String(error) : undefined });
    }
  }, [meeting.id]);

  return {
    handleOpenMeetingFolder,
  };
}
