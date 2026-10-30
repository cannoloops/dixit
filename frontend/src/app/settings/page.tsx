'use client';

import React, { useEffect, useState } from 'react';
import { ArrowLeft } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { TranscriptSettings } from '@/components/TranscriptSettings';
import { RecordingSettings } from '@/components/RecordingSettings';
import { PreferenceSettings } from '@/components/PreferenceSettings';
import { SummaryModelSettings } from '@/components/SummaryModelSettings';
import { BetaSettings } from '@/components/BetaSettings';
import { AutopilotSettings } from '@/components/AutopilotSettings';
import { PermissionsSettings } from '@/components/PermissionsSettings';
import { useConfig } from '@/contexts/ConfigContext';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { SettingsSection, SettingRow } from '@/components/molecules/form-components/setting-row';
import { t } from '@/i18n/settings';

// Tabs configuration (constant)
const TABS = [
  { value: 'general', label: t('tabGeneral') },
  { value: 'autopilot', label: t('tabAutopilot') },
  { value: 'recording', label: t('tabRecording') },
  { value: 'transcription', label: t('tabTranscription') },
  { value: 'summary', label: t('tabSummary') },
  { value: 'beta', label: t('tabBeta') },
  { value: 'advanced', label: t('tabAdvanced') },
] as const;

const EXPERT_KEY = 'dixit-expert-mode';

export default function SettingsPage() {
  const router = useRouter();
  const { transcriptModelConfig, setTranscriptModelConfig } = useConfig();
  // Dixit: model pickers hidden unless expert mode is on (per-Mac preference).
  const [expert, setExpert] = useState(false);
  useEffect(() => {
    try { setExpert(localStorage.getItem(EXPERT_KEY) === '1'); } catch {}
  }, []);
  const toggleExpert = (on: boolean) => {
    setExpert(on);
    try { localStorage.setItem(EXPERT_KEY, on ? '1' : '0'); } catch {}
  };

  // Load saved transcript configuration on mount
  useEffect(() => {
    const loadTranscriptConfig = async () => {
      try {
        const config = await invoke('api_get_transcript_config') as any;
        if (config) {
          console.log('Loaded saved transcript config:', config);
          setTranscriptModelConfig({
            provider: config.provider || 'localWhisper',
            model: config.model || 'large-v3',
            apiKey: config.apiKey || null
          });
        }
      } catch (error) {
        console.error('Failed to load transcript config:', error);
      }
    };
    loadTranscriptConfig();
  }, [setTranscriptModelConfig]);

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <Tabs defaultValue="general" className="mx-auto flex min-h-0 w-full max-w-3xl flex-1 flex-col px-6">
        <header className="shrink-0 space-y-3 pt-4">
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="sm" onClick={() => router.back()}>
              <ArrowLeft />
              {t('back')}
            </Button>
            <h1 className="text-xl font-semibold">{t('title')}</h1>
          </div>
          <TabsList className="flex w-full">
            {TABS.map((tab) => (
              <TabsTrigger key={tab.value} value={tab.value}>
                {tab.label}
              </TabsTrigger>
            ))}
          </TabsList>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto py-6">
          <TabsContent value="general" className="mt-0 space-y-8">
            <PermissionsSettings />
            <PreferenceSettings />
          </TabsContent>
          <TabsContent value="autopilot" className="mt-0">
            <AutopilotSettings />
          </TabsContent>
          <TabsContent value="recording" className="mt-0">
            <RecordingSettings />
          </TabsContent>
          <TabsContent value="transcription" className="mt-0">
            <TranscriptSettings
              transcriptModelConfig={transcriptModelConfig}
              setTranscriptModelConfig={setTranscriptModelConfig}
              readOnly
            />
          </TabsContent>
          <TabsContent value="summary" className="mt-0">
            <SummaryModelSettings />
          </TabsContent>
          <TabsContent value="beta" className="mt-0">
            <BetaSettings />
          </TabsContent>
          <TabsContent value="advanced" className="mt-0 space-y-8">
            <SettingsSection title={t('expertSection')}>
              <SettingRow label={t('expertMode')} description={t('expertModeDesc')} htmlFor="expert-mode">
                <Switch id="expert-mode" checked={expert} onCheckedChange={toggleExpert} />
              </SettingRow>
            </SettingsSection>
            {expert && (
              <>
                <TranscriptSettings
                  transcriptModelConfig={transcriptModelConfig}
                  setTranscriptModelConfig={setTranscriptModelConfig}
                />
                <SummaryModelSettings picker />
              </>
            )}
          </TabsContent>
        </div>
      </Tabs>
    </div>
  );
}
