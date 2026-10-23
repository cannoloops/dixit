/**
 * TranscriptRecovery Component
 *
 * Modal dialog for recovering interrupted meetings from IndexedDB.
 * Displays recoverable meetings, allows preview, and enables recovery or deletion.
 */

import React, { useState, useEffect } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { it as itLocale } from 'date-fns/locale';
import { Loader2, RotateCcw, Trash2 } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { formatClock } from '@/components/RecordingStatusBar';
import { MeetingMetadata, StoredTranscript } from '@/services/indexedDBService';
import { cn } from '@/lib/utils';
import { lang, locale } from '@/i18n';
import { t } from '@/i18n/recording';
import { t as tApp } from '@/i18n/app';

const PREVIEW_LIMIT = 10;

const segmentsLabel = (count: number) => (count === 1 ? t('segmentsOne') : t('segmentsMany', { count }));

const relativeTime = (ms: number) =>
  formatDistanceToNow(new Date(ms), { addSuffix: true, locale: lang === 'it' ? itLocale : undefined });

// Recording offset when known, otherwise the wall-clock time the line was stored with.
function previewTime(transcript: StoredTranscript): string {
  if (transcript.audio_start_time !== undefined) return formatClock(transcript.audio_start_time);
  const date = new Date(transcript.timestamp);
  return isNaN(date.getTime()) ? '--:--' : date.toLocaleTimeString(locale);
}

interface TranscriptRecoveryProps {
  isOpen: boolean;
  onClose: () => void;
  recoverableMeetings: MeetingMetadata[];
  onRecover: (meetingId: string) => Promise<any>;
  onDelete: (meetingId: string) => Promise<void>;
  onLoadPreview: (meetingId: string) => Promise<StoredTranscript[]>;
}

