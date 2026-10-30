import React, { useState, useEffect, useCallback } from 'react';
import { Switch } from '@/components/ui/switch';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Copy, Loader2 } from 'lucide-react';
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
  mcp_enabled: boolean;
}

const MCP_URL = 'http://127.0.0.1:7337/mcp';
const CLAUDE_CODE_CMD = `claude mcp add --transport http --scope user dixit ${MCP_URL}`;
const CLAUDE_DESKTOP_JSON = JSON.stringify(
  { mcpServers: { dixit: { command: 'npx', args: ['-y', 'mcp-remote', MCP_URL] } } },
  null,
  2,
);

/** A snippet to paste elsewhere, with a copy button. */
function CopyBlock({ text }: { text: string }) {
  const copy = async () => {
    await navigator.clipboard.writeText(text);
    toast.success(t('copied'));
  };
  return (
    <div className="flex items-start gap-2">
      <pre className="min-w-0 flex-1 overflow-x-auto rounded-md bg-muted p-2.5 font-mono text-xs">{text}</pre>
      <Button variant="outline" size="sm" onClick={copy} aria-label={t('copy')}>
        <Copy />
      </Button>
    </div>
  );
}

/** Trascrivi: automatic start/stop on calls, Teams integration, speaker name. */
export function AutopilotSettings() {
  const [settings, setSettings] = useState<AutopilotSettingsValue | null>(null);
  const [dumping, setDumping] = useState(false);
  // true = listening, false = off, string = why it could not start
  const [mcp, setMcp] = useState<boolean | string>(false);

  const refreshMcp = useCallback(async () => {
    setMcp(await invoke<boolean>('mcp_status').catch((e) => String(e)));
  }, []);

  useEffect(() => {
    invoke<AutopilotSettingsValue>('autopilot_get_settings').then(setSettings).catch(console.error);
    refreshMcp();
  }, [refreshMcp]);

  const save = async (patch: Partial<AutopilotSettingsValue>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    try {
      await invoke('autopilot_save_settings', { settings: next });
    } catch (error) {
      toast.error(t('settingsNotSaved'), { description: String(error) });
    }
    refreshMcp();
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
        <SettingRow label={t('diagnostics')} description={t('diagnosticsDesc')}>
          <Button variant="outline" size="sm" onClick={dumpTeams} disabled={dumping}>
            {dumping && <Loader2 className="animate-spin" />}
            {dumping ? t('savingDiagnostics') : t('saveDiagnostics')}
          </Button>
        </SettingRow>
      </SettingsSection>

      <SettingsSection title={t('agentsSection')} description={t('agentsDesc')}>
        <SettingRow
          label={t('mcpEnabled')}
          description={
            typeof mcp === 'string' ? (
              <span role="alert" className="text-destructive">{t('mcpError', { error: mcp })}</span>
            ) : mcp ? (
              t('mcpListening', { url: MCP_URL })
            ) : (
              t('mcpEnabledDesc')
            )
          }
          htmlFor="mcp-enabled"
        >
          <Switch id="mcp-enabled" checked={settings.mcp_enabled} onCheckedChange={(v) => save({ mcp_enabled: v })} />
        </SettingRow>
        {settings.mcp_enabled && (
          <div className="space-y-4 py-3 text-sm">
            <div className="space-y-1.5">
              <p className="font-medium">{t('mcpClaudeCode')}</p>
              <p className="text-xs text-muted-foreground">{t('mcpClaudeCodeDesc')}</p>
              <CopyBlock text={CLAUDE_CODE_CMD} />
            </div>
            <div className="space-y-1.5">
              <p className="font-medium">{t('mcpClaudeDesktop')}</p>
              <p className="text-xs text-muted-foreground">{t('mcpClaudeDesktopDesc')}</p>
              <CopyBlock text={CLAUDE_DESKTOP_JSON} />
            </div>
            <div className="space-y-1">
              <p className="font-medium">{t('mcpUsage')}</p>
              <p className="text-xs text-muted-foreground">{t('mcpUsageDesc')}</p>
            </div>
          </div>
        )}
      </SettingsSection>
    </div>
  );
}
