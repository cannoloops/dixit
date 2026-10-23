import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PaginatedTranscriptsResponse, Transcript } from '@/types';

// One read for the whole meeting: even a 3.5 h call is a few thousand lines.
const ALL_LINES = 1_000_000;

/** Distinct speakers in order of first appearance: the roster that fixes each speaker's colour. */
export function rosterOf(lines: { speaker?: string }[]): string[] {
  return Array.from(new Set(lines.map((l) => l.speaker).filter((s): s is string => !!s)));
}

/**
 * Every transcript line of a meeting in time order (the transcript list itself is paginated),
 * for the speaker band, the roster and the meeting duration.
 */
export function useMeetingTimeline(meetingId: string) {
  const [lines, setLines] = useState<Transcript[] | null>(null);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    let cancelled = false;
    invoke<PaginatedTranscriptsResponse>('api_get_meeting_transcripts', {
      meetingId,
      limit: ALL_LINES,
      offset: 0,
    })
      .then((response) => {
        if (cancelled) return;
        setLines(
          [...response.transcripts].sort((a, b) => (a.audio_start_time ?? 0) - (b.audio_start_time ?? 0))
        );
      })
      .catch((error) => {
        console.error('Failed to load the meeting timeline:', error);
        if (!cancelled) setLines([]);
      });
    return () => {
      cancelled = true;
    };
  }, [meetingId, revision]);

  /** Seconds from the start of the recording to the end of the last line (0 when lines carry no times). */
  const duration = useMemo(
    () => (lines ?? []).reduce((max, l) => Math.max(max, l.audio_end_time ?? l.audio_start_time ?? 0), 0),
    [lines]
  );

  const refresh = useCallback(() => setRevision((r) => r + 1), []);

  return { lines, duration, refresh };
}
