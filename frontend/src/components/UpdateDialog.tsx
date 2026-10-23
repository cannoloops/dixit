import React, { useState, useEffect } from 'react';
import { Download, AlertCircle, Loader2 } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Button } from './ui/button';
import { Progress } from './ui/progress';
import { updateService, UpdateInfo, UpdateProgress } from '@/services/updateService';
import { check, Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { toast } from 'sonner';
import { locale } from '@/i18n';
import { t } from '@/i18n/shell';
import { t as tApp } from '@/i18n/app';

interface UpdateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  updateInfo: UpdateInfo | null;
}

export function UpdateDialog({ open, onOpenChange, updateInfo }: UpdateDialogProps) {
  const [isDownloading, setIsDownloading] = useState(false);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [update, setUpdate] = useState<Update | null>(null);

  useEffect(() => {
    if (open && updateInfo?.available) {
      // Reset state when dialog opens
      setIsDownloading(false);
      setProgress(null);
      setError(null);

      // Get the update object when dialog opens
      check().then((updateResult) => {
        if (updateResult?.available) {
          setUpdate(updateResult);
        } else {
          setError(t('updateGone'));
        }
      }).catch((err) => {
        console.error('Failed to get update object:', err);
        setError(t('updatePrepareFailed', { error: err?.message || t('unknownError') }));
      });
    } else {
      // Reset state when dialog closes
      setIsDownloading(false);
      setProgress(null);
      setError(null);
      setUpdate(null);
    }
  }, [open, updateInfo]);

  const handleDownloadAndInstall = async () => {
    // Get update object if not already available
    let updateToUse: Update | null = update;
    if (!updateToUse) {
      try {
        const updateResult = await check();
        if (updateResult?.available) {
          updateToUse = updateResult;
          setUpdate(updateResult);
        } else {
          setError(t('updateGone'));
          return;
        }
      } catch (err: any) {
        setError(t('updatePrepareFailed', { error: err?.message || t('unknownError') }));
        return;
      }
    }

    // At this point, updateToUse is guaranteed to be non-null
    if (!updateToUse) {
      return; // This should never happen, but TypeScript needs this check
    }

    setIsDownloading(true);
    setError(null);
    setProgress({ downloaded: 0, total: 0, percentage: 0 });

    try {
      let downloaded = 0;
      let contentLength = 0;

      // Use the official Tauri updater API with progress callbacks
      await updateToUse.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength || 0;
            console.log(`[UpdateDialog] Started downloading ${contentLength} bytes`);
            setProgress({
              downloaded: 0,
              total: contentLength,
              percentage: 0,
            });
            break;

          case 'Progress':
            downloaded += event.data.chunkLength || 0;
            const percentage = contentLength > 0
              ? Math.round((downloaded / contentLength) * 100)
              : 0;
            console.log(`[UpdateDialog] Progress: ${downloaded} / ${contentLength} bytes (${percentage}%)`);
            setProgress({
              downloaded,
              total: contentLength,
              percentage,
            });
            break;

          case 'Finished':
            console.log('[UpdateDialog] Download finished');
            setProgress({
              downloaded: contentLength,
              total: contentLength,
              percentage: 100,
            });
            break;
        }
      });

      console.log('[UpdateDialog] Update installed successfully');
      toast.success(t('updateInstalled'));

      // Mark download as complete before closing
      setIsDownloading(false);

      // Close dialog before relaunch
      handleOpenChange(false);

      // Relaunch the app
      await relaunch();
    } catch (err: any) {
      console.error('Update failed:', err);
      setError(err?.message || t('updateInstallFailed'));
      setIsDownloading(false);
      toast.error(t('updateFailedToast', { error: err?.message || t('unknownError') }));
    }
  };

  const formatDate = (dateString?: string) => {
    if (!dateString) return '';
    try {
      return new Date(dateString).toLocaleDateString(locale);
    } catch {
      return dateString;
    }
  };

  // Prevent closing the dialog when downloading
  const handleOpenChange = (newOpen: boolean) => {
    // If trying to close while downloading, prevent it
    if (!newOpen && isDownloading) {
      return;
    }
    // Otherwise, allow normal close behavior
    onOpenChange(newOpen);
  };

  // Prevent ESC key from closing dialog during download
  const handleEscapeKeyDown = (event: KeyboardEvent) => {
    if (isDownloading) {
      event.preventDefault();
    }
  };

  // Prevent outside clicks from closing dialog during download
  const handleInteractOutside = (event: Event) => {
    if (isDownloading) {
      event.preventDefault();
    }
  };

  if (!updateInfo?.available) {
    return null;
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        className="sm:max-w-[500px]"
        onEscapeKeyDown={handleEscapeKeyDown}
        onInteractOutside={handleInteractOutside}
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {isDownloading ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                {t('downloadingUpdate')}
              </>
            ) : error ? (
              <>
                <AlertCircle className="h-4 w-4 text-destructive" />
                {t('updateFailed')}
              </>
            ) : (
              <>
                <Download className="h-4 w-4 text-primary" />
                {t('updateAvailable')}
              </>
            )}
          </DialogTitle>
          <DialogDescription>
            {isDownloading
              ? t('downloadingDesc')
              : error
              ? t('updateFailedDesc')
              : t('versionAvailable', { version: updateInfo.version ?? '' })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {!isDownloading && !error && (
            <>
              <dl className="space-y-1 text-sm">
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">{t('currentVersion')}</dt>
                  <dd className="font-medium tabular-nums">{updateInfo.currentVersion}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">{t('newVersion')}</dt>
                  <dd className="font-medium tabular-nums text-primary">{updateInfo.version}</dd>
                </div>
                {updateInfo.date && (
                  <div className="flex justify-between">
                    <dt className="text-muted-foreground">{t('releaseDate')}</dt>
                    <dd className="font-medium tabular-nums">{formatDate(updateInfo.date)}</dd>
                  </div>
                )}
              </dl>

              {updateInfo.body && (
                <p className="max-h-40 overflow-y-auto whitespace-pre-wrap border-t border-border pt-3 text-sm text-muted-foreground">
                  {updateInfo.body}
                </p>
              )}
            </>
          )}

          {isDownloading && progress && (
            <div className="space-y-2">
              <div>
                <Progress value={Math.min(progress.percentage, 100)} />
                <div className="mt-1 flex justify-between text-xs tabular-nums text-muted-foreground">
                  <span>{t('percentComplete', { percent: Math.round(progress.percentage) })}</span>
                  {progress.total > 0 && (
                    <span>
                      {formatBytes(progress.downloaded)} / {formatBytes(progress.total)}
                    </span>
                  )}
                </div>
              </div>
              <p className="text-xs text-muted-foreground">
                {t('restartNote')}
              </p>
            </div>
          )}

          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}
        </div>

        <DialogFooter>
          {!isDownloading && !error && (
            <>
              <Button variant="outline" onClick={() => handleOpenChange(false)}>
                {t('later')}
              </Button>
              <Button onClick={handleDownloadAndInstall}>
                <Download />
                {t('downloadInstall')}
              </Button>
            </>
          )}
          {error && (
            <Button variant="outline" onClick={() => handleOpenChange(false)}>
              {tApp('close')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return (bytes / Math.pow(k, i)).toLocaleString(locale, { maximumFractionDigits: 2 }) + ' ' + sizes[i];
}
