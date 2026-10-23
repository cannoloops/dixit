import React, { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Mic, Sparkles, Check, Loader2, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { toast } from 'sonner';
import { getSummaryModelSizeLabel, getSummaryModelSizeMb } from '@/lib/onboarding-summary-model';
import { locale } from '@/i18n';
import { t } from '@/i18n/onboarding';
import type { ParakeetDownloadProgressEvent } from '@/lib/parakeet';

const PARAKEET_MODEL = 'parakeet-tdt-0.6b-v3-int8';

type DownloadStatus = 'waiting' | 'downloading' | 'completed' | 'cancelled' | 'error';

const formatNumber = (n: number) =>
  n.toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });

interface DownloadState {
  status: DownloadStatus;
  progress: number;
  downloadedMb: number;
  totalMb: number;
  speedMbps: number;
  error?: string;
}

export function DownloadProgressStep() {
  const {
    goNext,
    selectedSummaryModel,
    recommendedSummaryModel,
    parakeetDownloaded,
    setParakeetDownloaded,
    summaryModelDownloaded,
    setSummaryModelDownloaded,
    startBackgroundDownloads,
    completeOnboarding,
  } = useOnboarding();

  const [isMac, setIsMac] = useState(false);

  const [parakeetState, setParakeetState] = useState<DownloadState>({
    status: parakeetDownloaded ? 'completed' : 'waiting',
    progress: parakeetDownloaded ? 100 : 0,
    downloadedMb: 0,
    totalMb: 670,
    speedMbps: 0,
  });

  const [summaryState, setSummaryState] = useState<DownloadState>({
    status: summaryModelDownloaded ? 'completed' : 'waiting',
    progress: summaryModelDownloaded ? 100 : 0,
    downloadedMb: 0,
    totalMb: 0,
    speedMbps: 0,
  });

  const [isCompleting, setIsCompleting] = useState(false);
  const parakeetDownloadStartedRef = useRef(false);
  const summaryDownloadStartedRef = useRef(false);
  const retryingRef = useRef(false);
  const retryingSummaryRef = useRef(false);

  // Retry download handler
  const handleRetryDownload = async () => {
    // Prevent multiple simultaneous retries
    if (retryingRef.current) {
      console.log('[DownloadProgressStep] Retry already in progress, ignoring');
      return;
    }

    console.log('[DownloadProgressStep] Retrying Parakeet download');
    retryingRef.current = true;

    // Reset error state
    setParakeetState((prev) => ({
      ...prev,
      status: 'waiting',
      error: undefined,
      progress: 0,
      downloadedMb: 0,
      speedMbps: 0,
    }));

    try {
      await invoke('parakeet_retry_download', { modelName: PARAKEET_MODEL });
      // Progress events will update state
    } catch (error) {
      console.error('[DownloadProgressStep] Retry failed:', error);
      setParakeetState((prev) => ({
        ...prev,
        status: 'error',
        error: error instanceof Error ? error.message : t('retryFailed'),
      }));

      toast.error(t('retryFailed'), {
        description: t('downloadFailedRemedy'),
      });
    } finally {
      // Allow retry again after 2 seconds
      setTimeout(() => {
        retryingRef.current = false;
      }, 2000);
    }
  };

  // Retry summary download handler
  const handleRetrySummaryDownload = async () => {
    // Prevent multiple simultaneous retries
    if (retryingSummaryRef.current) {
      console.log('[DownloadProgressStep] Summary retry already in progress, ignoring');
      return;
    }

    console.log('[DownloadProgressStep] Retrying summary model download');
    retryingSummaryRef.current = true;

    // Reset error state
    setSummaryState((prev) => ({
      ...prev,
      status: 'downloading',
      error: undefined,
      progress: 0,
      downloadedMb: 0,
      totalMb: getSummaryModelSizeMb(selectedSummaryModel || recommendedSummaryModel),
      speedMbps: 0,
    }));

    try {
      // Call download command directly (no retry command exists for built-in AI)
      const modelName = selectedSummaryModel;
      if (!modelName) {
        throw new Error(t('summaryNotReady'));
      }
      await invoke('builtin_ai_download_model', { modelName });
    } catch (error) {
      console.error('[DownloadProgressStep] Summary retry failed:', error);
      setSummaryState((prev) => ({
        ...prev,
        status: 'error',
        error: error instanceof Error ? error.message : t('retryFailed'),
      }));

      toast.error(t('retryFailedSummary'), {
        description: t('downloadFailedRemedy'),
      });
    } finally {
      // Allow retry again after 2 seconds
      setTimeout(() => {
        retryingSummaryRef.current = false;
      }, 2000);
    }
  };

  // Detect platform on mount
  useEffect(() => {
    const checkPlatform = async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };

    checkPlatform();
  }, []);

  // Start the required transcription model immediately; summary readiness must not block it.
  useEffect(() => {
    if (parakeetDownloadStartedRef.current) return;
    parakeetDownloadStartedRef.current = true;

    if (!parakeetDownloaded) {
      setParakeetState((prev) => ({ ...prev, status: 'downloading' }));
    }

    startBackgroundDownloads({
      includeParakeet: true,
      includeSummary: false,
    }).catch((error) => {
      console.error('Failed to start Parakeet download:', error);
      if (!parakeetDownloaded) {
        setParakeetState((prev) => ({ ...prev, status: 'error', error: String(error) }));
      }
    });
  }, []);

  // Start the selected summary model only after the backend recommendation is known.
  useEffect(() => {
    if (summaryDownloadStartedRef.current) return;
    if (!selectedSummaryModel) return;
    summaryDownloadStartedRef.current = true;

    startSummaryDownload();
  }, [selectedSummaryModel]);

  // Listen to Parakeet download progress
  useEffect(() => {
    const unlistenProgress = listen<ParakeetDownloadProgressEvent>(
      'parakeet-model-download-progress',
      (event) => {
        const { modelName, progress, downloaded_mb, total_mb, speed_mbps, status } = event.payload;
        if (modelName !== PARAKEET_MODEL) return;

        if (status === 'cancelled') {
          setParakeetState((prev) => ({
            ...prev,
            status: 'cancelled',
            progress: 0,
            downloadedMb: 0,
            speedMbps: 0,
          }));
          setParakeetDownloaded(false);
          return;
        }

        setParakeetState((prev) => ({
          ...prev,
          status: status === 'completed' ? 'completed' : 'downloading',
          progress,
          downloadedMb: downloaded_mb ?? prev.downloadedMb,
          totalMb: total_mb ?? prev.totalMb,
          speedMbps: speed_mbps ?? prev.speedMbps,
        }));

        if (status === 'completed') {
          setParakeetDownloaded(true);
        }
      }
    );

    const unlistenComplete = listen<{ modelName: string }>(
      'parakeet-model-download-complete',
      (event) => {
        if (event.payload.modelName === PARAKEET_MODEL) {
          setParakeetState((prev) => ({ ...prev, status: 'completed', progress: 100 }));
          setParakeetDownloaded(true);
        }
      }
    );

    const unlistenError = listen<{ modelName: string; error: string }>(
      'parakeet-model-download-error',
      (event) => {
        if (event.payload.modelName === PARAKEET_MODEL) {
          setParakeetState((prev) => ({
            ...prev,
            status: 'error',
            error: event.payload.error,
          }));
        }
      }
    );

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  // Listen to Summary Model download progress (always downloading for builtin-ai)
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
      if (selectedSummaryModel && model === selectedSummaryModel) {
        setSummaryState((prev) => ({
          ...prev,
          status: status === 'completed'
            ? 'completed'
            : status === 'error'
            ? 'error'
            : 'downloading',
          progress,
          downloadedMb: downloaded_mb ?? prev.downloadedMb,
          totalMb: (total_mb ?? prev.totalMb) || getSummaryModelSizeMb(model),
          speedMbps: speed_mbps ?? prev.speedMbps,
          error: status === 'error' ? error : undefined,
        }));

        if (status === 'completed' || progress >= 100) {
          setSummaryModelDownloaded(true);
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [selectedSummaryModel]);

  useEffect(() => {
    const modelForSize = selectedSummaryModel || recommendedSummaryModel;
    if (!modelForSize) return;

    setSummaryState((prev) => ({
      ...prev,
      status: summaryModelDownloaded
        ? 'completed'
        : prev.status === 'completed'
        ? 'waiting'
        : prev.status,
      progress: summaryModelDownloaded
        ? 100
        : prev.status === 'completed'
        ? 0
        : prev.progress,
      totalMb: prev.totalMb || getSummaryModelSizeMb(modelForSize),
    }));
  }, [selectedSummaryModel, recommendedSummaryModel, summaryModelDownloaded]);

  const startSummaryDownload = async () => {
    if (!summaryModelDownloaded && selectedSummaryModel) {
      try {
        setSummaryState((prev) => ({
          ...prev,
          status: 'downloading',
          totalMb: getSummaryModelSizeMb(selectedSummaryModel),
        }));
        await startBackgroundDownloads({
          includeParakeet: false,
          includeSummary: true,
          summaryModel: selectedSummaryModel,
        });
      } catch (error) {
        console.error('Failed to start summary model download:', error);
        setSummaryState((prev) => ({ ...prev, status: 'error', error: String(error) }));
      }
    }
  };

  const handleContinue = async () => {
    // Verify actual model availability (catches state drift)
    try {
      await invoke('parakeet_init');
      const actuallyAvailable = await invoke<boolean>('parakeet_has_available_models');

      if (actuallyAvailable && !parakeetDownloaded) {
        console.log('[DownloadProgressStep] Model available but state not updated');
        setParakeetDownloaded(true);
        setParakeetState((prev) => ({
          ...prev,
          status: 'completed',
          progress: 100,
        }));
      } else if (
        !actuallyAvailable &&
        (parakeetState.status === 'error' || parakeetState.status === 'cancelled')
      ) {
        toast.error(t('engineRequiredTitle'), {
          description: t('engineRequiredBody'),
        });
        return;
      }
    } catch (error) {
      console.warn('[DownloadProgressStep] Failed to verify model:', error);
    }

    // Check if downloads are complete for toast notification
    const downloadsComplete = parakeetState.status === 'completed' &&
      summaryState.status === 'completed';

    // Show toast if downloads still in progress
    if (!downloadsComplete) {
      toast.info(t('backgroundTitle'), {
        description: t('backgroundBody'),
        duration: 5000,
      });
    }

    if (isMac) {
      // macOS: Go to Permissions step (will complete after permissions granted)
      goNext();
    } else {
      // Non-macOS: Complete onboarding immediately (downloads continue in background)
      setIsCompleting(true);
      try {
        await completeOnboarding();

        // Small delay to ensure state is saved before reload
        await new Promise(resolve => setTimeout(resolve, 100));

        window.location.reload();
      } catch (error) {
        console.error('Failed to complete onboarding:', error);
        toast.error(t('completeFailedTitle'), {
          description: t('completeFailedBody'),
        });
        setIsCompleting(false);
      }
    }
  };

  return (
    <OnboardingContainer
      title={t('downloadsTitle')}
      description={t('downloadsDescription')}
      step={3}
      totalSteps={isMac ? 4 : 3}
    >
      <ul className="divide-y divide-border border-y border-border">
        <DownloadRow
          title={t('transcriptionEngine')}
          icon={<Mic className="h-4 w-4" />}
          state={parakeetState}
          modelSize="~670 MB"
          onRetry={handleRetryDownload}
        />
        <DownloadRow
          title={t('summaryEngine')}
          icon={<Sparkles className="h-4 w-4" />}
          state={summaryState}
          modelSize={getSummaryModelSizeLabel(selectedSummaryModel || recommendedSummaryModel, formatNumber)}
          sizeUnit="MiB"
          onRetry={handleRetrySummaryDownload}
        />
      </ul>

      {parakeetDownloaded && !summaryModelDownloaded && (
        <p className="mt-4 text-xs text-muted-foreground">{t('continueWhileDownloading')}</p>
      )}

      <div className="mt-8 flex justify-end">
        <Button size="lg" onClick={handleContinue} disabled={!parakeetDownloaded || isCompleting}>
          {(isCompleting || !parakeetDownloaded) && <Loader2 className="h-4 w-4 animate-spin" />}
          {parakeetDownloaded ? t('continue') : t('downloading')}
        </Button>
      </div>
    </OnboardingContainer>
  );
}

interface DownloadRowProps {
  title: string;
  icon: React.ReactNode;
  state: DownloadState;
  modelSize: string;
  sizeUnit?: string;
  onRetry: () => void;
}

/** One model download: name and size, state on the right, a bronze bar while it runs, the remedy if it fails. */
function DownloadRow({ title, icon, state, modelSize, sizeUnit = 'MB', onRetry }: DownloadRowProps) {
  const showBar = state.status === 'downloading' || state.status === 'completed';
  const isCancelled = state.status === 'cancelled';
  const failed = state.status === 'error' || isCancelled;

  return (
    <li className="space-y-2 py-3">
      <div className="flex items-center gap-3">
        <span className="flex-shrink-0 text-muted-foreground">{icon}</span>
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium">{title}</p>
          <p className="text-xs tabular-nums text-muted-foreground">{modelSize}</p>
        </div>
        <div className="flex-shrink-0 text-xs">
          {state.status === 'waiting' && <span className="text-muted-foreground">{t('statusWaiting')}</span>}
          {state.status === 'downloading' && (
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" aria-label={t('downloading')} />
          )}
          {state.status === 'completed' && (
            <span className="inline-flex items-center gap-1.5 font-medium text-primary">
              <Check className="h-4 w-4" aria-hidden />
              {t('statusReady')}
            </span>
          )}
          {state.status === 'error' && <span className="font-medium text-destructive">{t('statusFailed')}</span>}
          {isCancelled && <span className="text-muted-foreground">{t('statusCancelled')}</span>}
        </div>
      </div>

      {showBar && (
        <div className="space-y-1.5">
          <Progress value={state.progress} aria-label={title} className="h-1.5 rounded-sm bg-secondary" />
          <div className="flex items-center justify-between text-xs tabular-nums text-muted-foreground">
            <span>
              {formatNumber(state.downloadedMb)} {sizeUnit} / {formatNumber(state.totalMb)} {sizeUnit}
            </span>
            <span className="flex items-center gap-2">
              {state.speedMbps > 0 && (
                <span>
                  {formatNumber(state.speedMbps)} {sizeUnit}/s
                </span>
              )}
              <span className="font-medium text-foreground">{Math.round(state.progress)}%</span>
            </span>
          </div>
        </div>
      )}

      {failed && (
        <div className="space-y-2">
          <p className="text-sm text-destructive">
            {isCancelled ? t('downloadCancelled') : t('downloadFailed')}{' '}
            {isCancelled ? t('downloadCancelledRemedy') : t('downloadFailedRemedy')}
          </p>
          {state.error && <p className="break-words text-xs text-destructive">{state.error}</p>}
          <Button variant="outline" size="sm" onClick={onRetry}>
            <RefreshCw className="h-4 w-4" />
            {t('tryAgain')}
          </Button>
        </div>
      )}
    </li>
  );
}
