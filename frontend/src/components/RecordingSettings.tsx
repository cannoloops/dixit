import React, { useState, useEffect } from 'react';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { FolderOpen } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { DeviceSelection, SelectedDevices } from '@/components/DeviceSelection';
import Analytics from '@/lib/analytics';
import { toast } from 'sonner';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useConfig } from '@/contexts/ConfigContext';
import { SettingsSection, SettingRow } from '@/components/molecules/form-components/setting-row';
import { t } from '@/i18n/settings';

export interface RecordingPreferences {
  save_folder: string;
  auto_save: boolean;
  file_format: string;
  preferred_mic_device: string | null;
  preferred_system_device: string | null;
}

interface RecordingSettingsProps {
  onSave?: (preferences: RecordingPreferences) => void;
}

export function RecordingSettings({ onSave }: RecordingSettingsProps) {
  const [preferences, setPreferences] = useState<RecordingPreferences>({
    save_folder: '',
    auto_save: true,
    file_format: 'mp4',
    preferred_mic_device: null,
    preferred_system_device: null
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [showRecordingNotification, setShowRecordingNotification] = useState(true);
  const { isRecording } = useRecordingState();
  const { setSelectedDevices } = useConfig();

  // Load recording preferences on component mount
  useEffect(() => {
    const loadPreferences = async () => {
      try {
        const prefs = await invoke<RecordingPreferences>('get_recording_preferences');
        setPreferences(prefs);
      } catch (error) {
        console.error('Failed to load recording preferences:', error);
        // If loading fails, get default folder path
        try {
          const defaultPath = await invoke<string>('get_default_recordings_folder_path');
          setPreferences(prev => ({ ...prev, save_folder: defaultPath }));
        } catch (defaultError) {
          console.error('Failed to get default folder path:', defaultError);
        }
      } finally {
        setLoading(false);
      }
    };

    loadPreferences();
  }, []);

  // Load recording notification preference
  useEffect(() => {
    const loadNotificationPref = async () => {
      try {
        const { Store } = await import('@tauri-apps/plugin-store');
        const store = await Store.load('preferences.json');
        const show = await store.get<boolean>('show_recording_notification') ?? true;
        setShowRecordingNotification(show);
      } catch (error) {
        console.error('Failed to load notification preference:', error);
      }
    };
    loadNotificationPref();
  }, []);

  const handleAutoSaveToggle = async (enabled: boolean) => {
    const newPreferences = { ...preferences, auto_save: enabled };
    setPreferences(newPreferences);
    await savePreferences(newPreferences);

    // Track auto-save setting change
    await Analytics.track('auto_save_recording_toggled', {
      enabled: enabled.toString()
    });
  };

  const handleDeviceChange = async (devices: SelectedDevices) => {
    const newPreferences = {
      ...preferences,
      preferred_mic_device: devices.micDevice,
      preferred_system_device: devices.systemDevice
    };
    setPreferences(newPreferences);
    // Sync the in-memory selection ConfigContext exposes to the start path.
    // Without this, the picker only writes to disk (loaded into context once at
    // app mount), so a newly-chosen mic isn't honored until the next launch —
    // start keeps sending the stale launch-time device.
    setSelectedDevices(devices);
    await savePreferences(newPreferences);

    // Track default device preference changes
    // Note: Individual device selection analytics are tracked in DeviceSelection component
    await Analytics.track('default_devices_changed', {
      has_preferred_microphone: (!!devices.micDevice).toString(),
      has_preferred_system_audio: (!!devices.systemDevice).toString()
    });
  };

  const handleOpenFolder = async () => {
    try {
      await invoke('open_recordings_folder');
    } catch (error) {
      console.error('Failed to open recordings folder:', error);
    }
  };

  const handleNotificationToggle = async (enabled: boolean) => {
    try {
      setShowRecordingNotification(enabled);
      const { Store } = await import('@tauri-apps/plugin-store');
      const store = await Store.load('preferences.json');
      await store.set('show_recording_notification', enabled);
      await store.save();
      toast.success(t('prefSaved'));
      await Analytics.track('recording_notification_preference_changed', {
        enabled: enabled.toString()
      });
    } catch (error) {
      console.error('Failed to save notification preference:', error);
      toast.error(t('prefNotSaved'));
    }
  };

  const savePreferences = async (prefs: RecordingPreferences) => {
    setSaving(true);
    try {
      await invoke('set_recording_preferences', { preferences: prefs });
      onSave?.(prefs);

      // Show success toast with device details
      const micDevice = prefs.preferred_mic_device || t('defaultValue');
      const systemDevice = prefs.preferred_system_device || t('defaultValue');
      toast.success(t('devicesSaved'), {
        description: t('devicesSavedDesc', { mic: micDevice, system: systemDevice })
      });
    } catch (error) {
      console.error('Failed to save recording preferences:', error);
      toast.error(t('devicesNotSaved'), {
        description: error instanceof Error ? error.message : String(error)
      });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="animate-pulse">
        <div className="mb-4 h-4 w-1/4 rounded-sm bg-muted"></div>
        <div className="mb-4 h-8 rounded-sm bg-muted"></div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <SettingsSection title={t('recSaveSection')} description={t('recSaveDesc')}>
        <SettingRow label={t('saveAudio')} description={t('saveAudioDesc')} htmlFor="auto-save-audio">
          <Switch
            id="auto-save-audio"
            checked={preferences.auto_save}
            onCheckedChange={handleAutoSaveToggle}
            disabled={saving}
          />
        </SettingRow>

        {/* Folder and format: only shown when auto_save is enabled */}
        {preferences.auto_save && (
          <>
            <SettingRow
              label={t('saveLocation')}
              description={<span className="break-all">{preferences.save_folder || t('defaultFolder')}</span>}
            >
              <Button variant="outline" size="sm" onClick={handleOpenFolder}>
                <FolderOpen />
                {t('openFolder')}
              </Button>
            </SettingRow>
            <SettingRow
              label={t('fileFormat')}
              description={t('fileNameHint', { ext: preferences.file_format })}
            >
              <span className="text-sm text-muted-foreground">{preferences.file_format.toUpperCase()}</span>
            </SettingRow>
          </>
        )}

        {!preferences.auto_save && (
          <p className="py-2.5 text-sm text-muted-foreground">{t('audioDisabled')}</p>
        )}
      </SettingsSection>

      <SettingsSection title={t('noticesSection')}>
        <SettingRow label={t('startNotice')} description={t('startNoticeDesc')} htmlFor="recording-notification">
          <Switch
            id="recording-notification"
            checked={showRecordingNotification}
            onCheckedChange={handleNotificationToggle}
          />
        </SettingRow>
      </SettingsSection>

      <SettingsSection title={t('devicesSection')} description={t('devicesDesc')}>
        {isRecording && (
          <p role="status" aria-live="polite" className="py-2.5 text-sm text-muted-foreground">
            {t('devicesLocked')}
          </p>
        )}
        <DeviceSelection
          selectedDevices={{
            micDevice: preferences.preferred_mic_device,
            systemDevice: preferences.preferred_system_device
          }}
          onDeviceChange={handleDeviceChange}
          disabled={saving || isRecording}
        />
      </SettingsSection>
    </div>
  );
}
