'use client';

import { useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { t } from '@/i18n/shell';

/** A summary on its way or running (`api_summaries_in_progress`). */
export interface SummaryJob {
  meeting_id: string;
  automatic: boolean;
  /** `queued`, `transcribing`, `speakers`, `starting` (automatic only) or `generating`. */
  phase: 'queued' | 'transcribing' | 'speakers' | 'starting' | 'generating';
  /** Real percentage of the final pass while `transcribing`. */
  progress: number | null;
  /** Set while `generating`: what `api_cancel_summary` stops. */
  process_id: string | null;
}

// One poll for the whole app (sidebar and meeting view): every 2 s while something runs, 5 s otherwise.
// ponytail: polling an in-memory list; push events only if the cost ever shows.
let jobs: SummaryJob[] = [];
let snapshot = '[]';
const listeners = new Set<() => void>();
let timer: ReturnType<typeof setTimeout> | undefined;
let unlisten: Array<Promise<() => void>> = [];

async function poll() {
  clearTimeout(timer);
  const next = await invoke<SummaryJob[]>('api_summaries_in_progress').catch(() => jobs);
  const text = JSON.stringify(next);
  if (text !== snapshot) {
    snapshot = text;
    jobs = next;
    listeners.forEach((listener) => listener());
  }
  if (listeners.size > 0) timer = setTimeout(poll, jobs.length > 0 ? 2000 : 5000);
}

/** Re-reads the list now, e.g. right after starting or cancelling a summary. */
export function refreshSummariesInProgress() {
  if (listeners.size > 0) void poll();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  if (listeners.size === 1) {
    unlisten = ['recording-stopped', 'summary-auto-started', 'retranscription-complete'].map((event) =>
      listen(event, refreshSummariesInProgress)
    );
    void poll();
  }
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) {
      clearTimeout(timer);
      unlisten.forEach((off) => off.then((fn) => fn()).catch(() => {}));
      unlisten = [];
    }
  };
}

const EMPTY: SummaryJob[] = [];

/** Summaries in progress, automatic and manual, refreshed in the background. */
export function useSummariesInProgress(): SummaryJob[] {
  return useSyncExternalStore(subscribe, () => jobs, () => EMPTY);
}

/** "Trascrizione definitiva 42%", "Riconosco i parlanti", "Genero il riassunto"… */
export function phaseLabel(job: SummaryJob): string {
  switch (job.phase) {
    case 'queued':
      return t('summaryPhaseQueued');
    case 'transcribing':
      return job.progress != null
        ? t('summaryPhaseTranscribingAt', { progress: job.progress })
        : t('summaryPhaseTranscribing');
    case 'speakers':
      return t('summaryPhaseSpeakers');
    case 'starting':
      return t('summaryPhaseStarting');
    default:
      return t('summaryPhaseGenerating');
  }
}

/** Stops a running summary, or cancels an automatic one still waiting. True when it worked. */
export async function cancelSummaryJob(job: SummaryJob): Promise<boolean> {
  try {
    if (job.phase === 'generating' && job.process_id) {
      const result = await invoke<{ cancelled?: boolean }>('api_cancel_summary', {
        meetingId: job.meeting_id,
        processId: job.process_id,
      });
      return !!result?.cancelled;
    }
    return await invoke<boolean>('api_cancel_auto_summary', { meetingId: job.meeting_id });
  } finally {
    refreshSummariesInProgress();
  }
}
