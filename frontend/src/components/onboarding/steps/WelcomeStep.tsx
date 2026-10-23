import React from 'react';
import { Button } from '@/components/ui/button';
import { DixitMark } from '@/components/dixit/DixitMark';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { t } from '@/i18n/onboarding';

export function WelcomeStep() {
  const { goNext } = useOnboarding();

  return (
    <OnboardingContainer title="Dixit" step={1} hideProgress={true} hideHeader={true}>
      <div className="my-auto flex flex-col items-center text-center">
        <DixitMark className="h-20 w-20 text-5xl text-primary" />

        <h1 className="incisa mt-6 text-6xl">Dixit</h1>

        <p className="mt-6 max-w-md text-base">{t('welcomeTagline')}</p>

        <Button size="lg" onClick={goNext} className="mt-10 min-w-40">
          {t('welcomeStart')}
        </Button>

        <p className="mt-4 max-w-sm text-xs text-muted-foreground">{t('welcomeNote')}</p>
      </div>
    </OnboardingContainer>
  );
}
