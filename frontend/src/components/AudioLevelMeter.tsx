import React from 'react';
import { cn } from '@/lib/utils';

interface AudioLevelMeterProps {
  rmsLevel: number;    // 0.0 to 1.0
  peakLevel: number;   // 0.0 to 1.0
  isActive: boolean;   // Whether audio is being detected
  deviceName: string;
  className?: string;
  size?: 'small' | 'medium' | 'large';
}

// Log scaling for a more readable meter.
const toPercent = (level: number) => {
  const normalized = Math.max(0, Math.min(1, level));
  return Math.round((normalized > 0 ? Math.log10(normalized * 9 + 1) : 0) * 100);
};

const SIZES = {
  small: { meter: 'h-1.5', text: 'text-xs' },
  medium: { meter: 'h-2', text: 'text-xs' },
  large: { meter: 'h-3', text: 'text-sm' },
};

// Levels in bronze on a stone track: minium is reserved for "Dixit is recording".
export function AudioLevelMeter({
  rmsLevel,
  peakLevel,
  isActive,
  deviceName,
  className = '',
  size = 'medium'
}: AudioLevelMeterProps) {
  const rmsPercent = toPercent(rmsLevel);
  const peakPercent = toPercent(peakLevel);
  const sizes = SIZES[size];

  return (
    <div className={cn('flex items-center gap-2', className)}>
      <span
        className={cn('h-2 w-2 shrink-0 rounded-full', isActive ? 'bg-primary' : 'bg-muted-foreground/40')}
        title={deviceName}
      />

      <div className={cn('relative flex-1 overflow-hidden rounded-sm bg-muted', sizes.meter)}>
        <div
          className="h-full rounded-sm bg-primary transition-[width] duration-150 ease-out"
          style={{ width: `${rmsPercent}%` }}
        />
        {peakPercent > rmsPercent && (
          <div
            className="absolute inset-y-0 w-0.5 bg-foreground/50 transition-[left] duration-75"
            style={{ left: `${peakPercent}%` }}
          />
        )}
      </div>

      <span className={cn('min-w-[3rem] text-right tabular-nums text-muted-foreground', sizes.text)}>
        {rmsPercent}%
      </span>
    </div>
  );
}

interface CompactAudioLevelMeterProps {
  rmsLevel: number;
  peakLevel: number;
  isActive: boolean;
  className?: string;
}

// Compact version for inline display in dropdowns
export function CompactAudioLevelMeter({
  rmsLevel,
  isActive,
  className = ''
}: CompactAudioLevelMeterProps) {
  const rmsPercent = toPercent(rmsLevel);

  return (
    <div className={cn('flex items-center gap-1', className)}>
      <span className={cn('h-1.5 w-1.5 rounded-full', isActive ? 'bg-primary' : 'bg-muted-foreground/40')} />
      <div className="h-1.5 w-8 overflow-hidden rounded-sm bg-muted">
        <div className="h-full bg-primary transition-[width] duration-150" style={{ width: `${rmsPercent}%` }} />
      </div>
    </div>
  );
}
