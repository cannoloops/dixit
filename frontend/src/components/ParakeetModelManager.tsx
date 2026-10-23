import React, { useState, useEffect, useRef, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import {
  ModelStatus,
  ParakeetAPI,
  ParakeetDownloadProgressEvent,
  ParakeetModelInfo,
  formatFileSize,
  getModelDisplayInfo,
  getModelDisplayName
} from '../lib/parakeet';
import { t } from '@/i18n/models';
import { ModelList, ModelListError, ModelListLoading, ModelRow, ModelRowState, modelErrorMessage } from './ModelRow';

interface ParakeetModelManagerProps {
  selectedModel?: string;
  onModelSelect?: (modelName: string) => void;
  className?: string;
  autoSave?: boolean;
}

export function ParakeetModelManager({
  selectedModel,
  onModelSelect,
  className = '',
  autoSave = false
}: ParakeetModelManagerProps) {
  const [models, setModels] = useState<ParakeetModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [initialized, setInitialized] = useState(false);
  const [listenersReady, setListenersReady] = useState(false);
  const [downloadingModels, setDownloadingModels] = useState<Set<string>>(new Set());
  const [cancellingModels, setCancellingModels] = useState<Set<string>>(new Set());

  // Refs for stable callbacks
  const onModelSelectRef = useRef(onModelSelect);
  const autoSaveRef = useRef(autoSave);

  useEffect(() => {
    onModelSelectRef.current = onModelSelect;
    autoSaveRef.current = autoSave;
  }, [onModelSelect, autoSave]);

  // Progress throttle map to prevent rapid updates
  const progressThrottleRef = useRef<Map<string, { progress: number; timestamp: number }>>(new Map());
  const latestStatusByModelRef = useRef<Map<string, ModelStatus>>(new Map());
  const clearCancellingModel = (modelName: string) => {
    setCancellingModels(prev => {
      const next = new Set(prev);
      next.delete(modelName);
      return next;
    });
  };

  // Initialize and load models
  useEffect(() => {
    if (initialized || !listenersReady) return;

    const initializeModels = async () => {
      try {
        setLoading(true);
        await ParakeetAPI.init();
        const modelList = await ParakeetAPI.getAvailableModels();
        setModels(modelList.map(model => ({
          ...model,
          status: latestStatusByModelRef.current.get(model.name) ?? model.status
        })));

        setInitialized(true);
      } catch (err) {
        console.error('Failed to initialize Parakeet:', err);
        setError(err instanceof Error ? err.message : t('loadModelsFailed'));
        toast.error(t('loadTranscriptionModelsFailed'), {
          description: err instanceof Error ? err.message : t('unknownError'),
          duration: 5000
        });
      } finally {
        setLoading(false);
      }
    };

    initializeModels();
  }, [initialized, listenersReady, selectedModel, onModelSelect]);

  // Set up event listeners for download progress
  useEffect(() => {
    let disposed = false;
    let registeredUnlisteners: Array<() => void> = [];

    const setupListeners = async () => {
      console.log('[ParakeetModelManager] Setting up event listeners...');

      const registrations = await Promise.allSettled([
        listen<ParakeetDownloadProgressEvent>(
          'parakeet-model-download-progress',
          (event) => {
            const { modelName, progress, status } = event.payload;
            if (status === 'cancelled') {
              latestStatusByModelRef.current.set(modelName, 'Missing');
              progressThrottleRef.current.delete(modelName);
              clearCancellingModel(modelName);
              setDownloadingModels(prev => {
                const next = new Set(prev);
                next.delete(modelName);
                return next;
              });
              setModels(prevModels =>
                prevModels.map(model =>
                  model.name === modelName
                    ? { ...model, status: 'Missing' as ModelStatus }
                    : model
                )
              );
              toast.info(t('downloadCancelled', { name: getModelDisplayName(modelName) }), {
                duration: 3000
              });
              return;
            }

            const modelStatus: ModelStatus = status === 'completed'
              ? 'Available'
              : { Downloading: { progress } };
            latestStatusByModelRef.current.set(modelName, modelStatus);

            const now = Date.now();
            const throttleData = progressThrottleRef.current.get(modelName);
            const shouldUpdate = !throttleData ||
              now - throttleData.timestamp > 300 ||
              Math.abs(progress - throttleData.progress) >= 5;

            if (shouldUpdate) {
              console.log(`[ParakeetModelManager] Progress update for ${modelName}: ${progress}%`);
              progressThrottleRef.current.set(modelName, { progress, timestamp: now });
              setModels(prevModels =>
                prevModels.map(model =>
                  model.name === modelName
                    ? { ...model, status: modelStatus }
                    : model
                )
              );
            }
          }
        ),
        listen<{ modelName: string }>(
          'parakeet-model-download-complete',
          (event) => {
            const { modelName } = event.payload;
            const displayInfo = getModelDisplayInfo(modelName);
            const displayName = displayInfo?.friendlyName || modelName;
            latestStatusByModelRef.current.set(modelName, 'Available');
            clearCancellingModel(modelName);

            setModels(prevModels =>
              prevModels.map(model =>
                model.name === modelName
                  ? { ...model, status: 'Available' as ModelStatus }
                  : model
              )
            );

            setDownloadingModels(prev => {
              const newSet = new Set(prev);
              newSet.delete(modelName);
              return newSet;
            });

            // Clean up throttle data
            progressThrottleRef.current.delete(modelName);

            toast.success(t('modelReady', { name: displayName }), {
              description: t('modelReadyDescription'),
              duration: 4000
            });

            // Auto-select after download using stable refs
            if (onModelSelectRef.current) {
              onModelSelectRef.current(modelName);
              if (autoSaveRef.current) {
                saveModelSelection(modelName);
              }
            }
          }
        ),
        listen<{ modelName: string; error: string }>(
          'parakeet-model-download-error',
          (event) => {
            const { modelName, error } = event.payload;
            const displayInfo = getModelDisplayInfo(modelName);
            const displayName = displayInfo?.friendlyName || modelName;
            latestStatusByModelRef.current.set(modelName, { Error: error });
            clearCancellingModel(modelName);

            setModels(prevModels =>
              prevModels.map(model =>
                model.name === modelName
                  ? { ...model, status: { Error: error } as ModelStatus }
                  : model
              )
            );

            setDownloadingModels(prev => {
              const newSet = new Set(prev);
              newSet.delete(modelName);
              return newSet;
            });

            // Clean up throttle data
            progressThrottleRef.current.delete(modelName);

            toast.error(t('downloadFailedTitle', { name: displayName }), {
              description: error,
              duration: 6000,
              action: {
                label: t('retry'),
                onClick: () => downloadModel(modelName)
              }
            });
          }
        )
      ]);

      const unlisteners = registrations.flatMap(result =>
        result.status === 'fulfilled' ? [result.value] : []
      );
      const failedRegistration = registrations.find(
        (result): result is PromiseRejectedResult => result.status === 'rejected'
      );

      if (disposed || failedRegistration) {
        unlisteners.forEach(unlisten => unlisten());
        if (!disposed && failedRegistration) {
          const message = failedRegistration.reason instanceof Error
            ? failedRegistration.reason.message
            : typeof failedRegistration.reason === 'string'
              ? failedRegistration.reason
              : t('listenFailed');
          setError(message);
          setLoading(false);
        }
        return;
      }

      registeredUnlisteners = unlisteners;
      setListenersReady(true);
    };

    setupListeners();

    return () => {
      console.log('[ParakeetModelManager] Cleaning up event listeners...');
      disposed = true;
      registeredUnlisteners.forEach(unlisten => unlisten());
    };
  }, []); // Empty dependency array - listeners use refs for stable callbacks

  const saveModelSelection = async (modelName: string) => {
    try {
      await invoke('api_save_transcript_config', {
        provider: 'parakeet',
        model: modelName,
        apiKey: null
      });
    } catch (error) {
      console.error('Failed to save model selection:', error);
    }
  };

  const cancelDownload = async (modelName: string) => {
    const displayInfo = getModelDisplayInfo(modelName);
    const displayName = displayInfo?.friendlyName || modelName;

    setCancellingModels(prev => new Set([...prev, modelName]));
    try {
      const outcome = await ParakeetAPI.cancelDownload(modelName);
      if (outcome === 'pending') {
        toast.info(t('cancellingTitle', { name: displayName }), {
          description: t('cancellingDescription'),
          duration: 4000
        });
      }
    } catch (err) {
      clearCancellingModel(modelName);
      console.error('Failed to cancel download:', err);
      toast.error(t('cancelFailed'), {
        description: err instanceof Error ? err.message : t('unknownError'),
        duration: 4000
      });
    }
  };

  const downloadModel = async (modelName: string) => {
    if (downloadingModels.has(modelName) || cancellingModels.has(modelName)) return;
    clearCancellingModel(modelName);

    const displayInfo = getModelDisplayInfo(modelName);
    const displayName = displayInfo?.friendlyName || modelName;

    try {
      setDownloadingModels(prev => new Set([...prev, modelName]));

      setModels(prevModels =>
        prevModels.map(model =>
          model.name === modelName
            ? { ...model, status: { Downloading: { progress: 0 } } as ModelStatus }
            : model
        )
      );

      toast.info(t('downloadingTitle', { name: displayName }), {
        description: t('downloadingDescription'),
        duration: 5000  // Auto-dismiss after 5 seconds
      });

      await ParakeetAPI.downloadModel(modelName);
    } catch (err) {
      console.error('Download failed:', err);
      setDownloadingModels(prev => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });

      const errorMessage = err instanceof Error ? err.message : t('downloadFailedFallback');
      setModels(prev =>
        prev.map(model =>
          model.name === modelName ? { ...model, status: { Error: errorMessage } } : model
        )
      );
    }
  };

  const selectModel = async (modelName: string) => {
    if (onModelSelect) {
      onModelSelect(modelName);
    }

    if (autoSave) {
      await saveModelSelection(modelName);
    }

    const displayInfo = getModelDisplayInfo(modelName);
    const displayName = displayInfo?.friendlyName || modelName;
    toast.success(t('switchedTo', { name: displayName }), {
      duration: 3000
    });
  };

  const deleteModel = async (modelName: string) => {
    const displayInfo = getModelDisplayInfo(modelName);
    const displayName = displayInfo?.friendlyName || modelName;

    try {
      await ParakeetAPI.deleteCorruptedModel(modelName);

      // Refresh models list
      const modelList = await ParakeetAPI.getAvailableModels();
      setModels(modelList);

      toast.success(t('modelDeleted', { name: displayName }), {
        description: t('modelDeletedDescription'),
        duration: 3000
      });

      // If deleted model was selected, clear selection
      if (selectedModel === modelName && onModelSelect) {
        onModelSelect('');
      }
    } catch (err) {
      console.error('Failed to delete model:', err);
      toast.error(t('deleteFailedTitle', { name: displayName }), {
        description: err instanceof Error ? err.message : t('deleteFailedFallback'),
        duration: 4000
      });
    }
  };

  const toRowState = (model: ParakeetModelInfo): ModelRowState => {
    const status = model.status;
    const progress = typeof status === 'object' && 'Downloading' in status ? status.Downloading.progress : null;
    if (progress !== null || cancellingModels.has(model.name)) {
      const shown = progress ?? 0;
      return {
        kind: 'downloading',
        progress: shown,
        cancelling: cancellingModels.has(model.name),
        info: model.size_mb
          ? `${formatFileSize(model.size_mb * shown / 100)} / ${formatFileSize(model.size_mb)}`
          : undefined,
      };
    }
    if (status === 'Available') return { kind: 'installed', active: selectedModel === model.name };
    if (status === 'Missing') return { kind: 'missing' };
    if (typeof status === 'object' && 'Error' in status) {
      return { kind: 'error', message: modelErrorMessage(status.Error) };
    }
    return { kind: 'corrupted', message: t('corruptedMessage') };
  };

  if (loading) return <ModelListLoading />;

  if (error) return <ModelListError message={error} className={className} />;

  // The recommended model comes first, the others follow.
  const recommendedName = 'parakeet-tdt-0.6b-v3-int8';
  const ordered = [...models].sort((a, b) => Number(b.name === recommendedName) - Number(a.name === recommendedName));

  return (
    <div className={`space-y-3 ${className}`}>
      <ModelList>
        {ordered.map(model => {
          const displayInfo = getModelDisplayInfo(model.name);
          return (
            <ModelRow
              key={model.name}
              name={displayInfo?.friendlyName || model.name}
              detail={displayInfo?.tagline || model.description}
              size={formatFileSize(model.size_mb)}
              labels={model.name === recommendedName ? [t('recommended')] : undefined}
              state={toRowState(model)}
              onSelect={() => selectModel(model.name)}
              onDownload={() => downloadModel(model.name)}
              onCancel={() => cancelDownload(model.name)}
              onDelete={() => deleteModel(model.name)}
            />
          );
        })}
      </ModelList>

      {selectedModel && (
        <p className="text-xs text-muted-foreground">
          {t('usingForTranscription', { name: getModelDisplayName(selectedModel) })}
        </p>
      )}
    </div>
  );
}
