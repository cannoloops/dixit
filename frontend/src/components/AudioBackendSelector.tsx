import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '@/i18n/settings';

export interface BackendInfo {
  id: string;
  name: string;
  description: string;
}

interface AudioBackendSelectorProps {
  currentBackend?: string;
  onBackendChange?: (backend: string) => void;
  disabled?: boolean;
}

export function AudioBackendSelector({
  currentBackend: propBackend,
  onBackendChange,
  disabled = false,
}: AudioBackendSelectorProps) {
  const [backends, setBackends] = useState<BackendInfo[]>([]);
  const [currentBackend, setCurrentBackend] = useState<string>('coreaudio');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load available backends and current selection
  useEffect(() => {
    const loadBackends = async () => {
      try {
        setLoading(true);
        setError(null);

        // Get backend info (includes name and description)
        const backendInfo = await invoke<BackendInfo[]>('get_audio_backend_info');
        setBackends(backendInfo);

        // Get current backend if not provided via props
        if (!propBackend) {
          const current = await invoke<string>('get_current_audio_backend');
          setCurrentBackend(current);
        } else {
          setCurrentBackend(propBackend);
        }
      } catch (err) {
        console.error('Failed to load audio backends:', err);
        setError(t('backendLoadError'));
      } finally {
        setLoading(false);
      }
    };

    loadBackends();
  }, [propBackend]);

  // Handle backend selection
  const handleBackendChange = async (backendId: string) => {
    try {
      setError(null);
      await invoke('set_audio_backend', { backend: backendId });
      setCurrentBackend(backendId);

      // Notify parent component
      if (onBackendChange) {
        onBackendChange(backendId);
      }

      console.log(`Audio backend changed to: ${backendId}`);
    } catch (err) {
      console.error('Failed to set audio backend:', err);
      setError(t('backendChangeError'));
    }
  };

  // Only show selector if there are multiple backends
  if (loading) {
    return (
      <div className="animate-pulse py-3">
        <div className="mb-2 h-4 w-32 rounded-sm bg-muted"></div>
        <div className="h-8 rounded-sm bg-muted"></div>
      </div>
    );
  }

  // Hide if only one backend available
  if (backends.length <= 1) {
    return null;
  }

  return (
    <div className="space-y-2 py-2.5">
      <div>
        <div className="text-sm font-medium leading-snug">{t('backendTitle')}</div>
        <p className="mt-0.5 text-xs text-muted-foreground">{t('backendDesc')}</p>
      </div>

      {error && (
        <p role="alert" className="text-xs text-destructive">
          {error}
        </p>
      )}

      <div role="radiogroup" aria-label={t('backendTitle')} className="divide-y divide-border">
        {backends.map((backend) => {
          // Disable Core Audio option
          const isCoreAudio = backend.id === 'screencapturekit';
          const isDisabled = disabled || isCoreAudio;

          return (
            <label
              key={backend.id}
              className={`flex items-start gap-3 py-2 ${isDisabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}
            >
              <input
                type="radio"
                name="audioBackend"
                value={backend.id}
                checked={currentBackend === backend.id}
                onChange={() => handleBackendChange(backend.id)}
                disabled={isDisabled}
                className="mt-0.5 h-4 w-4 accent-primary focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
              />
              <div className="flex-1">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">{backend.name}</span>
                  {currentBackend === backend.id && (
                    <span className="text-xs text-muted-foreground">{t('backendActive')}</span>
                  )}
                  {isCoreAudio && (
                    <span className="text-xs text-muted-foreground">{t('backendUnavailable')}</span>
                  )}
                </div>
                <p className="text-xs text-muted-foreground">{backend.description}</p>
              </div>
            </label>
          );
        })}
      </div>
    </div>
  );
}
