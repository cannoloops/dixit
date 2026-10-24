'use client';

import { useRef, useState, useReducer, startTransition, useEffect, useMemo, memo } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Check, Loader2, Pencil, Trash2, X } from "lucide-react";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { useAutoScroll } from "@/hooks/useAutoScroll";
import { useTranscriptStreaming } from "@/hooks/useTranscriptStreaming";
import { ConfidenceIndicator } from "./ConfidenceIndicator";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import { formatClock } from "./RecordingStatusBar";
import { SpeakerName } from "@/components/dixit";
import { TranscriptSegmentData } from "@/types";
import { cn } from "@/lib/utils";
import { t } from "@/i18n/recording";

export interface VirtualizedTranscriptViewProps {
    /** Transcript segments to display */
    segments: TranscriptSegmentData[];
    /** Distinct speaker names in order of appearance (fixes each speaker's colour). Derived from `segments` when omitted. */
    roster?: string[];
    /** Whether recording is in progress */
    isRecording?: boolean;
    /** Whether recording is paused */
    isPaused?: boolean;
    /** Whether processing/finalizing transcription */
    isProcessing?: boolean;
    /** Whether stopping */
    isStopping?: boolean;
    /** Enable streaming effect for latest segment */
    enableStreaming?: boolean;
    /** Show confidence indicators */
    showConfidence?: boolean;
    /** Words of the sentence still being spoken (live only), replaced by the next segment */
    partial?: { text: string; start: number } | null;
    /** Completely disable auto-scroll behavior (for meeting details page) */
    disableAutoScroll?: boolean;

    // Pagination props (infinite scroll)
    hasMore?: boolean;
    isLoadingMore?: boolean;
    totalCount?: number;
    loadedCount?: number;
    onLoadMore?: () => void;

    /** Edit / delete of the user's notes (saved meetings only; live notes are read-only). */
    onEditNote?: (id: string, text: string) => void;
    onDeleteNote?: (id: string) => void;
}

// Threshold for enabling virtualization (below this, use simple rendering)
const VIRTUALIZATION_THRESHOLD = 10;
// While recording, the latest lines of whoever is speaking now are set larger (at most this many).
const CURRENT_TURN_MAX = 3;

/** Distinct speakers in order of first appearance: one pass, computed once per list (not per row). */
export function speakerRoster(segments: { speaker?: string }[]): string[] {
    const seen = new Set<string>();
    for (const s of segments) if (s.speaker) seen.add(s.speaker);
    return Array.from(seen);
}

// Helper function to remove filler words and repetitions
function cleanStopWords(text: string): string {
    const stopWords = ['uh', 'um', 'er', 'ah', 'hmm', 'hm', 'eh', 'oh'];

    let cleanedText = text;
    stopWords.forEach(word => {
        const pattern = new RegExp(`\\b${word}\\b[,\\s]*`, 'gi');
        cleanedText = cleanedText.replace(pattern, ' ');
    });

    return cleanedText.replace(/\s+/g, ' ').trim();
}

/**
 * One line of the epigraph: the speaker carved in the left margin (only when the turn changes),
 * the recording offset, then the text at a fixed measure.
 */
