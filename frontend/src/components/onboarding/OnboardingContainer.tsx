import React from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { ProgressIndicator } from './shared/ProgressIndicator';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { t } from '@/i18n/onboarding';
import type { OnboardingContainerProps } from '@/types/onboarding';

// hideHeader: the step draws its own heading (the welcome screen).
type ContainerProps = OnboardingContainerProps & { hideHeader?: boolean };

export function OnboardingContainer({
  title,
  description,
  children,
  step,
  totalSteps = 5,
  stepOffset = 0,
  hideProgress = false,
  hideHeader = false,
  className,
  showNavigation = false,
  onNext,
  onPrevious,
  canGoNext = true,
  canGoPrevious = true,
}: ContainerProps) {
  const { goToStep, goPrevious, goNext } = useOnboarding();

  const handlePrevious = () => {
    if (onPrevious) {
      onPrevious();
    } else {
      goPrevious();
    }
  };

  const handleNext = () => {
    if (onNext) {
      onNext();
    } else {
      goNext();
    }
  };

  const handleStepClick = (s: number) => {
    goToStep(s + stepOffset);
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-center overflow-hidden bg-background">
      <div className={cn('flex h-full max-h-screen w-full max-w-xl flex-col px-6 py-8', className)}>
        {/* Steps: primary colour for the current one, hairlines for the others */}
        {step && !hideProgress && (
          <div className="mb-8 flex flex-shrink-0 items-center gap-2">
            {showNavigation && (
              <Button
                variant="ghost"
                size="icon"
                aria-label={t('back')}
                onClick={handlePrevious}
                disabled={!canGoPrevious || step === 1}
                className={cn((!canGoPrevious || step === 1) && 'invisible')}
              >
                <ChevronLeft />
              </Button>
            )}

            <ProgressIndicator current={step} total={totalSteps} onStepClick={handleStepClick} />

            {showNavigation && (
              <Button
                variant="ghost"
                size="icon"
                aria-label={t('next')}
                onClick={handleNext}
                disabled={!canGoNext || step === totalSteps}
                className={cn((!canGoNext || step === totalSteps) && 'invisible')}
              >
                <ChevronRight />
              </Button>
            )}
          </div>
        )}

        {!hideHeader && (
          <div className="mb-6 flex-shrink-0 space-y-1">
            <h1 className="text-xl font-semibold">{title}</h1>
            {description && <p className="text-sm text-muted-foreground">{description}</p>}
          </div>
        )}

        {/* Content: scrolls when the window is short */}
        <div className="flex flex-1 flex-col overflow-y-auto">{children}</div>
      </div>
    </div>
  );
}
