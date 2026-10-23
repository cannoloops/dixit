'use client';

import React from 'react';
import { Shield } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { t } from '@/i18n/shell';

interface AnalyticsDataModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirmDisable: () => void;
}

const CATEGORIES = [
  ['adModelsTitle', 'adModels', 'adModelsWhy'],
  ['adMetricsTitle', 'adMetrics', 'adMetricsWhy'],
  ['adDevicesTitle', 'adDevices', 'adDevicesWhy'],
  ['adUsageTitle', 'adUsage', 'adUsageWhy'],
  ['adPlatformTitle', 'adPlatform', 'adPlatformWhy'],
] as const;

const EXAMPLE_EVENT = `{
  "event": "meeting_ended",
  "app_version": "0.4.1",
  "transcription_provider": "parakeet",
  "transcription_model": "parakeet-tdt-0.6b-v3-int8",
  "summary_provider": "ollama",
  "summary_model": "llama3.2:latest",
  "total_duration_seconds": "125.5",
  "microphone_device_type": "Wired",
  "system_audio_device_type": "Bluetooth",
  "chunks_processed": "150",
  "had_fatal_error": "false"
}`;

export default function AnalyticsDataModal({ isOpen, onClose, onConfirmDisable }: AnalyticsDataModalProps) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-primary" />
            {t('adTitle')}
          </DialogTitle>
          <DialogDescription>{t('adPrivacyBody')}</DialogDescription>
        </DialogHeader>

        <section>
          <h3 className="text-sm font-semibold">{t('adCollected')}</h3>
          <div className="mt-2 divide-y divide-border border-y border-border">
            {CATEGORIES.map(([title, items, why]) => (
              <div key={title} className="py-3">
                <h4 className="text-sm font-medium">{t(title)}</h4>
                <ul className="mt-1 ml-4 list-disc space-y-0.5 text-sm text-muted-foreground">
                  {t(items).split('\n').map((item) => <li key={item}>{item}</li>)}
                </ul>
                <p className="mt-1 text-xs text-muted-foreground">{t(why)}</p>
              </div>
            ))}
          </div>
        </section>

        <section>
          <h3 className="text-sm font-semibold">{t('adNotCollectedTitle')}</h3>
          <ul className="mt-1 ml-4 list-disc space-y-0.5 text-sm text-muted-foreground">
            {t('adNotCollected').split('\n').map((item) => <li key={item}>{item}</li>)}
          </ul>
        </section>

        <section>
          <h3 className="text-sm font-semibold">{t('adExample')}</h3>
          <pre className="mt-2 overflow-x-auto rounded-sm border border-border bg-muted p-3 font-mono text-xs">
            {EXAMPLE_EVENT}
          </pre>
        </section>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t('adKeep')}
          </Button>
          <Button onClick={onConfirmDisable}>
            {t('adDisable')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
