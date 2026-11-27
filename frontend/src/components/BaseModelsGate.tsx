'use client';

import React, { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { exit } from '@tauri-apps/plugin-process';
import { Download, Loader2, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { locale } from '@/i18n';
import { t } from '@/i18n/onboarding';

const gb = (bytes: number) =>
  `${(bytes / 1e9).toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} GB`;

const mbps = (bytesPerSec: number) =>
  `${(bytesPerSec / 1e6).toLocaleString(locale, { maximumFractionDigits: bytesPerSec < 1e7 ? 1 : 0 })} MB/s`;

function remaining(seconds: number): string {
  if (seconds < 60) return t('modelsLessThanMinute');
  const min = Math.round(seconds / 60);
  if (min < 60) return t('modelsMinutes', { n: String(min) });
  return t('modelsHours', { h: String(Math.floor(min / 60)), m: String(min % 60) });
}

/** Dixit: the models are not in the .app. Until they are on disk, ask to download them, then show progress instead of the app. */
export function BaseModelsGate({ children }: { children: React.ReactNode }) {
  const [missing, setMissing] = useState<number | null>(null);
  const [started, setStarted] = useState(false);
  const [progress, setProgress] = useState({ downloadedBytes: 0, totalBytes: 0 });
  const [rate, setRate] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Progress samples of the last ~10 s, for speed and time left.
  // ponytail: sliding window over events every 250 ms; enough for a steady download.
  const samples = useRef<{ at: number; bytes: number }[]>([]);

  const download = useCallback(async () => {
    setStarted(true);
    setError(null);
    samples.current = [];
    setRate(null);
    try {
      await invoke('base_models_download');
      setMissing(await invoke<number>('base_models_missing_bytes'));
    } catch (e) {
      if (!String(e).includes('already in progress')) setError(String(e));
    }
  }, []);

  useEffect(() => {
    const unlisten = listen<typeof progress>('base-models-progress', (e) => {
      // Also covers a reload mid-download, when this page did not start it.
      setStarted(true);
      setProgress(e.payload);
      const now = Date.now();
      const recent = [...samples.current, { at: now, bytes: e.payload.downloadedBytes }].filter((s) => now - s.at <= 10_000);
      samples.current = recent;
      const first = recent[0];
      if (now - first.at >= 2_000) setRate(Math.max(0, (e.payload.downloadedBytes - first.bytes) / ((now - first.at) / 1000)));
      if (e.payload.downloadedBytes >= e.payload.totalBytes) {
        invoke<number>('base_models_missing_bytes').then(setMissing).catch(() => {});
      }
    });
    invoke<number>('base_models_missing_bytes')
      .then(setMissing)
      .catch(() => setMissing(0)); // never lock the app out on an unexpected failure
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  if (missing === null) return null;
  if (missing === 0) return <>{children}</>;

  const total = progress.totalBytes || missing;
  const percent = total ? (progress.downloadedBytes / total) * 100 : 0;
  const quit = (
    <Button variant="ghost" size="sm" onClick={() => void exit(0)}>
      {t('modelsQuit')}
    </Button>
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background px-6">
      <div className="w-full max-w-xl space-y-6">
        <div className="space-y-1">
          <h1 className="text-xl font-semibold">{t('modelsTitle')}</h1>
          <p className="text-sm text-muted-foreground">{t('modelsDescription', { size: gb(missing) })}</p>
        </div>

        {!started ? (
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Button onClick={download}>
                <Download className="h-4 w-4" />
                {t('modelsStart', { size: gb(missing) })}
              </Button>
              {quit}
            </div>
            <p className="text-xs text-muted-foreground">{t('modelsResume')}</p>
          </div>
        ) : (
          <>
            <div className="space-y-1.5">
              <Progress value={percent} aria-label={t('modelsTitle')} className="h-1.5 rounded-sm bg-secondary" />
              <div className="flex items-center justify-between text-xs tabular-nums text-muted-foreground">
                <span>
                  {gb(progress.downloadedBytes)} / {gb(total)}
                  {rate !== null && rate > 0 && !error && (
                    <> · {t('modelsRate', { speed: mbps(rate), time: remaining((total - progress.downloadedBytes) / rate) })}</>
                  )}
                </span>
                <span className="font-medium text-foreground">{Math.floor(percent)}%</span>
              </div>
            </div>

            {error ? (
              <div className="space-y-2" role="alert">
                <p className="text-sm text-destructive">
                  {t('downloadFailed')} {t('downloadFailedRemedy')}
                </p>
                <p className="wrap-break-word text-xs text-muted-foreground">{error}</p>
                <div className="flex items-center gap-2">
                  <Button variant="outline" size="sm" onClick={download}>
                    <RefreshCw className="h-4 w-4" />
                    {t('tryAgain')}
                  </Button>
                  {quit}
                </div>
              </div>
            ) : (
              <div className="flex items-center justify-between gap-4">
                <p className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
                  {t('modelsResume')}
                </p>
                {quit}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
