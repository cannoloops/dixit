"use client";

import { Transcript, TranscriptSegmentData } from '@/types';
import { VirtualizedTranscriptView } from '@/components/VirtualizedTranscriptView';
import { Textarea } from '@/components/ui/textarea';
import { TranscriptButtonGroup } from './TranscriptButtonGroup';
import { useEffect, useMemo, useRef } from 'react';
import { t } from '@/i18n/meeting';
import { MeetingNote, withNotes } from '@/lib/notes';

interface TranscriptPanelProps {
  transcripts: Transcript[];
  customPrompt: string;
  onPromptChange: (value: string) => void;
  onCopyTranscript: () => void;
  onOpenMeetingFolder: () => Promise<void>;
  isRecording: boolean;
  disableAutoScroll?: boolean;

  // Optional pagination props (when using virtualization)
  usePagination?: boolean;
  segments?: TranscriptSegmentData[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;

  // Retranscription props
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;

  /** Ordered distinct speakers of the meeting, so each name keeps its colour. */
  roster?: string[];
  /** Line to scroll to (from the speaker band); a new nonce repeats the jump. */
  jumpTo?: { id: string; nonce: number } | null;

  /** The user's notes, shown in the transcript at their instant. */
  notes?: MeetingNote[];
  onEditNote?: (id: string, text: string) => void;
  onDeleteNote?: (id: string) => void;
  includeNotes?: boolean;
  onIncludeNotesChange?: (value: boolean) => void;
}

/** Scrolls the list to a line, also when it sits outside the virtualized window. */
function revealSegment(list: HTMLElement | null, id: string, index: number, count: number, attempt = 0) {
  const row = document.getElementById(`segment-${id}`);
  if (row) {
    row.scrollIntoView({ block: 'start' });
    return;
  }
  const scroller = list?.firstElementChild as HTMLElement | null;
  if (!scroller || attempt > 4) return;
  scroller.scrollTop = (index / Math.max(count, 1)) * scroller.scrollHeight;
  window.setTimeout(() => revealSegment(list, id, index, count, attempt + 1), 60);
}

export function TranscriptPanel({
  transcripts,
  customPrompt,
  onPromptChange,
  onCopyTranscript,
  onOpenMeetingFolder,
  isRecording,
  disableAutoScroll = false,
  usePagination = false,
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
  roster,
  jumpTo = null,
  notes = [],
  onEditNote,
  onDeleteNote,
  includeNotes = true,
  onIncludeNotesChange,
}: TranscriptPanelProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const handledJump = useRef<number | null>(null);

  // Convert transcripts to segments if pagination is not used but we want virtualization
  const convertedSegments = useMemo(() => {
    const base = usePagination && segments ? segments : transcripts.map(line => ({
      id: line.id,
      timestamp: line.audio_start_time ?? 0,
      endTime: line.audio_end_time,
      text: line.text,
      confidence: line.confidence,
      speaker: line.speaker,
    }));
    return withNotes(base, notes, !hasMore);
  }, [transcripts, usePagination, segments, notes, hasMore]);

  // Jump from the speaker band: load pages until the line is there, then scroll to it.
  useEffect(() => {
    if (!jumpTo || handledJump.current === jumpTo.nonce) return;
    const index = convertedSegments.findIndex((s) => s.id === jumpTo.id);
    if (index >= 0) {
      handledJump.current = jumpTo.nonce;
      revealSegment(listRef.current, jumpTo.id, index, convertedSegments.length);
      return;
    }
    if (!hasMore || !onLoadMore) {
      handledJump.current = jumpTo.nonce;
      return;
    }
    if (isLoadingMore) return;
    // The pager ignores calls closer than 100 ms apart.
    const timer = window.setTimeout(onLoadMore, 150);
    return () => window.clearTimeout(timer);
  }, [jumpTo, convertedSegments, hasMore, isLoadingMore, onLoadMore]);

  const lineCount = usePagination ? (totalCount ?? segments?.length ?? 0) : (transcripts?.length || 0);

  return (
    <div className="@container relative flex h-full w-full min-w-0 flex-col bg-background">
      <div className="flex h-10 shrink-0 items-center gap-2 border-b border-border px-4">
        <h2 className="text-sm font-semibold">{t('transcript')}</h2>
        <span className="text-xs tabular-nums text-muted-foreground">
          {lineCount === 1 ? t('lineCountOne') : t('lineCount', { n: lineCount })}
        </span>
        <div className="ml-auto">
          <TranscriptButtonGroup
            transcriptCount={lineCount}
            onCopyTranscript={onCopyTranscript}
            onOpenMeetingFolder={onOpenMeetingFolder}
            meetingId={meetingId}
            meetingFolderPath={meetingFolderPath}
            onRefetchTranscripts={onRefetchTranscripts}
            hasNotes={notes.length > 0}
            includeNotes={includeNotes}
            onIncludeNotesChange={onIncludeNotesChange}
          />
        </div>
      </div>

      {/* The rows keep a fixed measure themselves (names in the margin, text at most 68ch). */}
      <div ref={listRef} className="min-h-0 w-full flex-1 overflow-hidden">
        <VirtualizedTranscriptView
          segments={convertedSegments}
          roster={roster}
          isRecording={isRecording}
          isPaused={false}
          isProcessing={false}
          isStopping={false}
          enableStreaming={false}
          showConfidence={true}
          disableAutoScroll={disableAutoScroll}
          hasMore={hasMore}
          isLoadingMore={isLoadingMore}
          totalCount={totalCount}
          loadedCount={loadedCount}
          onLoadMore={onLoadMore}
          onEditNote={onEditNote}
          onDeleteNote={onDeleteNote}
        />
      </div>

      {!isRecording && convertedSegments.length > 0 && (
        <div className="shrink-0 border-t border-border p-2">
          <Textarea
            aria-label={t('contextLabel')}
            placeholder={t('contextPlaceholder')}
            className="min-h-[56px] resize-y bg-background"
            value={customPrompt}
            onChange={(e) => onPromptChange(e.target.value)}
          />
        </div>
      )}
    </div>
  );
}
