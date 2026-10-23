import React, { useState, useEffect, useCallback } from 'react';
import { Switch } from '@/components/ui/switch';
import { Input } from '@/components/ui/input';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

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
      toast.error('Impostazioni non salvate', { description: String(error) });
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
      toast.success('Diagnostica Teams salvata', { description: path, duration: 10000 });
    } catch (error) {
      toast.error('Diagnostica Teams non riuscita', { description: String(error) });
    } finally {
      setDumping(false);
    }
  };

  if (!settings) {
    return (
      <div className="animate-pulse">
        <div className="h-4 bg-gray-200 rounded w-1/4 mb-4"></div>
        <div className="h-8 bg-gray-200 rounded mb-4"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-4">Automazione</h3>
        <p className="text-sm text-gray-600 mb-6">
          Dixit avvia e ferma la trascrizione da solo quando inizia o finisce una call su Teams, Zoom o Meet.
        </p>
      </div>

      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">Avvio automatico sulle call</div>
          <div className="text-sm text-gray-600">
            All&apos;avvio compare un avviso con il pulsante Annulla.
          </div>
        </div>
        <Switch checked={settings.auto_start} onCheckedChange={(v) => save({ auto_start: v })} />
      </div>

      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">Avvia la trascrizione nativa di Teams</div>
          <div className="text-sm text-gray-600">
            Visibile a tutti i partecipanti; serve il permesso nella riunione. Migliora i nomi dei parlanti.
          </div>
        </div>
        <Switch
          checked={settings.native_teams_transcription}
          onCheckedChange={(v) => save({ native_teams_transcription: v })}
        />
      </div>

      <div className="p-4 border rounded-lg space-y-2">
        <label htmlFor="user-display-name" className="font-medium block">
          Il tuo nome
        </label>
        <div className="text-sm text-gray-600">Come appare in Teams (es. Rossi Mario): etichetta i tuoi interventi.</div>
        <Input
          id="user-display-name"
          value={settings.user_display_name}
          placeholder="Cognome Nome"
          onChange={(e) => setSettings({ ...settings, user_display_name: e.target.value })}
          onBlur={() => save({})}
        />
      </div>

      <div className="p-4 border rounded-lg space-y-3">
        <div className="font-medium">Tempi di rilevamento</div>
        <div className="grid grid-cols-2 gap-4">
          <label className="text-sm text-gray-600 space-y-1">
            <span className="block">Secondi prima dell&apos;avvio</span>
            <Input
              type="number"
              min={1}
              value={settings.start_after_secs}
              onChange={(e) => save({ start_after_secs: Math.max(1, Number(e.target.value) || 1) })}
            />
          </label>
          <label className="text-sm text-gray-600 space-y-1">
            <span className="block">Secondi prima dello stop</span>
            <Input
              type="number"
              min={10}
              value={settings.stop_after_secs}
              onChange={(e) => save({ stop_after_secs: Math.max(10, Number(e.target.value) || 10) })}
            />
          </label>
        </div>
      </div>

      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">Accessibilità</div>
          <div className="text-sm text-gray-600">
            Serve per leggere la finestra di Teams (titolo, sottotitoli, trascrizione nativa).
          </div>
        </div>
        {trusted ? (
          <span className="text-sm text-green-700">Concessa</span>
        ) : (
          <button
            onClick={requestAccessibility}
            className="px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-50 transition-colors"
          >
            Concedi
          </button>
        )}
      </div>

      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">Diagnostica Teams</div>
          <div className="text-sm text-gray-600">
            Durante una call, salva la struttura della finestra di Teams per correggere il riconoscimento dopo un aggiornamento.
          </div>
        </div>
        <button
          onClick={dumpTeams}
          disabled={dumping}
          className="px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-50 transition-colors disabled:opacity-50"
        >
          {dumping ? 'Salvo…' : 'Salva diagnostica'}
        </button>
      </div>
    </div>
  );
}
