import { VirtualizedTranscriptView } from '@/components/VirtualizedTranscriptView';
import { PermissionWarning } from '@/components/PermissionWarning';
import { Button } from '@/components/ui/button';
import { Copy, Globe } from 'lucide-react';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useConfig } from '@/contexts/ConfigContext';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { usePermissionCheck } from '@/hooks/usePermissionCheck';
import { ModalType } from '@/hooks/useModalState';
import { useIsLinux } from '@/hooks/usePlatform';
import { useMemo } from 'react';
import { t } from '@/i18n/recording';

/**
 * TranscriptPanel Component
 *
 * The live transcript: a fixed-measure column that grows like an inscription while Dixit records,
 * with a teaching empty state before the first recording.
 */

interface TranscriptPanelProps {
  // indicates stop-processing state for transcripts; derived from backend statuses.
  isProcessingStop: boolean;
  isStopping: boolean;
  showModal: (name: ModalType, message?: string) => void;
}

export function TranscriptPanel({
  isProcessingStop,
  isStopping,
  showModal
}: TranscriptPanelProps) {
  // Contexts
  const { transcripts, transcriptContainerRef, copyTranscript, meetingTitle } = useTranscripts();
  const { transcriptModelConfig, showConfidenceIndicator } = useConfig();
  const { isRecording, isPaused } = useRecordingState();
  const { checkPermissions, isChecking, hasSystemAudio, hasMicrophone } = usePermissionCheck();
  const isLinux = useIsLinux();

  // Convert transcripts to segments for virtualized view
  const segments = useMemo(() =>
    transcripts.map(t => ({
      id: t.id,
      timestamp: t.audio_start_time ?? 0,
      endTime: t.audio_end_time,
      text: t.text,
      confidence: t.confidence,
      speaker: t.speaker,
    })),
    [transcripts]
  );

  const canPickLanguage = transcriptModelConfig.provider === 'localWhisper';
  const showToolbar = isRecording || transcripts.length > 0 || canPickLanguage;
  const showEmptyState = !isRecording && transcripts.length === 0;

  return (
    <div ref={transcriptContainerRef} className="flex w-full flex-col overflow-y-auto bg-background">
      {showToolbar && (
        <div className="sticky top-0 z-10 flex h-11 shrink-0 items-center gap-3 border-b border-border bg-background px-6">
          {isRecording && (
            <h1 className="incisa min-w-0 truncate text-[13px] leading-5">{meetingTitle}</h1>
          )}
          <div className="ml-auto flex items-center gap-1">
            {transcripts.length > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={copyTranscript}
                title={t('copyTranscript')}
                aria-label={t('copyTranscript')}
              >
                <Copy />
                <span className="hidden md:inline">{t('copy')}</span>
              </Button>
            )}
            {canPickLanguage && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => showModal('languageSettings')}
                title={t('transcriptionLanguage')}
                aria-label={t('transcriptionLanguage')}
              >
                <Globe />
                <span className="hidden md:inline">{t('language')}</span>
              </Button>
            )}
          </div>
        </div>
      )}

      {/* Permission Warning - Not needed on Linux */}
      {!isRecording && !isChecking && !isLinux && (
        <div className="flex justify-center px-6 pt-4">
          <PermissionWarning
            hasMicrophone={hasMicrophone}
            hasSystemAudio={hasSystemAudio}
            onRecheck={checkPermissions}
            isRechecking={isChecking}
          />
        </div>
      )}

      {showEmptyState ? (
        // Teaching empty state: Dixit records calls on its own; the manual start is the exception.
        <section className="mx-auto w-full max-w-[60ch] px-6 pb-32 pt-16 text-sm">
          <h2 className="text-base font-semibold">{t('emptyTitle')}</h2>
          <p className="mt-2 leading-6 text-muted-foreground">{t('emptyAuto')}</p>
          {hasMicrophone && <p className="mt-1 leading-6 text-muted-foreground">{t('emptyManual')}</p>}
          <p className="mt-4 border-t border-border pt-4 text-xs leading-5 text-muted-foreground">{t('emptyNames')}</p>
        </section>
      ) : (
        // Bottom padding keeps the newest line clear of the floating recording controls.
        <div className="pb-32 pt-2">
          <VirtualizedTranscriptView
            segments={segments}
            isRecording={isRecording}
            isPaused={isPaused}
            isProcessing={isProcessingStop}
            isStopping={isStopping}
            enableStreaming={isRecording}
            showConfidence={showConfidenceIndicator}
          />
        </div>
      )}
    </div>
  );
}
