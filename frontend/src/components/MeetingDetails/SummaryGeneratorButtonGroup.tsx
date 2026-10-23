"use client";

import { ModelConfig, ModelSettingsModal } from '@/components/ModelSettingsModal';
import {
  Dialog,
  DialogContent,
  DialogTrigger,
  DialogTitle,
} from "@/components/ui/dialog"
import { VisuallyHidden } from "@/components/ui/visually-hidden"
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Sparkles, Settings, Loader2, FileText, Check } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { useState, useEffect, ReactNode } from 'react';
import { t } from '@/i18n/meeting';

type SummaryStatus = 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error';

interface GenerateSummaryButtonProps {
  summaryStatus: SummaryStatus;
  hasSummary: boolean;
  hasTranscripts: boolean;
  isModelConfigLoading: boolean;
  onGenerate: () => void;
  onStop: () => void;
}

/** The meeting's primary action (top right): generate the summary, or stop it while it runs. */
export function GenerateSummaryButton({
  summaryStatus,
  hasSummary,
  hasTranscripts,
  isModelConfigLoading,
  onGenerate,
  onStop,
}: GenerateSummaryButtonProps) {
  const isGenerating = summaryStatus === 'processing' || summaryStatus === 'summarizing' || summaryStatus === 'regenerating';

  if (isGenerating) {
    return (
      <Button
        variant="outline"
        title={t('stopTitle')}
        onClick={() => {
          Analytics.trackButtonClick('stop_summary_generation', 'meeting_details');
          onStop();
        }}
      >
        <Loader2 className="animate-spin" />
        {t('stop')}
      </Button>
    );
  }

  return (
    <Button
      variant={hasSummary ? 'outline' : 'default'}
      disabled={isModelConfigLoading || !hasTranscripts}
      title={
        isModelConfigLoading ? t('loadingModel') : !hasTranscripts ? t('noTranscriptToSummarize') : undefined
      }
      onClick={() => {
        Analytics.trackButtonClick('generate_summary', 'meeting_details');
        onGenerate();
      }}
    >
      {isModelConfigLoading ? <Loader2 className="animate-spin" /> : <Sparkles />}
      {hasSummary ? t('regenerate') : t('generate')}
    </Button>
  );
}

interface SummaryGeneratorButtonGroupProps {
  languageSlot?: ReactNode;
  modelConfig: ModelConfig;
  setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
  onSaveModelConfig: (config?: ModelConfig) => Promise<void>;
  availableTemplates: Array<{ id: string, name: string, description: string }>;
  selectedTemplate: string;
  onTemplateSelect: (templateId: string, templateName: string) => void;
  hasTranscripts?: boolean;
  onOpenModelSettings?: (openFn: () => void) => void;
}

/** Summary options next to the editor: language, model and template. */
export function SummaryGeneratorButtonGroup({
  modelConfig,
  setModelConfig,
  onSaveModelConfig,
  availableTemplates,
  selectedTemplate,
  onTemplateSelect,
  hasTranscripts = true,
  onOpenModelSettings,
  languageSlot
}: SummaryGeneratorButtonGroupProps) {
  const [settingsDialogOpen, setSettingsDialogOpen] = useState(false);

  // Let the parent open the model dialog (e.g. from an error that needs a model).
  useEffect(() => {
    onOpenModelSettings?.(() => setSettingsDialogOpen(true));
  }, [onOpenModelSettings]);

  if (!hasTranscripts) {
    return null;
  }

  return (
    <ButtonGroup>
      {languageSlot}

      <Dialog open={settingsDialogOpen} onOpenChange={setSettingsDialogOpen}>
        <DialogTrigger asChild>
          <Button variant="outline" size="sm" title={t('summaryModel')}>
            <Settings />
            <span className="hidden @[40rem]:inline">{t('model')}</span>
          </Button>
        </DialogTrigger>
        <DialogContent aria-describedby={undefined}>
          <VisuallyHidden>
            <DialogTitle>{t('modelSettings')}</DialogTitle>
          </VisuallyHidden>
          <ModelSettingsModal
            onSave={async (config) => {
              await onSaveModelConfig(config);
              setSettingsDialogOpen(false);
            }}
            modelConfig={modelConfig}
            setModelConfig={setModelConfig}
            skipInitialFetch={true}
            layout="dialog"
          />
        </DialogContent>
      </Dialog>

      {availableTemplates.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" title={t('chooseTemplate')}>
              <FileText />
              <span className="hidden @[40rem]:inline">{t('template')}</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {availableTemplates.map((template) => (
              <DropdownMenuItem
                key={template.id}
                onClick={() => onTemplateSelect(template.id, template.name)}
                title={template.description}
                className="flex items-center justify-between gap-2"
              >
                <span>{template.name}</span>
                {selectedTemplate === template.id && (
                  <Check className="h-4 w-4 text-primary" />
                )}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </ButtonGroup>
  );
}
