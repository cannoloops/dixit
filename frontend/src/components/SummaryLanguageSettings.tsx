'use client';

import { useState } from 'react';
import { Pin, Plus, X } from 'lucide-react';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import { LanguagePickerPopover } from '@/components/LanguagePickerPopover';
import { SettingsSection } from '@/components/molecules/form-components/setting-row';
import { useRecentLanguages } from '@/hooks/useRecentLanguages';
import { labelForCode } from '@/lib/summary-languages';
import { t } from '@/i18n/settings';

export function SummaryLanguageSettings() {
  const { recents, pinned, addRecent, removeRecent, setPinned } = useRecentLanguages();
  const [pickerOpen, setPickerOpen] = useState(false);

  const togglePin = (code: string) => {
    setPinned(pinned === code ? null : code);
  };

  return (
    <SettingsSection title={t('summaryLangSection')} description={t('summaryLangDesc')}>
      <div className="space-y-2 py-3">
        <div className="flex flex-wrap items-center gap-2">
          {recents.map((code) => {
            const isPinned = pinned === code;
            const label = labelForCode(code);
            return (
              <span
                key={code}
                className={`inline-flex items-center overflow-hidden rounded-md border text-sm transition-colors ${
                  isPinned ? 'border-primary bg-accent text-foreground' : 'border-input bg-background text-foreground'
                }`}
              >
                <button
                  type="button"
                  aria-label={isPinned ? t('unpinLang', { lang: label }) : t('pinLang', { lang: label })}
                  aria-pressed={isPinned}
                  title={isPinned ? t('pinTitleOn') : t('pinTitleOff')}
                  onClick={() => togglePin(code)}
                  className="flex items-center gap-1.5 py-1 pl-2.5 pr-2 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-ring"
                >
                  <Pin
                    className={`h-4 w-4 ${isPinned ? 'text-primary' : 'text-muted-foreground'}`}
                    fill={isPinned ? 'currentColor' : 'none'}
                  />
                  {label}
                </button>
                <button
                  type="button"
                  aria-label={t('removeLang', { lang: label })}
                  onClick={() => removeRecent(code)}
                  className="py-1 pl-0.5 pr-2 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-ring"
                >
                  <X className="h-4 w-4" />
                </button>
              </span>
            );
          })}

          <Popover open={pickerOpen} onOpenChange={setPickerOpen}>
            <PopoverTrigger asChild>
              <button
                type="button"
                disabled={recents.length >= 5}
                className="inline-flex items-center gap-1 rounded-md border border-dashed border-input px-2.5 py-1 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
              >
                <Plus className="h-4 w-4" />
                {t('addLanguage')}
              </button>
            </PopoverTrigger>
            <PopoverContent align="start" className="w-auto border-0 bg-transparent p-0 shadow-none">
              <LanguagePickerPopover
                mode="settings"
                value={null}
                onChange={(code) => {
                  if (code) addRecent(code);
                  setPickerOpen(false);
                }}
                onClose={() => setPickerOpen(false)}
              />
            </PopoverContent>
          </Popover>
        </div>

        <p className="text-xs text-muted-foreground">
          {pinned ? t('pinnedHint', { lang: labelForCode(pinned) }) : t('unpinnedHint')}
        </p>
      </div>
    </SettingsSection>
  );
}
