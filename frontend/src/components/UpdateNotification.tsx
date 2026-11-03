import React from 'react';
import { Download } from 'lucide-react';
import { toast } from 'sonner';
import { UpdateInfo } from '@/services/updateService';
import { t } from '@/i18n/shell';

let globalShowDialogCallback: (() => void) | null = null;

export function setUpdateDialogCallback(callback: () => void) {
  globalShowDialogCallback = callback;
}

export function showUpdateNotification(updateInfo: UpdateInfo, onUpdateClick?: () => void) {
  const handleClick = () => {
    if (onUpdateClick) {
      onUpdateClick();
    } else if (globalShowDialogCallback) {
      globalShowDialogCallback();
    }
  };

  toast.info(
    <div className="flex items-center justify-between gap-4">
      <div className="flex items-center gap-2">
        <Download className="h-4 w-4 shrink-0 text-primary" />
        <div>
          <p className="text-sm font-medium">{t('updateAvailable')}</p>
          <p className="text-xs tabular-nums text-muted-foreground">
            {t('versionAvailable', { version: updateInfo.version ?? '' })}
          </p>
        </div>
      </div>
      <button
        onClick={(e) => {
          e.stopPropagation();
          handleClick();
        }}
        className="shrink-0 rounded-sm text-sm font-medium text-primary underline-offset-4 transition-colors duration-150 hover:underline focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
      >
        {t('viewDetails')}
      </button>
    </div>,
    {
      duration: 10000,
      position: 'bottom-center',
    }
  );
}
