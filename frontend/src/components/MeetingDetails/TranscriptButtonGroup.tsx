"use client";

import { useState, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, FolderOpen, RefreshCw } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { RetranscribeDialog } from './RetranscribeDialog';
import { useConfig } from '@/contexts/ConfigContext';
import { t } from '@/i18n/meeting';


interface TranscriptButtonGroupProps {
  transcriptCount: number;
  onCopyTranscript: () => void;
  onOpenMeetingFolder: () => Promise<void>;
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
  hasNotes?: boolean;
  includeNotes?: boolean;
  onIncludeNotesChange?: (value: boolean) => void;
}


export function TranscriptButtonGroup({
  transcriptCount,
  onCopyTranscript,
  onOpenMeetingFolder,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
  hasNotes = false,
  includeNotes = true,
  onIncludeNotesChange,
}: TranscriptButtonGroupProps) {
  const { betaFeatures } = useConfig();
  const [showRetranscribeDialog, setShowRetranscribeDialog] = useState(false);
  const canRetranscribe = betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath;

  const handleRetranscribeComplete = useCallback(async () => {
    // Refetch transcripts to show the updated data
    if (onRefetchTranscripts) {
      await onRefetchTranscripts();
    }
  }, [onRefetchTranscripts]);

  return (
    <>
      {hasNotes && onIncludeNotesChange && (
        <label className="mr-3 inline-flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground">
          <input
            type="checkbox"
            className="accent-primary"
            checked={includeNotes}
            onChange={(e) => onIncludeNotesChange(e.target.checked)}
          />
          <span className="hidden @[28rem]:inline">{t('includeNotes')}</span>
        </label>
      )}
      <ButtonGroup>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            Analytics.trackButtonClick('copy_transcript', 'meeting_details');
            onCopyTranscript();
          }}
          disabled={transcriptCount === 0}
          title={transcriptCount === 0 ? t('noTranscript') : t('copyTranscript')}
        >
          <Copy />
          <span className="hidden @[28rem]:inline">{t('copy')}</span>
        </Button>

        <Button
          size="sm"
          variant="outline"
          onClick={() => {
            Analytics.trackButtonClick('open_recording_folder', 'meeting_details');
            onOpenMeetingFolder();
          }}
          title={t('openFolder')}
        >
          <FolderOpen />
          <span className="hidden @[28rem]:inline">{t('folder')}</span>
        </Button>

        {canRetranscribe && (
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              Analytics.trackButtonClick('enhance_transcript', 'meeting_details');
              setShowRetranscribeDialog(true);
            }}
            title={t('retranscribeTitle')}
          >
            <RefreshCw />
            <span className="hidden @[28rem]:inline">{t('retranscribe')}</span>
          </Button>
        )}
      </ButtonGroup>

      {canRetranscribe && (
        <RetranscribeDialog
          open={showRetranscribeDialog}
          onOpenChange={setShowRetranscribeDialog}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onComplete={handleRetranscribeComplete}
        />
      )}
    </>
  );
}