export function TranscriptRecovery({
  isOpen,
  onClose,
  recoverableMeetings,
  onRecover,
  onDelete,
  onLoadPreview,
}: TranscriptRecoveryProps) {
  const [selectedMeetingId, setSelectedMeetingId] = useState<string | null>(null);
  const [previewTranscripts, setPreviewTranscripts] = useState<StoredTranscript[]>([]);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);
  const [isRecovering, setIsRecovering] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);

  // Reset selection when dialog opens
  useEffect(() => {
    if (isOpen) {
      setSelectedMeetingId(null);
      setPreviewTranscripts([]);
    }
  }, [isOpen]);

  // Auto-select first meeting if available
  useEffect(() => {
    if (isOpen && recoverableMeetings.length > 0 && !selectedMeetingId) {
      handleMeetingSelect(recoverableMeetings[0].meetingId);
    }
  }, [isOpen, recoverableMeetings]);

  const handleMeetingSelect = async (meetingId: string) => {
    setSelectedMeetingId(meetingId);
    setIsLoadingPreview(true);

    try {
      const transcripts = await onLoadPreview(meetingId);
      // Limit to first 10 for preview
      setPreviewTranscripts(transcripts.slice(0, PREVIEW_LIMIT));
    } catch (error) {
      console.error('Failed to load preview:', error);
      setPreviewTranscripts([]);
    } finally {
      setIsLoadingPreview(false);
    }
  };

  const handleRecover = async () => {
    if (!selectedMeetingId) return;

    setIsRecovering(true);
    try {
      const result = await onRecover(selectedMeetingId);
      console.log('Recovery successful:', result);
      onClose();
    } catch (error) {
      console.error('Recovery failed:', error);
      alert(t('recoverRetry'));
    } finally {
      setIsRecovering(false);
    }
  };

  const handleDelete = async () => {
    if (!selectedMeetingId) return;

    if (!confirm(t('deleteConfirm'))) {
      return;
    }

    setIsDeleting(true);
    try {
      await onDelete(selectedMeetingId);
      setSelectedMeetingId(null);
      setPreviewTranscripts([]);
    } catch (error) {
      console.error('Delete failed:', error);
      alert(t('deleteRetry'));
    } finally {
      setIsDeleting(false);
    }
  };

  const selectedMeeting = recoverableMeetings.find(m => m.meetingId === selectedMeetingId);
  const busy = isRecovering || isDeleting;

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="flex h-[80vh] max-w-4xl flex-col gap-0 p-0">
        <DialogHeader className="border-b border-border px-6 py-4">
          <DialogTitle>{t('recoveryTitle')}</DialogTitle>
          <DialogDescription>
            {recoverableMeetings.length === 1
              ? t('recoveryFoundOne')
              : t('recoveryFoundMany', { count: recoverableMeetings.length })}
          </DialogDescription>
        </DialogHeader>

        <div className="flex min-h-0 flex-1">
          {/* Meeting list */}
          <div className="flex w-1/3 min-w-0 flex-col border-r border-border">
            <h3 className="px-4 pb-1 pt-3 text-xs font-medium text-muted-foreground">{t('interruptedMeetings')}</h3>
            <ScrollArea className="flex-1">
              <div className="px-2 pb-2">
                {recoverableMeetings.map((meeting) => (
                  <button
                    key={meeting.meetingId}
                    onClick={() => handleMeetingSelect(meeting.meetingId)}
                    className={cn(
                      'w-full rounded-sm px-2 py-1.5 text-left transition-colors hover:bg-accent',
                      selectedMeetingId === meeting.meetingId && 'bg-accent'
                    )}
                  >
                    <span className="block truncate text-sm font-semibold">{meeting.title}</span>
                    <span className="block truncate text-xs text-muted-foreground">
                      {relativeTime(meeting.lastUpdated)} · <span className="tabular-nums">{segmentsLabel(meeting.transcriptCount)}</span>
                      {!meeting.folderPath && <> · {t('noAudio')}</>}
                    </span>
                  </button>
                ))}
              </div>
            </ScrollArea>
          </div>

          {/* Preview */}
          <div className="flex min-w-0 flex-1 flex-col">
            {selectedMeeting ? (
              <>
                <div className="border-b border-border px-6 py-3">
                  <h4 className="truncate font-display text-sm font-semibold">{selectedMeeting.title}</h4>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {t('startedAt', { date: new Date(selectedMeeting.startTime).toLocaleString(locale) })}
                    {' · '}<span className="tabular-nums">{segmentsLabel(selectedMeeting.transcriptCount)}</span>
                    {' · '}{selectedMeeting.folderPath ? t('audioAvailable') : t('noAudio')}
                  </p>
                </div>

                <ScrollArea className="flex-1">
                  <div className="px-6 py-3 text-sm">
                    {isLoadingPreview ? (
                      <p className="text-muted-foreground">{t('loadingPreview')}</p>
                    ) : previewTranscripts.length > 0 ? (
                      <>
                        <p className="mb-2 text-xs text-muted-foreground">
                          {t('previewShowing', { shown: previewTranscripts.length, total: selectedMeeting.transcriptCount })}
                        </p>
                        <ul>
                          {previewTranscripts.map((transcript, index) => (
                            <li key={index} className="grid grid-cols-[3.5rem_minmax(0,1fr)] items-baseline gap-x-3 py-1">
                              <span className="text-right text-xs tabular-nums text-muted-foreground">{previewTime(transcript)}</span>
                              <span className="leading-6">{transcript.text}</span>
                            </li>
                          ))}
                        </ul>
                        {selectedMeeting.transcriptCount > PREVIEW_LIMIT && (
                          <p className="mt-2 text-xs text-muted-foreground">
                            {t('previewMore', { count: selectedMeeting.transcriptCount - PREVIEW_LIMIT })}
                          </p>
                        )}
                      </>
                    ) : (
                      <p className="text-muted-foreground">{t('noPreview')}</p>
                    )}
                  </div>
                </ScrollArea>
              </>
            ) : (
              <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
                {t('selectToPreview')}
              </div>
            )}
          </div>
        </div>

        <DialogFooter className="border-t border-border px-6 py-3">
          <Button variant="outline" onClick={onClose} disabled={busy}>
            {tApp('cancel')}
          </Button>
          <Button variant="destructive" onClick={handleDelete} disabled={!selectedMeetingId || busy}>
            {isDeleting ? <Loader2 className="animate-spin" /> : <Trash2 />}
            {isDeleting ? t('deleting') : t('delete')}
          </Button>
          <Button onClick={handleRecover} disabled={!selectedMeetingId || busy}>
            {isRecovering ? <Loader2 className="animate-spin" /> : <RotateCcw />}
            {isRecovering ? t('recovering') : t('recover')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
