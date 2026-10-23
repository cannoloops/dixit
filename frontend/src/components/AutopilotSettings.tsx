import React, { useState, useEffect, useCallback } from 'react';
import { Switch } from '@/components/ui/switch';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Check, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { SettingsSection, SettingRow } from '@/components/molecules/form-components/setting-row';
import { t } from '@/i18n/settings';

export interface AutopilotSettingsValue {
  auto_start: boolean;
  native_teams_transcription: boolean;
  user_display_name: string;
  start_after_secs: number;
  stop_after_secs: number;
}

/** Trascrivi: automatic start/stop on calls, Teams integration, speaker name. */
export function AutopilotSettings() {
  const [settings, setSettings] = useState<AutopilotSettingsValue | null>(null);
  const [trusted, setTrusted] = useState<boolean | null>(null);
  const [dumping, setDumping] = useState(false);

  const refreshTrust = useCallback(async () => {
    setTrusted(await invoke<boolean>('autopilot_request_accessibility', { prompt: false }));
  }, []);

  useEffect(() => {
    invoke<AutopilotSettingsValue>('autopilot_get_settings').then(setSettings).catch(console.error);
    refreshTrust().catch(console.error);
  }, [refreshTrust]);

  const save = async (patch: Partial<AutopilotSettingsValue>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    try {
      await invoke('autopilot_save_settings', { settings: next });
    } catch (error) {
      toast.error(t('settingsNotSaved'), { description: String(error) });
    }
  };

  const requestAccessibility = async () => {
    await invoke<boolean>('autopilot_request_accessibility', { prompt: true });
    setTimeout(() => refreshTrust().catch(console.error), 3000);
  };

  const dumpTeams = async () => {
    setDumping(true);
    try {
      const path = await invoke<string>('teams_dump_ax_tree');
      toast.success(t('diagnosticsSaved'), { description: path, duration: 10000 });
    } catch (error) {
      toast.error(t('diagnosticsFailed'), { description: String(error) });
    } finally {
      setDumping(false);
    }
  };

  if (!settings) {
    return (
      <div className="animate-pulse">
        <div className="mb-4 h-4 w-1/4 rounded-sm bg-muted"></div>
        <div className="mb-4 h-8 rounded-sm bg-muted"></div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <p className="text-sm text-muted-foreground">{t('autopilotIntro')}</p>

      <SettingsSection title={t('autoSection')}>
        <SettingRow label={t('autoStart')} description={t('autoStartDesc')} htmlFor="auto-start">
          <Switch id="auto-start" checked={settings.auto_start} onCheckedChange={(v) => save({ auto_start: v })} />
        </SettingRow>
        <SettingRow label={t('startAfter')} htmlFor="start-after-secs">
          <Input
            id="start-after-secs"
            type="number"
            min={1}
            className="w-24 tabular-nums"
            value={settings.start_after_secs}
            onChange={(e) => save({ start_after_secs: Math.max(1, Number(e.target.value) || 1) })}
          />
        </SettingRow>
        <SettingRow label={t('stopAfter')} htmlFor="stop-after-secs">
          <Input
            id="stop-after-secs"
            type="number"
            min={10}
            className="w-24 tabular-nums"
            value={settings.stop_after_secs}
            onChange={(e) => save({ stop_after_secs: Math.max(10, Number(e.target.value) || 10) })}
          />
        </SettingRow>
      </SettingsSection>

      <SettingsSection title={t('teamsSection')}>
        <SettingRow label={t('nativeTeams')} description={t('nativeTeamsDesc')} htmlFor="native-teams-transcription">
          <Switch
            id="native-teams-transcription"
            checked={settings.native_teams_transcription}
            onCheckedChange={(v) => save({ native_teams_transcription: v })}
          />
        </SettingRow>
        <SettingRow label={t('yourName')} description={t('yourNameDesc')} htmlFor="user-display-name">
          <Input
            id="user-display-name"
            className="w-56"
            value={settings.user_display_name}
            placeholder={t('yourNamePlaceholder')}
            onChange={(e) => setSettings({ ...settings, user_display_name: e.target.value })}
            onBlur={() => save({})}
          />
        </SettingRow>
        <SettingRow label={t('accessibility')} description={t('accessibilityDesc')}>
          {trusted ? (
            <span className="inline-flex items-center gap-1.5 text-sm text-muted-foreground">
              <Check className="h-4 w-4" />
              {t('granted')}
            </span>
          ) : (
            <Button variant="outline" size="sm" onClick={requestAccessibility}>
              {t('grant')}
            </Button>
          )}
        </SettingRow>
        <SettingRow label={t('diagnostics')} description={t('diagnosticsDesc')}>
          <Button variant="outline" size="sm" onClick={dumpTeams} disabled={dumping}>
            {dumping && <Loader2 className="animate-spin" />}
            {dumping ? t('savingDiagnostics') : t('saveDiagnostics')}
          </Button>
        </SettingRow>
      </SettingsSection>
    </div>
  );
}
