import React from 'react';
import { AlertTriangle, Mic, Speaker, RefreshCw } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { invoke } from '@tauri-apps/api/core';
import { useIsLinux } from '@/hooks/usePlatform';
import { Button } from '@/components/ui/button';
import { t } from '@/i18n/shell';

interface PermissionWarningProps {
  hasMicrophone: boolean;
  hasSystemAudio: boolean;
  onRecheck: () => void;
  isRechecking?: boolean;
}

export function PermissionWarning({
  hasMicrophone,
  hasSystemAudio,
  onRecheck,
  isRechecking = false
}: PermissionWarningProps) {
  const isLinux = useIsLinux();

  // Don't show on Linux - permission handling is not needed
  if (isLinux) {
    return null;
  }

  // Don't show if both permissions are granted
  if (hasMicrophone && hasSystemAudio) {
    return null;
  }

  const isMacOS = navigator.userAgent.includes('Mac');

  const openMicrophoneSettings = async () => {
    if (isMacOS) {
      try {
        await invoke('open_system_settings', { preferencePane: 'Privacy_Microphone' });
      } catch (error) {
        console.error('Failed to open microphone settings:', error);
      }
    }
  };

  const openScreenRecordingSettings = async () => {
    if (isMacOS) {
      try {
        await invoke('open_system_settings', { preferencePane: 'Privacy_ScreenCapture' });
      } catch (error) {
        console.error('Failed to open screen recording settings:', error);
      }
    }
  };

  return (
    <div className="mb-4 max-w-md">
      {/* Combined Permission Warning - Show when either permission is missing */}
      {(!hasMicrophone || !hasSystemAudio) && (
        <Alert>
          <AlertTriangle className="h-4 w-4 !text-destructive" />
          <AlertTitle className="flex items-center gap-2 font-semibold">
            {!hasMicrophone && !hasSystemAudio
              ? t('permissionsRequired')
              : !hasMicrophone
                ? t('micPermissionRequired')
                : t('systemAudioPermissionRequired')}
          </AlertTitle>
          <AlertDescription className="mt-2 space-y-3 text-muted-foreground">
            {/* Microphone Warning */}
            {!hasMicrophone && (
              <div className="space-y-1">
                <p>{t('micMissing')}</p>
                <p className="font-medium text-foreground">{t('pleaseCheck')}</p>
                <ul className="ml-4 list-disc space-y-0.5">
                  {t('micChecks').split('\n').map((item) => <li key={item}>{item}</li>)}
                </ul>
              </div>
            )}

            {/* System Audio Warning */}
            {!hasSystemAudio && (
              <div className="space-y-1">
                <p>{hasMicrophone ? t('systemAudioMissing') : t('systemAudioMissingToo')}</p>
                {isMacOS && (
                  <>
                    <p className="font-medium text-foreground">{t('enableSystemAudio')}</p>
                    <ul className="ml-4 list-disc space-y-0.5">
                      {t('systemAudioSteps').split('\n').map((item) => <li key={item}>{item}</li>)}
                    </ul>
                  </>
                )}
              </div>
            )}
          </AlertDescription>
          {/* Action Buttons */}
          <div className="mt-3 flex flex-wrap gap-2">
            {isMacOS && !hasMicrophone && (
              <Button variant="outline" size="sm" onClick={openMicrophoneSettings}>
                <Mic />
                {t('openMicSettings')}
              </Button>
            )}
            {isMacOS && !hasSystemAudio && (
              <Button variant="outline" size="sm" onClick={openScreenRecordingSettings}>
                <Speaker />
                {t('openScreenSettings')}
              </Button>
            )}
            <Button variant="ghost" size="sm" onClick={onRecheck} disabled={isRechecking}>
              <RefreshCw className={isRechecking ? 'animate-spin' : ''} />
              {t('recheck')}
            </Button>
          </div>
        </Alert>
      )}
    </div>
  );
}
