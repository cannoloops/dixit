'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AgentsDemo } from '@/components/AgentsDemo';
import type { AutopilotSettingsValue } from '@/components/AutopilotSettings';
import { t } from '@/i18n/shell';

// ponytail: per-Mac flag in localStorage; if it gets wiped the announcement shows once more.
const SEEN_KEY = 'dixit-agents-announced';

const SHOW_EVENT = 'dixit-show-agents-announcement';
/** Fired after the announcement turns MCP on, so an open Settings page reloads its values. */
export const MCP_ENABLED_EVENT = 'dixit-mcp-enabled';

const markSeen = () => {
  try { localStorage.setItem(SEEN_KEY, '1'); } catch {}
};

/** Opens the announcement again on demand (from Settings). */
export const showAgentsAnnouncement = () => window.dispatchEvent(new Event(SHOW_EVENT));

/** Once, at startup: tells the user Dixit can feed their agents (MCP), unless it is already on. */
export function AgentsAnnouncement() {
  const router = useRouter();
  const [settings, setSettings] = useState<AutopilotSettingsValue | null>(null);

  useEffect(() => {
    const reopen = () => invoke<AutopilotSettingsValue>('autopilot_get_settings').then(setSettings).catch(console.error);
    window.addEventListener(SHOW_EVENT, reopen);
    return () => window.removeEventListener(SHOW_EVENT, reopen);
  }, []);

  useEffect(() => {
    try { if (localStorage.getItem(SEEN_KEY)) return; } catch { return; }
    invoke<AutopilotSettingsValue>('autopilot_get_settings')
      .then((s) => (s.mcp_enabled ? markSeen() : setSettings(s)))
      .catch(console.error);
  }, []);

  const close = () => {
    markSeen();
    setSettings(null);
  };

  const enable = async () => {
    if (!settings) return;
    try {
      await invoke('autopilot_save_settings', { settings: { ...settings, mcp_enabled: true } });
      window.dispatchEvent(new Event(MCP_ENABLED_EVENT));
    } catch (error) {
      toast.error(String(error));
    }
    close();
    router.push('/settings?tab=autopilot');
  };

  return (
    <Dialog open={settings !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent className="gap-5 sm:max-w-xl">
        <div className="-mx-6 -mt-6 rounded-t-md bg-muted px-6 pb-5 pt-10">
          <AgentsDemo />
        </div>
        <DialogHeader>
          <DialogTitle className="text-lg">{t('agentsTitle')}</DialogTitle>
          <DialogDescription>{t('agentsBody')}</DialogDescription>
        </DialogHeader>
        <p className="text-xs text-muted-foreground">{t('agentsLater')}</p>
        <DialogFooter>
          <Button variant="outline" onClick={close}>{t('agentsNotNow')}</Button>
          <Button onClick={enable}>{t('agentsEnable')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
