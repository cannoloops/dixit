'use client';

import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Loader2, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { locale } from '@/i18n';
import { t } from '@/i18n/onboarding';

const gb = (bytes: number) =>
  `${(bytes / 1e9).toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} GB`;

/** Dixit: the models are not in the .app. Until they are on disk, download them (with progress) instead of the app. */
export function BaseModelsGate({ children }: { children: React.ReactNode }) {
  const [missing, setMissing] = useState<number | null>(null);
  const [progress, setProgress] = useState({ downloadedBytes: 0, totalBytes: 0 });
  const [error, setError] = useState<string | null>(null);

  const download = useCallback(async () => {
    setError(null);
    try {
      await invoke('base_models_download');
      setMissing(await invoke<number>('base_models_missing_bytes'));
    } catch (e) {
      if (!String(e).includes('already in progress')) setError(String(e));
    }
  }, []);

  useEffect(() => {
    const unlisten = listen<typeof progress>('base-models-progress', (e) => {
      setProgress(e.payload);
      // Also covers a reload mid-download, when this page did not start it.
      if (e.payload.downloadedBytes >= e.payload.totalBytes) {
        invoke<number>('base_models_missing_bytes').then(setMissing).catch(() => {});
      }
    });
    invoke<number>('base_models_missing_bytes')
      .then((bytes) => {
        setMissing(bytes);
        if (bytes > 0) download();
      })
      .catch(() => setMissing(0)); // never lock the app out on an unexpected failure
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [download]);

  if (missing === null) return null;
  if (missing === 0) return <>{children}</>;

  const total = progress.totalBytes || missing;
  const percent = total ? (progress.downloadedBytes / total) * 100 : 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background px-6">
      <div className="w-full max-w-xl space-y-6">
        <div className="space-y-1">
          <h1 className="text-xl font-semibold">{t('modelsTitle')}</h1>
          <p className="text-sm text-muted-foreground">{t('modelsDescription', { size: gb(missing) })}</p>
        </div>

        <div className="space-y-1.5">
          <Progress value={percent} aria-label={t('modelsTitle')} className="h-1.5 rounded-sm bg-secondary" />
          <div className="flex items-center justify-between text-xs tabular-nums text-muted-foreground">
            <span>
              {gb(progress.downloadedBytes)} / {gb(total)}
            </span>
            <span className="font-medium text-foreground">{Math.floor(percent)}%</span>
          </div>
        </div>

        {error ? (
          <div className="space-y-2">
            <p className="text-sm text-destructive">
              {t('downloadFailed')} {t('downloadFailedRemedy')}
            </p>
            <p className="wrap-break-word text-xs text-destructive">{error}</p>
            <Button variant="outline" size="sm" onClick={download}>
              <RefreshCw className="h-4 w-4" />
              {t('tryAgain')}
            </Button>
          </div>
        ) : (
          <p className="flex items-center gap-2 text-xs text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
            {t('modelsResume')}
          </p>
        )}
      </div>
    </div>
  );
}
