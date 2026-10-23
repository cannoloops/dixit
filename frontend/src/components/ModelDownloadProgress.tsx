import React from 'react';
import { Check, Loader2 } from 'lucide-react';
import { ModelStatus, formatFileSize } from '../lib/whisper';
import { t } from '@/i18n/models';
import { DownloadBar } from './ModelRow';

interface ModelDownloadProgressProps {
  status: ModelStatus;
  modelName: string;
  onCancel?: () => void;
}

export function ModelDownloadProgress({ status, modelName, onCancel }: ModelDownloadProgressProps) {
  if (typeof status !== 'object' || !('Downloading' in status)) {
    return null;
  }

  const progress = status.Downloading.progress;
  const isCompleted = progress >= 100;

  return (
    <div className="space-y-1.5 border-y border-border py-2">
      <p className="flex items-center gap-2 text-sm font-medium text-foreground">
        {isCompleted ? <Check className="h-4 w-4 text-primary" /> : <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />}
        {isCompleted ? t('finalizing') : t('downloadingModel', { name: modelName })}
      </p>
      <DownloadBar progress={progress} />
      {isCompleted && <p className="text-xs text-muted-foreground">{t('downloadCompletedLoading')}</p>}
    </div>
  );
}

interface ProgressRingProps {
  progress: number;
  size?: number;
  strokeWidth?: number;
}

export function ProgressRing({ progress, size = 40, strokeWidth = 3 }: ProgressRingProps) {
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const strokeDasharray = circumference;
  const strokeDashoffset = circumference - (progress / 100) * circumference;

  return (
    <div className="relative inline-flex items-center justify-center">
      <svg
        width={size}
        height={size}
        className="-rotate-90"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(progress)}
      >
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          className="text-muted"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          fill="transparent"
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          className="text-primary transition-[stroke-dashoffset] duration-200"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          strokeDasharray={strokeDasharray}
          strokeDashoffset={strokeDashoffset}
          strokeLinecap="butt"
          fill="transparent"
        />
      </svg>
      <span className="absolute text-xs font-medium tabular-nums text-foreground">
        {Math.round(progress)}%
      </span>
    </div>
  );
}

interface DownloadSummaryProps {
  totalModels: number;
  downloadedModels: number;
  totalSizeMb: number;
}

export function DownloadSummary({ totalModels, downloadedModels, totalSizeMb }: DownloadSummaryProps) {
  return (
    <div className="space-y-1 border-y border-border py-2 text-sm">
      <div className="flex items-center justify-between">
        <span className="tabular-nums text-foreground">
          {t('modelsAvailable', { downloaded: downloadedModels, total: totalModels })}
        </span>
        <span className="tabular-nums text-muted-foreground">
          {t('totalSize', { size: formatFileSize(totalSizeMb) })}
        </span>
      </div>
      {downloadedModels > 0 && (
        <p className="text-xs text-muted-foreground">{t('modelsRunLocally')}</p>
      )}
    </div>
  );
}
