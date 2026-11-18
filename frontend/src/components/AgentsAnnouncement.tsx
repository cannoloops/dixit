'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { Bot } from 'lucide-react';
import { toast } from 'sonner';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import type { AutopilotSettingsValue } from '@/components/AutopilotSettings';
import { t } from '@/i18n/shell';

// ponytail: per-Mac flag in localStorage; if it gets wiped the announcement shows once more.
const SEEN_KEY = 'dixit-agents-announced';

const markSeen = () => {
  try { localStorage.setItem(SEEN_KEY, '1'); } catch {}
};

/** Once, at startup: tells the user Dixit can feed their agents (MCP), unless it is already on. */
export function AgentsAnnouncement() {
  const router = useRouter();
  const [settings, setSettings] = useState<AutopilotSettingsValue | null>(null);

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
    } catch (error) {
      toast.error(String(error));
    }
    close();
    router.push('/settings?tab=autopilot');
  };

  return (
    <Dialog open={settings !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent>
        <DialogHeader>
          <Bot className="h-6 w-6 text-primary" aria-hidden />
          <DialogTitle>{t('agentsTitle')}</DialogTitle>
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
