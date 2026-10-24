import React, { useEffect, useState } from 'react';
import { Info, Mic, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { t } from '@/i18n/onboarding';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

export function SetupOverviewStep() {
  const { goToStep, completeOnboarding } = useOnboarding();
  const [isMac, setIsMac] = useState(false);

  useEffect(() => {
    const checkPlatform = async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  const models = [
    {
      type: 'transcription',
      icon: Mic,
      title: t('transcriptionEngine'),
      purpose: t('transcriptionPurpose'),
    },
    {
      type: 'summarization',
      icon: Sparkles,
      title: t('summaryEngine'),
      purpose: t('summaryPurpose'),
    },
  ];

  return (
    <OnboardingContainer
      title={t('overviewTitle')}
      description={t('overviewDescription')}
      step={2}
      totalSteps={isMac ? 3 : 2}
    >
      <ul className="divide-y divide-border border-y border-border">
        {models.map((model) => {
          const Icon = model.icon;
          return (
            <li key={model.type} className="flex items-center gap-3 py-3">
              <Icon className="h-4 w-4 flex-shrink-0 text-muted-foreground" aria-hidden />
              <div className="min-w-0 flex-1">
                <p className="flex items-center gap-1.5 text-sm font-medium">
                  {model.title}
                  {model.type === 'summarization' && (
                    <TooltipProvider>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <button
                            type="button"
                            aria-label={t('summaryInfoLabel')}
                            className="rounded-sm text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                          >
                            <Info className="h-4 w-4" />
                          </button>
                        </TooltipTrigger>
                        <TooltipContent className="max-w-xs">{t('summaryInfo')}</TooltipContent>
                      </Tooltip>
                    </TooltipProvider>
                  )}
                </p>
                <p className="text-xs text-muted-foreground">{model.purpose}</p>
              </div>
            </li>
          );
        })}
      </ul>

      <div className="mt-8 flex justify-end">
        {/* Dixit: models are bundled, so no download step: straight to the macOS permissions (step 4). */}
        <Button
          size="lg"
          onClick={async () => {
            if (isMac) return goToStep(4);
            await completeOnboarding();
            window.location.reload();
          }}
        >
          {t('continue')}
        </Button>
      </div>
    </OnboardingContainer>
  );
}
