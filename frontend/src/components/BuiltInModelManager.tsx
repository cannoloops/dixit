'use client';

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { formatSummaryModelSizeLabelFromMb } from '@/lib/onboarding-summary-model';
import { t } from '@/i18n/models';
import { ModelList, ModelListLoading, ModelRow, ModelRowState, modelErrorMessage } from './ModelRow';

interface ModelInfo {
  name: string;
  display_name: string;
  status: {
    type: 'not_downloaded' | 'downloading' | 'available' | 'corrupted' | 'error';
    progress?: number;
  };
  size_mb: number;
  context_size: number;
  description: string;
  gguf_file: string;
}

interface DownloadProgressInfo {
  downloadedMb: number;
  totalMb: number;
  speedMbps: number;
}

interface BuiltInModelManagerProps {
  selectedModel: string;
  onModelSelect: (model: string) => void;
  layout?: 'inline' | 'dialog';
}

export function BuiltInModelManager({
  selectedModel,
  onModelSelect,
  layout = 'inline',
}: BuiltInModelManagerProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [hasFetched, setHasFetched] = useState<boolean>(false);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [downloadProgressInfo, setDownloadProgressInfo] = useState<Record<string, DownloadProgressInfo>>({});
  const [downloadingModels, setDownloadingModels] = useState<Set<string>>(new Set());

  const fetchModels = async () => {
    try {
      setIsLoading(true);
      const data = (await invoke('builtin_ai_list_models')) as ModelInfo[];
      setModels(data);

      // Auto-select first available model if none selected
      if (data.length > 0 && !selectedModel) {
        const firstAvailable = data.find((m) => m.status.type === 'available');
        if (firstAvailable) {
          onModelSelect(firstAvailable.name);
        }
      }
    } catch (error) {
      console.error('Failed to fetch built-in AI models:', error);
      toast.error(t('loadModelsFailed'));
    } finally {
      setIsLoading(false);
      setHasFetched(true);
    }
  };

  useEffect(() => {
    fetchModels();
  }, []);

  // Listen for download progress events
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen('builtin-ai-download-progress', (event: any) => {
        const { model, progress, downloaded_mb, total_mb, speed_mbps, status } = event.payload;

        // Update percentage progress
        setDownloadProgress((prev) => ({
          ...prev,
          [model]: progress,
        }));

        // Update detailed progress info (MB, speed)
        setDownloadProgressInfo((prev) => ({
          ...prev,
          [model]: {
            downloadedMb: downloaded_mb ?? 0,
            totalMb: total_mb ?? 0,
            speedMbps: speed_mbps ?? 0,
          },
        }));

        // Handle downloading status - restore downloadingModels state on modal reopen
        if (status === 'downloading') {
          setDownloadingModels((prev) => {
            if (!prev.has(model)) {
              const newSet = new Set(prev);
              newSet.add(model);
              return newSet;
            }
            return prev;
          });
        }

        // Handle completed status
        if (status === 'completed') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          setDownloadProgressInfo((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          // Refresh models list
          fetchModels();
          toast.success(t('builtinDownloaded', { name: model }));
        }

        // Handle cancelled status
        if (status === 'cancelled') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          setDownloadProgressInfo((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          // Refresh models list
          fetchModels();
        }

        // Handle error status
        if (status === 'error') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          setDownloadProgressInfo((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });

          // Update model status to error locally instead of fetching from backend
          // Backend doesn't persist error status, so fetchModels() would return not_downloaded
          setModels((prevModels) =>
            prevModels.map((m) =>
              m.name === model
                ? {
                    ...m,
                    status: {
                      type: 'error',
                      progress: 0,
                    } as any,
                  }
                : m
            )
          );

          // Don't show error toast here - DownloadProgressToast already handles it
          // Don't call fetchModels() - it would overwrite error status with not_downloaded
        }
      });
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const downloadModel = async (modelName: string) => {
    try {
      // Optimistically add to downloadingModels for immediate UI feedback
      setDownloadingModels((prev) => new Set([...prev, modelName]));

      await invoke('builtin_ai_download_model', { modelName });
    } catch (error) {
      console.error('Failed to download model:', error);

      // Check if this is a cancellation error (starts with "CANCELLED:")
      const errorMsg = String(error);
      if (errorMsg.startsWith('CANCELLED:')) {
        // Cancel handler already removed from downloadingModels
        // Don't show error toast for cancellations - cancel function already shows info toast
        return;
      }

      // For real errors, show toast and remove from downloading
      toast.error(t('downloadFailedTitle', { name: modelName }));

      setDownloadingModels((prev) => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });

      // Refresh model list to get updated Error status from backend
      fetchModels();
    }
  };

  const cancelDownload = async (modelName: string) => {
    try {
      await invoke('builtin_ai_cancel_download', { modelName });
      toast.info(t('downloadCancelled', { name: modelName }));
      setDownloadingModels((prev) => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });
    } catch (error) {
      console.error('Failed to cancel download:', error);
    }
  };

  const deleteModel = async (modelName: string) => {
    try {
      await invoke('builtin_ai_delete_model', { modelName });
      toast.success(t('builtinDeleted', { name: modelName }));
      fetchModels();
    } catch (error) {
      console.error('Failed to delete model:', error);
      toast.error(t('deleteFailedTitle', { name: modelName }));
    }
  };
  const toRowState = (model: ModelInfo): ModelRowState => {
    const progress = downloadProgress[model.name];
    const info = downloadProgressInfo[model.name];

    if (downloadingModels.has(model.name)) {
      return {
        kind: 'downloading',
        progress: progress ?? 0,
        info:
          info?.totalMb > 0
            ? `${info.downloadedMb.toFixed(1)} / ${info.totalMb.toFixed(1)} MiB${info.speedMbps > 0 ? ` · ${info.speedMbps.toFixed(1)} MiB/s` : ''}`
            : undefined,
      };
    }
    switch (model.status.type) {
      case 'available':
        return { kind: 'installed', active: selectedModel === model.name };
      case 'corrupted':
        return { kind: 'corrupted', message: t('builtinCorrupted') };
      case 'error':
        return {
          kind: 'error',
          message:
            'Error' in model.status && typeof model.status.Error === 'string'
              ? modelErrorMessage(model.status.Error)
              : t('builtinGenericError'),
        };
      default:
        return { kind: 'missing' };
    }
  };

  // Don't show loading spinner if we have downloads in progress - show the model list instead
  if (isLoading && downloadingModels.size === 0) {
    return <ModelListLoading />;
  }

  // Only show "no models" message after fetch has completed
  if (hasFetched && models.length === 0) {
    return <p className="text-sm text-muted-foreground">{t('builtinNoModels')}</p>;
  }

  return (
    <div>
      <h4 className="mb-2 text-sm font-semibold">{t('builtinModels')}</h4>

      <div className={cn(layout === 'dialog' && 'max-h-[50vh] overflow-y-auto pr-2')}>
        <ModelList>
          {models.map((model) => {
            const sizeLabel = formatSummaryModelSizeLabelFromMb(model.size_mb);
            const detail = [model.description, t('contextTokens', { count: model.context_size })]
              .filter(Boolean)
              .join(' · ');
            const state = toRowState(model);
            return (
              <ModelRow
                key={model.name}
                name={model.display_name || model.name}
                detail={detail}
                size={sizeLabel}
                state={state}
                onSelect={() => onModelSelect(model.name)}
                onDownload={() => downloadModel(model.name)}
                onCancel={() => cancelDownload(model.name)}
                // The model in use cannot be deleted.
                onDelete={
                  state.kind === 'installed' && state.active ? undefined : () => deleteModel(model.name)
                }
              />
            );
          })}
        </ModelList>
      </div>
    </div>
  );
}
