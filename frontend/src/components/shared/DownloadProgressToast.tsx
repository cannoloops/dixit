'use client';

import React, { useEffect, useState, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { X, Download, Check } from 'lucide-react';
import { getDownloadTotalMb } from '@/lib/onboarding-summary-model';
import type { ParakeetDownloadProgressEvent } from '@/lib/parakeet';
import { locale } from '@/i18n';
import { t } from '@/i18n/models';

interface DownloadProgress {
  modelName: string;
  displayName: string;
  progress: number;
  downloadedMb: number;
  totalMb: number;
  speedMbps: number;
  status: 'downloading' | 'completed' | 'error' | 'cancelled';
  unitLabel?: string;
  error?: string;
}

// Categorize error messages for better user experience
function categorizeError(error: string): string {
  const lowerError = error.toLowerCase();

  if (lowerError.includes('network') ||
    lowerError.includes('connection') ||
    lowerError.includes('timeout') ||
    lowerError.includes('failed to start download')) {
    return t('errNetwork');
  }

  if (lowerError.includes('status:') || lowerError.includes('http')) {
    return t('errServer');
  }

  if (lowerError.includes('disk') ||
    lowerError.includes('write') ||
    lowerError.includes('file')) {
    return t('errStorage');
  }

  if (lowerError.includes('invalid') || lowerError.includes('validation')) {
    return t('errValidation');
  }

  // Fallback to original error
  return error;
}

// Custom toast component for download progress
function DownloadToastContent({
  download,
  onDismiss,
}: {
  download: DownloadProgress;
  onDismiss?: () => void;
}) {
  const isComplete = download.status === 'completed';
  const hasError = download.status === 'error';
  const isCancelled = download.status === 'cancelled';
  const unitLabel = download.unitLabel ?? 'MB';

  const fmt = (n: number) => n.toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });

  return (
    <div className="flex w-full max-w-sm items-center gap-3 rounded-md border border-border bg-popover p-3 text-popover-foreground shadow-(--shadow-float)">
      {/* Icon */}
      <div className="shrink-0">
        {isComplete ? (
          <Check className="h-4 w-4 text-primary" />
        ) : hasError ? (
          <X className="h-4 w-4 text-destructive" />
        ) : isCancelled ? (
          <X className="h-4 w-4 text-muted-foreground" />
        ) : (
          <Download className="h-4 w-4 text-muted-foreground" />
        )}
      </div>

      {/* Content */}
      <div className="min-w-0 flex-1">
        <p className="mb-1 truncate text-sm font-medium text-foreground">{download.displayName}</p>

        {hasError ? (
          <p className="text-xs text-destructive">{download.error || t('toastDownloadFailed')}</p>
        ) : isComplete ? (
          <p className="text-xs text-muted-foreground">{t('toastDownloadComplete')}</p>
        ) : isCancelled ? (
          <p className="text-xs text-muted-foreground">{t('toastDownloadCancelled')}</p>
        ) : (
          <>
            {/* Progress bar */}
            <div className="mb-1.5 h-1.5 w-full overflow-hidden rounded-sm bg-muted">
              <div
                className="h-full bg-primary transition-[width] duration-200"
                style={{ width: `${download.progress}%` }}
              />
            </div>

            {/* Progress text */}
            <div className="flex items-center justify-between text-xs tabular-nums text-muted-foreground">
              <span>
                {fmt(download.downloadedMb)} / {fmt(download.totalMb)} {unitLabel}
              </span>
              <span className="flex items-center gap-1">
                {download.speedMbps > 0 && (
                  <span>{fmt(download.speedMbps)} {unitLabel}/s</span>
                )}
                <span className="font-medium text-foreground">{Math.round(download.progress)}%</span>
              </span>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

// Hook to manage download progress toasts
export function useDownloadProgressToast() {
  const [downloads, setDownloads] = useState<Map<string, DownloadProgress>>(new Map());
  const [dismissedModels, setDismissedModels] = useState<Set<string>>(new Set());

  const updateDownload = useCallback((modelName: string, data: Partial<DownloadProgress>) => {
    setDownloads((prev) => {
      const updated = new Map(prev);
      const existing = updated.get(modelName) || {
        modelName,
        displayName: modelName,
        progress: 0,
        downloadedMb: 0,
        totalMb: 0,
        speedMbps: 0,
        status: 'downloading' as const,
      };

      updated.set(modelName, { ...existing, ...data });
      return updated;
    });
  }, []);

  const cleanupDownload = useCallback((modelName: string, delay: number = 4000) => {
    // Remove download from map after delay (allows toast to show and auto-dismiss)
    setTimeout(() => {
      setDownloads((prev) => {
        const updated = new Map(prev);
        updated.delete(modelName);
        return updated;
      });
    }, delay);
  }, []);

  const showDownloadToast = useCallback((download: DownloadProgress) => {
    const toastId = `download-${download.modelName}`;

    // Determine duration based on status
    const getDuration = () => {
      switch (download.status) {
        case 'completed': return 3000;      // 3 seconds
        case 'cancelled': return 5000;      // 5 seconds
        case 'error': return 10000;         // 10 seconds
        case 'downloading': return Infinity; // Manual dismiss only
      }
    };

    // Dismiss handler
    const dismissToast = () => {
      toast.dismiss(toastId);
      setDismissedModels(prev => {
        const next = new Set(prev);
        next.add(download.modelName);
        return next;
      });
    };

    toast.custom(
      () => (
        <DownloadToastContent
          download={download}
          onDismiss={dismissToast}
        />
      ),
      {
        position: 'top-right',
        id: toastId,
        duration: getDuration(),
      }
    );
  }, []);

  // Effect to handle toast visibility based on dismissed state
  useEffect(() => {
    downloads.forEach((download) => {
      // If model was dismissed and is still downloading, don't show it
      if (dismissedModels.has(download.modelName) && download.status === 'downloading') {
        return;
      }

      // If status changed to completed or error, we might want to show it even if dismissed previously
      // (Optional: remove from dismissed set if you want to force show completion)
      if (download.status === 'completed' || download.status === 'error') {
        if (dismissedModels.has(download.modelName)) {
          // Remove from dismissed so we can show the completion/error toast
          setDismissedModels(prev => {
            const next = new Set(prev);
            next.delete(download.modelName);
            return next;
          });
        }
      }

      showDownloadToast(download);
    });
  }, [downloads, dismissedModels, showDownloadToast]);

  // Listen to Parakeet download events
  useEffect(() => {
    const unlistenProgress = listen<ParakeetDownloadProgressEvent>(
      'parakeet-model-download-progress',
      (event) => {
        const { modelName, progress, downloaded_mb, total_mb, speed_mbps, status } = event.payload;
        const downloadData: DownloadProgress = {
          modelName,
          displayName: t('toastTranscriptionModel'),
          progress,
          downloadedMb: downloaded_mb ?? 0,
          totalMb: total_mb ?? 670,
          speedMbps: speed_mbps ?? 0,
          status: status === 'cancelled'
            ? 'cancelled'
            : status === 'completed'
              ? 'completed'
              : 'downloading',
        };

        updateDownload(modelName, downloadData);
        if (downloadData.status === 'cancelled') {
          cleanupDownload(modelName, 6000);
        }
      }
    );

    const unlistenComplete = listen<{ modelName: string }>(
      'parakeet-model-download-complete',
      (event) => {
        const { modelName } = event.payload;
        const downloadData: DownloadProgress = {
          modelName,
          displayName: t('toastTranscriptionModel'),
          progress: 100,
          downloadedMb: 670,
          totalMb: 670,
          speedMbps: 0,
          status: 'completed',
        };
        updateDownload(modelName, downloadData);
        // Clean up after 4 seconds (completion toast duration is 3s + 1s buffer)
        cleanupDownload(modelName, 4000);
      }
    );

    const unlistenError = listen<{ modelName: string; error: string }>(
      'parakeet-model-download-error',
      (event) => {
        const { modelName, error } = event.payload;
        const downloadData: DownloadProgress = {
          modelName,
          displayName: t('toastTranscriptionModel'),
          progress: 0,
          downloadedMb: 0,
          totalMb: 670,
          speedMbps: 0,
          status: 'error',
          error: categorizeError(error),
        };
        updateDownload(modelName, downloadData);
        // Clean up after 11 seconds (error toast duration is 10s + 1s buffer)
        cleanupDownload(modelName, 11000);
      }
    );

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, [updateDownload, cleanupDownload]);

  // Listen to Built-in AI summary model download events
  useEffect(() => {
    const unlisten = listen<{
      model: string;
      progress: number;
      downloaded_mb?: number;
      total_mb?: number;
      speed_mbps?: number;
      status: string;
      error?: string;
    }>('builtin-ai-download-progress', (event) => {
      const { model, progress, downloaded_mb, total_mb, speed_mbps, status, error } = event.payload;

      const downloadData: DownloadProgress = {
        modelName: model,
        displayName: t('toastSummaryModel', { model }),
        progress: progress ?? 0,
        downloadedMb: downloaded_mb ?? 0,
        totalMb: getDownloadTotalMb(total_mb, model),
        speedMbps: speed_mbps ?? 0,
        unitLabel: 'MiB',
        status: status === 'completed' || progress >= 100
          ? 'completed'
          : status === 'cancelled'
            ? 'cancelled'
            : status === 'error'
              ? 'error'
              : 'downloading',
        error: status === 'error' ? categorizeError(error || t('toastDownloadFailed')) : undefined,
      };

      updateDownload(model, downloadData);

      // Clean up finished downloads after delay to prevent endless toasts
      if (downloadData.status === 'completed') {
        cleanupDownload(model, 4000);  // 3s toast + 1s buffer
      } else if (downloadData.status === 'error') {
        cleanupDownload(model, 11000); // 10s toast + 1s buffer
      } else if (downloadData.status === 'cancelled') {
        cleanupDownload(model, 6000);  // 5s toast + 1s buffer
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [updateDownload, cleanupDownload]);

  return { downloads };
}

// Component to initialize download toast listeners at app level
export function DownloadProgressToastProvider() {
  useDownloadProgressToast();
  return null;
}