const TranscriptLine = memo(function TranscriptLine({
    segment,
    visibleText,
    roster,
    hasSpeakers,
    isTurnStart,
    isCurrent,
    isCurrentStart,
    isStreaming,
    showConfidence,
}: {
    segment: TranscriptSegmentData;
    visibleText: string;
    roster: string[];
    hasSpeakers: boolean;
    isTurnStart: boolean;
    isCurrent: boolean;
    isCurrentStart: boolean;
    isStreaming: boolean;
    showConfidence: boolean;
}) {
    const isSilence = segment.text.trim() === '';
    const shown = isSilence ? t('silence') : cleanStopWords(visibleText) || visibleText;
    // While streaming, reserve the full text's height so the line doesn't grow under the controls.
    const full = isSilence ? shown : cleanStopWords(segment.text) || segment.text;

    const offset = formatClock(segment.timestamp);
    const hasConfidence = showConfidence && segment.confidence !== undefined;

    return (
        <div
            id={`segment-${segment.id}`}
            className={cn(
                'grid items-baseline justify-center gap-x-3 text-sm',
                hasSpeakers
                    ? 'grid-cols-[minmax(0,8rem)_3.5rem_minmax(0,68ch)]'
                    : 'grid-cols-[3.5rem_minmax(0,68ch)]',
                isCurrentStart ? 'pb-1 pt-5' : isTurnStart && hasSpeakers ? 'pb-1 pt-4' : 'py-1'
            )}
        >
            {hasSpeakers && (
                <div className="min-w-0 text-right">
                    {isTurnStart && segment.speaker && (
                        <SpeakerName
                            name={segment.speaker}
                            roster={roster}
                            className={cn('max-w-full', isCurrent && 'text-sm leading-7')}
                        />
                    )}
                </div>
            )}

            <div className="text-right text-xs tabular-nums text-muted-foreground">
                {hasConfidence ? (
                    <Tooltip>
                        <TooltipTrigger className="cursor-default rounded-sm tabular-nums">{offset}</TooltipTrigger>
                        <TooltipContent>
                            <ConfidenceIndicator confidence={segment.confidence!} showIndicator />
                        </TooltipContent>
                    </Tooltip>
                ) : (
                    offset
                )}
            </div>

            <p
                className={cn(
                    'min-w-0 text-foreground',
                    isCurrent ? 'text-lg leading-7' : 'leading-6',
                    isSilence && 'text-muted-foreground'
                )}
            >
                {isStreaming ? (
                    <span className="relative block">
                        <span className="invisible" aria-hidden>{full}</span>
                        <span className="absolute inset-0">{shown}</span>
                    </span>
                ) : (
                    shown
                )}
            </p>
        </div>
    );
});

/** A note of the user at its instant: petrol rule on the left, "Nota" label, edit and delete on hover. */
const NoteLine = memo(function NoteLine({
    segment,
    hasSpeakers,
    onEdit,
    onDelete,
}: {
    segment: TranscriptSegmentData;
    hasSpeakers: boolean;
    onEdit?: (id: string, text: string) => void;
    onDelete?: (id: string) => void;
}) {
    const [editing, setEditing] = useState(false);
    const [draft, setDraft] = useState(segment.text);
    const commit = () => {
        const text = draft.trim();
        setEditing(false);
        if (text && text !== segment.text) onEdit?.(segment.id, text);
    };

    return (
        <div
            id={`segment-${segment.id}`}
            className={cn(
                'group grid items-baseline justify-center gap-x-3 py-2 text-sm',
                hasSpeakers
                    ? 'grid-cols-[minmax(0,8rem)_3.5rem_minmax(0,68ch)]'
                    : 'grid-cols-[3.5rem_minmax(0,68ch)]'
            )}
        >
            {hasSpeakers && <div />}
            <div className="text-right text-xs tabular-nums text-muted-foreground">{formatClock(segment.timestamp)}</div>
            <div className="flex min-w-0 items-baseline gap-2 border-l-2 border-primary pl-3">
                {editing ? (
                    <>
                        <Input
                            autoFocus
                            value={draft}
                            onChange={(e) => setDraft(e.target.value)}
                            onKeyDown={(e) => {
                                if (e.key === 'Enter' && !e.nativeEvent.isComposing) commit();
                                if (e.key === 'Escape') setEditing(false);
                            }}
                            aria-label={t('noteEdit')}
                        />
                        <Button size="sm" variant="ghost" onClick={commit} aria-label={t('noteEdit')}><Check /></Button>
                        <Button size="sm" variant="ghost" onClick={() => setEditing(false)} aria-label="Esc"><X /></Button>
                    </>
                ) : (
                    <>
                        <p className="min-w-0 leading-6 text-foreground">
                            <span className="mr-2 text-xs font-semibold text-primary">{t('noteLabel')}</span>
                            {segment.text}
                        </p>
                        {(onEdit || onDelete) && (
                            <span className="ml-auto flex shrink-0 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
                                {onEdit && (
                                    <Button
                                        size="sm"
                                        variant="ghost"
                                        title={t('noteEdit')}
                                        aria-label={t('noteEdit')}
                                        onClick={() => { setDraft(segment.text); setEditing(true); }}
                                    >
                                        <Pencil />
                                    </Button>
                                )}
                                {onDelete && (
                                    <Button
                                        size="sm"
                                        variant="ghost"
                                        title={t('noteDelete')}
                                        aria-label={t('noteDelete')}
                                        onClick={() => onDelete(segment.id)}
                                    >
                                        <Trash2 />
                                    </Button>
                                )}
                            </span>
                        )}
                    </>
                )}
            </div>
        </div>
    );
});

