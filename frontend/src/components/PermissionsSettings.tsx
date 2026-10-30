import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Accessibility, Mic, Volume2 } from 'lucide-react';
import { toast } from 'sonner';
import { SettingsSection } from '@/components/molecules/form-components/setting-row';
import { PermissionRow } from '@/components/onboarding/shared';
import type { PermissionStatus } from '@/types/onboarding';
import { t as tPerm } from '@/i18n/onboarding';
import { t } from '@/i18n/settings';

interface Status {
  microphone: PermissionStatus;
  system_audio: PermissionStatus;
  accessibility: PermissionStatus;
}

const openPane = (pane: string) =>
  invoke('open_system_settings', { preferencePane: pane }).catch(() => toast.error(tPerm('openSettingsFailed')));

/** Settings › Generali: what macOS grants Dixit right now, with a shortcut to fix each permission. */
export function PermissionsSettings() {
  const [status, setStatus] = useState<Status | null>(null);
  const [pending, setPending] = useState<keyof Status | null>(null);

  const refresh = useCallback(() => {
    invoke<Status>('permissions_status').then(setStatus).catch(console.error);
  }, []);

  // ponytail: poll while the section is open (window "focus" doesn't fire in the Tauri webview
  // when coming back from System Settings); three cheap reads, a Tauri focus event if it ever matters.
  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, 2000);
    return () => clearInterval(timer);
  }, [refresh]);

  // Never asked: show the macOS prompt. Denied: only System Settings can change it.
  const fix = (key: keyof Status, command: string, pane: string) => async () => {
    if (status?.[key] === 'denied') return openPane(pane);
    setPending(key);
    try {
      await invoke(command);
    } catch (error) {
      console.error(error);
    } finally {
      setPending(null);
      refresh();
    }
  };

  // Registers Dixit in the Accessibility list (the prompt), then opens it.
  const fixAccessibility = async () => {
    await invoke('autopilot_request_accessibility', { prompt: true }).catch(console.error);
    openPane('Privacy_Accessibility');
  };

  return (
    <SettingsSection title={t('permissionsSection')} description={t('permissionsDesc')}>
      <PermissionRow
        icon={<Mic />}
        title={tPerm('micName')}
        description={tPerm('micPurpose')}
        deniedHint={tPerm('micDenied')}
        status={status?.microphone ?? 'checking'}
        isPending={pending === 'microphone'}
        onAction={fix('microphone', 'trigger_microphone_permission', 'Privacy_Microphone')}
      />
      <PermissionRow
        icon={<Volume2 />}
        title={tPerm('systemName')}
        description={tPerm('systemPurpose')}
        deniedHint={tPerm('systemDenied')}
        status={status?.system_audio ?? 'checking'}
        isPending={pending === 'system_audio'}
        onAction={fix('system_audio', 'trigger_system_audio_permission_command', 'Privacy_ScreenCapture')}
      />
      <PermissionRow
        icon={<Accessibility />}
        title={tPerm('accessibilityName')}
        description={tPerm('accessibilityPurpose')}
        deniedHint={t('accessibilityMissing')}
        status={status?.accessibility ?? 'checking'}
        onAction={fixAccessibility}
      />
    </SettingsSection>
  );
}
