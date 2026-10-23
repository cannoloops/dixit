import React, { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Mic, Volume2, Accessibility } from 'lucide-react';
import { toast } from 'sonner';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { PermissionStatus } from '@/types/onboarding';
import type { AutopilotSettingsValue } from '@/components/AutopilotSettings';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { PermissionRow } from '../shared';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { t } from '@/i18n/onboarding';

export function PermissionsStep() {
  const { setPermissionStatus, setPermissionsSkipped, permissions, completeOnboarding } = useOnboarding();
  const [pending, setPending] = useState<'microphone' | 'systemAudio' | null>(null);
  // Trascrivi: Accessibility (Teams window reading) and the user's speaker name
  const [accessibility, setAccessibility] = useState<PermissionStatus>('not_determined');
  const [displayName, setDisplayName] = useState('');

  useEffect(() => {
    invoke<boolean>('autopilot_request_accessibility', { prompt: false })
      .then((ok) => setAccessibility(ok ? 'authorized' : 'not_determined'))
      .catch(console.error);
    invoke<AutopilotSettingsValue>('autopilot_get_settings')
      .then((s) => setDisplayName(s.user_display_name))
      .catch(console.error);
  }, []);

  const handleAccessibilityAction = async () => {
    await invoke<boolean>('autopilot_request_accessibility', { prompt: true });
    // macOS grants it in System Settings; re-check shortly after.
    setTimeout(async () => {
      const ok = await invoke<boolean>('autopilot_request_accessibility', { prompt: false });
      setAccessibility(ok ? 'authorized' : 'denied');
    }, 4000);
  };

  const saveDisplayName = async () => {
    try {
      const current = await invoke<AutopilotSettingsValue>('autopilot_get_settings');
      await invoke('autopilot_save_settings', { settings: { ...current, user_display_name: displayName.trim() } });
    } catch (error) {
      console.error('[PermissionsStep] Failed to save display name:', error);
    }
  };

  // Check permissions - only logs current state, doesn't auto-authorize
  // Actual permission checks are done via explicit user actions (clicking Enable)
  const checkPermissions = useCallback(async () => {
    console.log('[PermissionsStep] Current permission states:');
    console.log(`  - Microphone: ${permissions.microphone}`);
    console.log(`  - System Audio: ${permissions.systemAudio}`);
    // Don't auto-set permissions based on device availability
    // Permissions should only be set after explicit user action via Enable button
  }, [permissions.microphone, permissions.systemAudio]);

  // Check permissions on mount
  useEffect(() => {
    checkPermissions();
  }, [checkPermissions]);

  // Request microphone permission
  const handleMicrophoneAction = async () => {
    if (pending) return;
    if (permissions.microphone === 'denied') {
      // Try to open system settings
      try {
        await invoke('open_system_settings', { preferencePane: 'Privacy_Microphone' });
      } catch {
        toast.error(t('openSettingsFailed'), { description: t('micDenied') });
      }
      return;
    }

    setPending('microphone');
    try {
      console.log('[PermissionsStep] Triggering microphone permission...');
      const granted = await invoke<boolean>('trigger_microphone_permission');
      console.log('[PermissionsStep] Microphone permission result:', granted);

      if (granted) {
        setPermissionStatus('microphone', 'authorized');
      } else {
        // Permission was denied or dialog was dismissed
        setPermissionStatus('microphone', 'denied');
      }
    } catch (err) {
      console.error('[PermissionsStep] Failed to request microphone permission:', err);
      setPermissionStatus('microphone', 'denied');
    } finally {
      setPending(null);
    }
  };

  // Request system audio permission
  const handleSystemAudioAction = async () => {
    if (pending) return;
    if (permissions.systemAudio === 'denied') {
      // Try to open system settings
      try {
        await invoke('open_system_settings', { preferencePane: 'Privacy_ScreenCapture' });
      } catch {
        toast.error(t('openSettingsFailed'), { description: t('systemDenied') });
      }
      return;
    }

    setPending('systemAudio');
    try {
      console.log('[PermissionsStep] Triggering Audio Capture permission...');
      // Backend creates Core Audio tap, captures audio, and verifies it's not silence
      // Returns true if permission granted and audio verified, false if denied (silence)
      const granted = await invoke<boolean>('trigger_system_audio_permission_command');
      console.log('[PermissionsStep] System audio permission result:', granted);

      if (granted) {
        setPermissionStatus('systemAudio', 'authorized');
        console.log('[PermissionsStep] Audio Capture permission verified - audio is not silence');
      } else {
        // Permission was denied (audio is silence)
        setPermissionStatus('systemAudio', 'denied');
        console.log('[PermissionsStep] Audio Capture permission denied - audio is silence');
      }
    } catch (err) {
      console.error('[PermissionsStep] Failed to request system audio permission:', err);
      setPermissionStatus('systemAudio', 'denied');
    } finally {
      setPending(null);
    }
  };

  const handleFinish = async () => {
    await saveDisplayName();
    // Speaker model (26 MB) downloads in the background; attribution also fetches it on demand.
    invoke('speakers_ensure_model').catch((e) => console.error('Speaker model download failed:', e));
    try {
      await completeOnboarding();
      window.location.reload();
    } catch (error) {
      console.error('Failed to complete onboarding:', error);
    }
  };

  const handleSkip = async () => {
    setPermissionsSkipped(true);
    await handleFinish();
  };

  const allPermissionsGranted =
    permissions.microphone === 'authorized' &&
    permissions.systemAudio === 'authorized';

  return (
    <OnboardingContainer
      title={t('permissionsTitle')}
      description={t('permissionsDescription')}
      step={4}
      hideProgress={true}
      showNavigation={allPermissionsGranted}
      canGoNext={allPermissionsGranted}
    >
      <div className="divide-y divide-border border-y border-border">
        <PermissionRow
          icon={<Mic />}
          title={t('micName')}
          description={t('micPurpose')}
          deniedHint={t('micDenied')}
          status={permissions.microphone}
          isPending={pending === 'microphone'}
          onAction={handleMicrophoneAction}
        />

        <PermissionRow
          icon={<Volume2 />}
          title={t('systemName')}
          description={t('systemPurpose')}
          deniedHint={t('systemDenied')}
          status={permissions.systemAudio}
          isPending={pending === 'systemAudio'}
          onAction={handleSystemAudioAction}
        />

        {/* Accessibility (Trascrivi) */}
        <PermissionRow
          icon={<Accessibility />}
          title={t('accessibilityName')}
          description={t('accessibilityPurpose')}
          deniedHint={t('accessibilityDenied')}
          status={accessibility}
          onAction={handleAccessibilityAction}
        />
      </div>

      <div className="mt-6 space-y-1.5">
        <Label htmlFor="onboarding-display-name">{t('displayNameLabel')}</Label>
        <Input
          id="onboarding-display-name"
          value={displayName}
          placeholder={t('displayNamePlaceholder')}
          onChange={(e) => setDisplayName(e.target.value)}
          onBlur={saveDisplayName}
        />
        <p className="text-xs text-muted-foreground">{t('displayNameHint')}</p>
      </div>

      <div className="mt-8 space-y-3">
        {!allPermissionsGranted && <p className="text-xs text-muted-foreground">{t('permissionsNote')}</p>}
        <div className="flex items-center justify-between">
          <Button variant="ghost" onClick={handleSkip}>
            {t('doLater')}
          </Button>
          <Button size="lg" onClick={handleFinish} disabled={!allPermissionsGranted}>
            {t('finishSetup')}
          </Button>
        </div>
      </div>
    </OnboardingContainer>
  );
}
