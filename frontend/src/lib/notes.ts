import { TranscriptSegmentData } from '@/types';

export interface MeetingNote {
  id: string;
  text: string;
  offset_ms: number;
  created_at: string;
  updated_at: string;
}

/**
 * Puts each note after the lines that start before it. While the transcript is still paginated
 * (`complete` false) notes past the last loaded line wait, so they do not jump above lines to come.
 */
export function withNotes(
  segments: TranscriptSegmentData[],
  notes: MeetingNote[],
  complete = true
): TranscriptSegmentData[] {
  if (notes.length === 0) return segments;
  const lastLoaded = segments.length ? segments[segments.length - 1].timestamp : 0;
  const shown = [...notes]
    .filter((n) => complete || n.offset_ms / 1000 <= lastLoaded)
    .sort((a, b) => a.offset_ms - b.offset_ms);
  const out: TranscriptSegmentData[] = [];
  let i = 0;
  for (const n of shown) {
    const at = n.offset_ms / 1000;
    while (i < segments.length && segments[i].timestamp <= at) out.push(segments[i++]);
    out.push({ id: n.id, timestamp: at, text: n.text, note: true });
  }
  return out.concat(segments.slice(i));
}
