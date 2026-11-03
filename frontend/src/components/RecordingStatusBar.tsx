'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { MicOff } from 'lucide-react';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { LiveIndicator } from '@/components/dixit';
import { cn } from '@/lib/utils';
import { t } from '@/i18n/recording';

/** Recording-relative clock: MM:SS, or H:MM:SS past the hour. */
export function formatClock(seconds: number | undefined): string {
  if (seconds === undefined || !Number.isFinite(seconds)) return '--:--';
  const total = Math.max(0, Math.floor(seconds));
  const h = Math.floor(total / 3600);
  const mm = String(Math.floor((total % 3600) / 60)).padStart(2, '0');
  const ss = String(total % 60).padStart(2, '0');
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

interface RecordingStatusBarProps {
  isPaused?: boolean;
  className?: string;
}

/** The recording light (red only while live) and the elapsed time. */
export const RecordingStatusBar: React.FC<RecordingStatusBarProps> = ({ isPaused = false, className }) => {
  // Backend-synced duration (polled every 500ms); keep the last value if it goes null.
  const { activeDuration } = useRecordingState();
  const [displaySeconds, setDisplaySeconds] = useState(0);

  useEffect(() => {
    if (activeDuration !== null) setDisplaySeconds(Math.floor(activeDuration));
  }, [activeDuration]);

  // Muted in the call app (Teams): the mic is recorded as silence.
  const [micMuted, setMicMuted] = useState(false);
  useEffect(() => {
    invoke<boolean>('mic_muted').then(setMicMuted).catch(() => {});
    const unlisten = listen<boolean>('mic-muted', (e) => setMicMuted(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return (
    <span className={cn('inline-flex items-center gap-2', className)}>
      {isPaused ? (
        <span className="inline-flex items-center gap-1.5 text-xs font-medium text-muted-foreground" role="status">
          <span className="h-2 w-2 rounded-full bg-muted-foreground" />
          {t('paused')}
        </span>
      ) : (
        <LiveIndicator />
      )}
      <span className="text-sm tabular-nums text-foreground">{formatClock(displaySeconds)}</span>
      {micMuted && (
        <span
          className="inline-flex items-center gap-1.5 text-xs font-medium text-muted-foreground"
          role="status"
          title={t('micMutedHint')}
        >
          <MicOff className="h-3.5 w-3.5" aria-hidden />
          {t('micMuted')}
        </span>
      )}
    </span>
  );
};
