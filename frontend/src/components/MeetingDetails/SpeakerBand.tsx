"use client";

import { useMemo } from 'react';
import type { Transcript } from '@/types';
import { cn } from '@/lib/utils';
import { speakerColor, speakerLabel, useMe } from '@/components/dixit';
import { SpeakersPanel } from './SpeakersPanel';
import { t } from '@/i18n/meeting';

interface Turn {
  /** Id of the turn's first transcript line (the row to jump to). */
  id: string;
  speaker: string;
  start?: number;
  end?: number;
}

interface SpeakerBandProps {
  meetingId: string;
  /** Every transcript line of the meeting in time order; null while loading. */
  lines: Transcript[] | null;
  /** Seconds covered by the recording (0 when the lines carry no times). */
  duration: number;
  roster: string[];
  /** Called after a rename, so the transcript and the band reload. */
  onChanged: () => unknown;
  /** Clicking a turn on the strip goes to its first line. */
  onSelectTurn?: (lineId: string) => void;
  className?: string;
}

/** Recording-relative clock: m:ss, or h:mm:ss from the first hour. */
function clock(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${m}:${ss}`;
}

/** Consecutive lines of the same speaker form one turn. */
function buildTurns(lines: Transcript[]) {
  const turns: Turn[] = [];
  const counts = new Map<string, number>();
  for (const line of lines) {
    if (!line.speaker) continue;
    const end = line.audio_end_time ?? line.audio_start_time;
    const last = turns[turns.length - 1];
    if (last && last.speaker === line.speaker) {
      if (end !== undefined) last.end = Math.max(last.end ?? end, end);
      continue;
    }
    counts.set(line.speaker, (counts.get(line.speaker) ?? 0) + 1);
    turns.push({ id: line.id, speaker: line.speaker, start: line.audio_start_time, end });
  }
  const speakers = Array.from(counts.entries()).sort((a, b) => b[1] - a[1]);
  return { turns, speakers };
}

/**
 * Speaker band: a thin strip where every turn is a segment in its speaker's pigment, on one time
 * scale, with the carved names below (each one click away from renaming). Without times, legend only.
 */
export function SpeakerBand({
  meetingId,
  lines,
  duration,
  roster,
  onChanged,
  onSelectTurn,
  className,
}: SpeakerBandProps) {
  const me = useMe();
  const { turns, speakers } = useMemo(() => buildTurns(lines ?? []), [lines]);
  const timed = duration > 0 ? turns.filter((turn) => turn.start !== undefined) : [];

  if (speakers.length === 0) return null;

  return (
    <section aria-label={t('speakerBand')} className={cn('space-y-1.5', className)}>
      {timed.length > 0 && (
        <div aria-hidden>
          <div className="relative h-2.5 w-full overflow-hidden rounded-sm bg-muted">
            {timed.map((turn) => {
              const start = turn.start ?? 0;
              const end = Math.max(turn.end ?? start, start);
              return (
                <span
                  key={turn.id}
                  title={`${speakerLabel(turn.speaker, me)} · ${clock(start)}`}
                  onClick={onSelectTurn ? () => onSelectTurn(turn.id) : undefined}
                  className={cn(
                    'absolute inset-y-0',
                    onSelectTurn && 'cursor-pointer transition-opacity duration-150 hover:opacity-70'
                  )}
                  style={{
                    left: `${(start / duration) * 100}%`,
                    width: `max(2px, ${((end - start) / duration) * 100}%)`,
                    backgroundColor: speakerColor(turn.speaker, roster, me),
                  }}
                />
              );
            })}
          </div>
          <div className="mt-0.5 flex justify-between text-[11px] tabular-nums leading-4 text-muted-foreground">
            <span>{clock(0)}</span>
            <span>{clock(duration)}</span>
          </div>
        </div>
      )}
      <SpeakersPanel meetingId={meetingId} speakers={speakers} roster={roster} onChanged={onChanged} />
    </section>
  );
}
