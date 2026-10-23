import React, { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Mic, Volume2, Accessibility } from 'lucide-react';
import { Input } from '@/components/ui/input';
import type { PermissionStatus } from '@/types/onboarding';
import type { AutopilotSettingsValue } from '@/components/AutopilotSettings';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { PermissionRow } from '../shared';
import { useOnboarding } from '@/contexts/OnboardingContext';

export function PermissionsStep() {
  const { setPermissionStatus, setPermissionsSkipped, permissions, completeOnboarding } = useOnboarding();
  const [isPending, setIsPending] = useState(false);
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
    if (permissions.microphone === 'denied') {
      // Try to open system settings
      try {
        await invoke('open_system_settings');
      } catch {
        alert('Please enable microphone access in System Preferences > Security & Privacy > Microphone');
      }
      return;
    }

    setIsPending(true);
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
      setIsPending(false);
    }
  };

  // Request system audio permission
  const handleSystemAudioAction = async () => {
    if (permissions.systemAudio === 'denied') {
      // Try to open system settings
      try {
        await invoke('open_system_settings');
      } catch {
        alert('Please enable Audio Capture in System Settings → Privacy & Security → Audio Capture');
      }
      return;
    }

    setIsPending(true);
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
      setIsPending(false);
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
      title="Grant Permissions"
      description="Trascrivi needs access to your microphone and system audio to record meetings"
      step={4}
      hideProgress={true}
      showNavigation={allPermissionsGranted}
      canGoNext={allPermissionsGranted}
    >
      <div className="max-w-lg mx-auto space-y-6">
        {/* Permission Rows */}
        <div className="space-y-4">
          {/* Microphone */}
          <PermissionRow
            icon={<Mic className="w-5 h-5" />}
            title="Microphone"
            description="Required to capture your voice during meetings"
            status={permissions.microphone}
            isPending={isPending}
            onAction={handleMicrophoneAction}
          />

          {/* System Audio */}
          <PermissionRow
            icon={<Volume2 className="w-5 h-5" />}
            title="System Audio"
            description="Click Enable to grant Audio Capture permission"
            status={permissions.systemAudio}
            isPending={isPending}
            onAction={handleSystemAudioAction}
          />

          {/* Accessibility (Trascrivi) */}
          <PermissionRow
            icon={<Accessibility className="w-5 h-5" />}
            title="Accessibilità"
            description="Per riconoscere le call e leggere i nomi da Teams"
            status={accessibility}
            isPending={false}
            onAction={handleAccessibilityAction}
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="onboarding-display-name" className="text-sm font-medium block">
            Il tuo nome (come appare in Teams, es. Rossi Mario)
          </label>
          <Input
            id="onboarding-display-name"
            value={displayName}
            placeholder="Cognome Nome"
            onChange={(e) => setDisplayName(e.target.value)}
            onBlur={saveDisplayName}
          />
          <p className="text-xs text-muted-foreground">
            In Teams imposta la lingua parlata su italiano, così sottotitoli e trascrizione riportano bene i nomi.
          </p>
        </div>

        {/* Action Buttons */}
        <div className="flex flex-col gap-3 pt-4">
          <Button onClick={handleFinish} disabled={!allPermissionsGranted} className="w-full h-11">
            Finish Setup
          </Button>

          <button
            onClick={handleSkip}
            className="text-sm text-neutral-500 hover:text-neutral-700 transition-colors"
          >
            I'll do this later
          </button>

          {!allPermissionsGranted && (
            <p className="text-xs text-center text-muted-foreground">
              Recording won't work without permissions. You can grant them later in settings.
            </p>
          )}
        </div>
      </div>
    </OnboardingContainer>
  );
}
