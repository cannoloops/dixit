import React from 'react';
import { cn } from '@/lib/utils';
import { t } from '@/i18n/onboarding';

interface ProgressIndicatorProps {
  current: number;
  total: number;
  onStepClick?: (step: number) => void;
}

/** One hairline per step: bronze and heavier for the current one, a faint bronze for the done ones. */
export function ProgressIndicator({ current, total, onStepClick }: ProgressIndicatorProps) {
  const steps = Array.from({ length: total }, (_, i) => i + 1);

  return (
    <nav aria-label={t('progress')} className="flex min-w-0 flex-1 items-center gap-3">
      <ol className="flex flex-1 items-center gap-1.5">
        {steps.map((step) => {
          const isCurrent = step === current;
          const isCompleted = step < current;
          const isClickable = isCompleted && Boolean(onStepClick);

          return (
            <li key={step} className="flex-1">
              <button
                type="button"
                onClick={() => onStepClick?.(step)}
                disabled={!isClickable}
                aria-label={isClickable ? t('goToStep', { step }) : t('stepOf', { current: step, total })}
                aria-current={isCurrent ? 'step' : undefined}
                className="group flex h-6 w-full items-center rounded-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-default"
              >
                <span
                  className={cn(
                    'w-full transition-colors duration-150',
                    isCurrent && 'h-0.5 bg-primary',
                    isCompleted && 'h-px bg-primary/50 group-enabled:group-hover:bg-primary',
                    !isCurrent && !isCompleted && 'h-px bg-border'
                  )}
                />
              </button>
            </li>
          );
        })}
      </ol>
      <span className="flex-shrink-0 text-xs tabular-nums text-muted-foreground">
        {t('stepOf', { current, total })}
      </span>
    </nav>
  );
}