export const VirtualizedTranscriptView: React.FC<VirtualizedTranscriptViewProps> = ({
    segments,
    roster: rosterProp,
    isRecording = false,
    isPaused = false,
    enableStreaming = false,
    showConfidence = true,
    disableAutoScroll = false,
    partial = null,
    hasMore = false,
    isLoadingMore = false,
    totalCount = 0,
    loadedCount = 0,
    onLoadMore,
    onEditNote,
    onDeleteNote,
}) => {
    // Create scroll ref first - shared between virtualizer and auto-scroll hook
    const scrollRef = useRef<HTMLDivElement>(null);
    // Ref for infinite scroll trigger element
    const loadMoreTriggerRef = useRef<HTMLDivElement>(null);

    // Force re-render without flushSync (avoids React warning)
    const [, rerender] = useReducer((x: number) => x + 1, 0);

    const roster = useMemo(() => rosterProp ?? speakerRoster(segments), [rosterProp, segments]);
    const hasSpeakers = roster.length > 0;

    // While recording, the protagonist is whoever spoke last: their latest lines (same speaker,
    // up to CURRENT_TURN_MAX; just the last line when speakers are not known yet) are set larger.
    const currentFrom = useMemo(() => {
        if (!isRecording || segments.length === 0) return segments.length;
        let last = segments.length - 1;
        while (last > 0 && segments[last].note) last--; // a note is never "who spoke last"
        const speaker = segments[last].speaker;
        if (!speaker) return last;
        let i = last;
        while (i > 0 && last - i < CURRENT_TURN_MAX - 1 && segments[i - 1].speaker === speaker) i--;
        return i;
    }, [isRecording, segments]);

    // Setup virtualizer for efficient rendering of large lists
    const virtualizer = useVirtualizer({
        count: segments.length,
        getScrollElement: () => scrollRef.current,
        estimateSize: () => 32, // One 28-32px line per segment
        overscan: 10, // Render extra items above/below viewport
        onChange: () => {
            startTransition(() => {
                rerender();
            });
        },
    });

    // Custom hook for auto-scrolling (supports both virtualized and non-virtualized)
    useAutoScroll({
        scrollRef,
        segments,
        isRecording,
        isPaused,
        virtualizer,
        virtualizationThreshold: VIRTUALIZATION_THRESHOLD,
        disableAutoScroll,
    });

    // Streaming text effect hook (typewriter animation for new transcripts)
    const { streamingSegmentId, getDisplayText } = useTranscriptStreaming(
        segments,
        isRecording,
        enableStreaming
    );

    // Infinite scroll: IntersectionObserver to trigger loading more
    useEffect(() => {
        if (!onLoadMore || !hasMore || isLoadingMore || isRecording || segments.length === 0) {
            return;
        }

        const triggerElement = loadMoreTriggerRef.current;
        if (!triggerElement) return;

        const observer = new IntersectionObserver(
            (entries) => {
                if (entries[0].isIntersecting && hasMore && !isLoadingMore) {
                    onLoadMore();
                }
            },
            {
                root: null,
                rootMargin: '100px',
                threshold: 0,
            }
        );

        observer.observe(triggerElement);

        return () => observer.disconnect();
    }, [hasMore, isLoadingMore, onLoadMore, isRecording, segments.length]);

    // Scroll-based fallback for fast scrolling
    useEffect(() => {
        if (!onLoadMore || !hasMore || isLoadingMore || isRecording) return;

        const scrollElement = scrollRef.current;
        if (!scrollElement) return;

        let ticking = false;

        const handleScroll = () => {
            if (ticking || isLoadingMore || !hasMore) return;

            ticking = true;
            requestAnimationFrame(() => {
                const { scrollTop, scrollHeight, clientHeight } = scrollElement;
                const scrollBottom = scrollHeight - scrollTop - clientHeight;

                // Trigger load when within 200px of bottom
                if (scrollBottom < 200 && hasMore && !isLoadingMore) {
                    onLoadMore();
                }
                ticking = false;
            });
        };

        scrollElement.addEventListener('scroll', handleScroll, { passive: true });
        return () => scrollElement.removeEventListener('scroll', handleScroll);
    }, [onLoadMore, hasMore, isLoadingMore, isRecording]);

    // Use simple rendering for small lists, virtualization for large lists
    const useVirtualization = segments.length >= VIRTUALIZATION_THRESHOLD;

    const renderLine = (index: number) => {
        const segment = segments[index];
        if (segment.note) {
            return <NoteLine segment={segment} hasSpeakers={hasSpeakers} onEdit={onEditNote} onDelete={onDeleteNote} />;
        }
        return (
            <TranscriptLine
                segment={segment}
                visibleText={getDisplayText(segment)}
                roster={roster}
                hasSpeakers={hasSpeakers}
                isTurnStart={index === 0 || segments[index - 1].speaker !== segment.speaker}
                isCurrent={index >= currentFrom && !segments[index].note}
                isCurrentStart={index === currentFrom && index > 0}
                isStreaming={streamingSegmentId === segment.id}
                showConfidence={showConfidence}
            />
        );
    };

    const partialLine = partial && (
        <div className="opacity-50">
            <TranscriptLine
                segment={{ id: 'partial', timestamp: partial.start, text: partial.text }}
                visibleText={partial.text}
                roster={roster}
                hasSpeakers={hasSpeakers}
                isTurnStart={false}
                isCurrent
                isCurrentStart={false}
                isStreaming={false}
                showConfidence={false}
            />
        </div>
    );

    const loadMore = (hasMore || isLoadingMore) && !isRecording && segments.length > 0 && (
        <div ref={loadMoreTriggerRef} className="flex items-center justify-center py-4 text-xs text-muted-foreground">
            {isLoadingMore ? (
                <span className="inline-flex items-center gap-2">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {t('loadingMore')}
                </span>
            ) : hasMore && totalCount > 0 ? (
                <span className="tabular-nums">{t('showingOf', { loaded: loadedCount, total: totalCount })}</span>
            ) : null}
        </div>
    );

    return (
        <div ref={scrollRef} className="flex h-full flex-col overflow-y-auto px-4 py-2">
            {segments.length === 0 && !partial ? (
                <p className="mx-auto w-full max-w-[60ch] pt-10 text-sm text-muted-foreground">
                    {isRecording ? (isPaused ? t('pausedEmpty') : t('listeningEmpty')) : t('noTranscript')}
                </p>
            ) : useVirtualization ? (
                <>
                    <div
                        style={{
                            height: virtualizer.getTotalSize(),
                            width: "100%",
                            position: "relative",
                            // The scroller is a flex column: without this the list shrinks to the
                            // viewport and the load-more footer lands on top of the rows.
                            flexShrink: 0,
                        }}
                    >
                        {virtualizer.getVirtualItems().map((virtualRow) => (
                            <div
                                key={segments[virtualRow.index].id}
                                data-index={virtualRow.index}
                                ref={virtualizer.measureElement}
                                style={{
                                    position: "absolute",
                                    top: 0,
                                    left: 0,
                                    width: "100%",
                                    transform: `translateY(${virtualRow.start}px)`,
                                }}
                            >
                                {renderLine(virtualRow.index)}
                            </div>
                        ))}
                    </div>
                    {partialLine}
                    {loadMore}
                </>
            ) : (
                <>
                    <div>
                        {segments.map((segment, index) => (
                            <div key={segment.id}>{renderLine(index)}</div>
                        ))}
                    </div>
                    {partialLine}
                    {loadMore}
                </>
            )}
        </div>
    );
};
