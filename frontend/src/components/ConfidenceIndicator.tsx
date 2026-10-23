'use client';

import { cn } from '@/lib/utils';
import { t } from '@/i18n/recording';

interface ConfidenceIndicatorProps {
  confidence: number;
  showIndicator?: boolean;
}

// Bronze when the transcription is reliable, stone when middling, porphyry when it likely needs checking.
function level(conf: number) {
  if (conf >= 0.8) return { dot: 'bg-primary', label: t('confidenceHigh') };
  if (conf >= 0.7) return { dot: 'bg-primary/60', label: t('confidenceGood') };
  if (conf >= 0.4) return { dot: 'bg-muted-foreground', label: t('confidenceMedium') };
  return { dot: 'bg-destructive', label: t('confidenceLow') };
}

export const ConfidenceIndicator: React.FC<ConfidenceIndicatorProps> = ({
  confidence,
  showIndicator = true,
}) => {
  if (!showIndicator) return null;

  const percent = (confidence * 100).toFixed(0);
  const { dot, label } = level(confidence);

  return (
    <span
      className="inline-flex items-center gap-1.5 text-xs"
      role="status"
      aria-label={t('confidenceAria', { percent })}
    >
      <span className={cn('h-2 w-2 rounded-full', dot)} />
      {label} · <span className="tabular-nums">{percent}%</span>
    </span>
  );
};
