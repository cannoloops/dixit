"use client";

import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, Save, Loader2 } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { t } from '@/i18n/meeting';

interface SummaryUpdaterButtonGroupProps {
  isSaving: boolean;
  isDirty: boolean;
  onSave: () => Promise<void>;
  onCopy: () => Promise<void>;
}

export function SummaryUpdaterButtonGroup({
  isSaving,
  isDirty,
  onSave,
  onCopy,
}: SummaryUpdaterButtonGroupProps) {
  return (
    <ButtonGroup>
      <Button
        variant="outline"
        size="sm"
        title={isSaving ? t('saving') : isDirty ? t('unsavedChanges') : t('saveChanges')}
        onClick={() => {
          Analytics.trackButtonClick('save_changes', 'meeting_details');
          onSave();
        }}
        disabled={isSaving}
      >
        {isSaving ? <Loader2 className="animate-spin" /> : <Save />}
        <span className="hidden @[40rem]:inline">{isSaving ? t('saving') : t('save')}</span>
        {isDirty && !isSaving && (
          <span aria-hidden className="h-1.5 w-1.5 rounded-full bg-primary" />
        )}
      </Button>

      <Button
        variant="outline"
        size="sm"
        title={t('copySummary')}
        onClick={() => {
          Analytics.trackButtonClick('copy_summary', 'meeting_details');
          onCopy();
        }}
      >
        <Copy />
        <span className="hidden @[40rem]:inline">{t('copy')}</span>
      </Button>
    </ButtonGroup>
  );
}
