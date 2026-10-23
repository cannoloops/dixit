'use client';

import { Settings } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { t } from '@/i18n/meeting';

interface EmptyStateSummaryProps {
  /** False when no summary model is configured. */
  hasModel: boolean;
  hasTranscripts?: boolean;
  error?: string | null;
  /** Opens the summary model dialog. */
  onChooseModel?: () => void;
}

/** No summary yet, or the last generation failed: says what to do (the action is "Genera riassunto", top right). */
export function EmptyStateSummary({
  hasModel,
  hasTranscripts = true,
  error = null,
  onChooseModel,
}: EmptyStateSummaryProps) {
  return (
    <div className="flex h-full flex-col items-center justify-center p-8 text-center">
      <div className="max-w-sm space-y-2">
        {error ? (
          <>
            <h3 className="text-sm font-semibold text-destructive">{t('statusError')}</h3>
            <p role="alert" className="text-sm text-foreground">{error}</p>
            <p className="text-xs text-muted-foreground">{t('summaryErrorFix')}</p>
          </>
        ) : (
          <>
            <h3 className="text-sm font-semibold">{t('emptyTitle')}</h3>
            <p className="text-sm text-muted-foreground">
              {hasTranscripts ? t('emptyBody') : t('emptyNoTranscript')}
            </p>
            {!hasModel && <p className="text-xs text-muted-foreground">{t('noModel')}</p>}
          </>
        )}

        {(error || !hasModel) && onChooseModel && (
          <Button variant="outline" size="sm" className="mt-2" onClick={onChooseModel}>
            <Settings />
            {t('chooseModel')}
          </Button>
        )}
      </div>
    </div>
  );
}
