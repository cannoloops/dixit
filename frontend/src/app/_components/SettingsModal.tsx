import { ModelConfig } from "@/components/ModelSettingsModal";
import { PreferenceSettings } from "@/components/PreferenceSettings";
import { DeviceSelection } from "@/components/DeviceSelection";
import { LanguageSelection } from "@/components/LanguageSelection";
import { TranscriptSettings } from "@/components/TranscriptSettings";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { AlertCircle, Check } from "lucide-react";
import { toast } from "sonner";
import { useConfig } from "@/contexts/ConfigContext";
import { useRecordingState } from "@/contexts/RecordingStateContext";
import { cn } from "@/lib/utils";
import { t } from "@/i18n/recording";
import { t as tApp } from "@/i18n/app";

type modalType = "modelSettings" | "deviceSettings" | "languageSettings" | "modelSelector" | "errorAlert" | "chunkDropWarning";

/**
 * SettingsModals Component
 *
 * All settings dialogs of the live page, on the shared ui/dialog.
 * Uses ConfigContext and RecordingStateContext internally - no prop drilling needed!
 */

interface SettingsModalsProps {
  modals: {
    modelSettings: boolean;
    deviceSettings: boolean;
    languageSettings: boolean;
    modelSelector: boolean;
    errorAlert: boolean;
    chunkDropWarning: boolean;
  };
  messages: {
    errorAlert: string;
    chunkDropWarning: string;
    modelSelector: string;
  };
  onClose: (name: modalType) => void;
}

const selectClass =
  "h-8 rounded-md border border-input bg-background px-2.5 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring";

export function SettingsModals({
  modals,
  messages,
  onClose,
}: SettingsModalsProps) {
  // Contexts
  const {
    modelConfig,
    setModelConfig,
    models,
    modelOptions,
    error,
    selectedDevices,
    setSelectedDevices,
    selectedLanguage,
    setSelectedLanguage,
    transcriptModelConfig,
    setTranscriptModelConfig,
    showConfidenceIndicator,
    toggleConfidenceIndicator,
  } = useConfig();

  const { isRecording } = useRecordingState();

  // Radix calls onOpenChange(false) on Esc, outside click and the close button.
  const closeOn = (name: modalType) => (open: boolean) => {
    if (!open) onClose(name);
  };

  return <>
    {/* Legacy settings dialog */}
    <Dialog open={modals.modelSettings} onOpenChange={closeOn("modelSettings")}>
      <DialogContent className="flex max-h-[85vh] max-w-3xl flex-col gap-0 p-0">
        <DialogHeader className="border-b border-border px-6 py-4">
          <DialogTitle>{t('preferences')}</DialogTitle>
        </DialogHeader>

        <div className="flex-1 space-y-6 overflow-y-auto px-6 py-4">
          <PreferenceSettings />

          <section className="space-y-3 border-t border-border pt-6">
            <h4 className="text-base font-semibold">{t('summaryModel')}</h4>
            <div className="flex gap-2">
              <select
                className={selectClass}
                value={modelConfig.provider}
                aria-label={t('summaryModel')}
                onChange={(e) => {
                  const provider = e.target.value as ModelConfig['provider'];
                  setModelConfig({
                    ...modelConfig,
                    provider,
                    model: modelOptions[provider][0]
                  });
                }}
              >
                <option value="builtin-ai">{t('builtinAi')}</option>
                <option value="claude">Claude</option>
                <option value="groq">Groq</option>
                <option value="ollama">Ollama</option>
                <option value="openrouter">OpenRouter</option>
                <option value="openai">OpenAI</option>
              </select>

              <select
                className={cn(selectClass, "min-w-0 flex-1")}
                value={modelConfig.model}
                aria-label={t('model')}
                onChange={(e) => setModelConfig((prev: ModelConfig) => ({ ...prev, model: e.target.value }))}
              >
                {modelOptions[modelConfig.provider].map((model: string) => (
                  <option key={model} value={model}>
                    {model}
                  </option>
                ))}
              </select>
            </div>

            {modelConfig.provider === 'ollama' && (
              <div className="space-y-2 pt-2">
                <h4 className="text-sm font-semibold">{t('ollamaModels')}</h4>
                {error && <p className="text-sm text-destructive">{error}</p>}
                <div className="max-h-[400px] divide-y divide-border overflow-y-auto border-y border-border">
                  {models.map((model) => {
                    const selected = modelConfig.model === model.name;
                    return (
                      <button
                        key={model.id}
                        type="button"
                        className={cn(
                          "flex h-8 w-full items-center gap-3 px-2 text-left text-sm transition-colors hover:bg-accent",
                          selected && "bg-accent"
                        )}
                        onClick={() => setModelConfig((prev: ModelConfig) => ({ ...prev, model: model.name }))}
                      >
                        <span className="min-w-0 flex-1 truncate font-medium">{model.name}</span>
                        <span className="text-xs tabular-nums text-muted-foreground">{model.size}</span>
                        <span className="hidden text-xs text-muted-foreground sm:inline">
                          {t('modelModified', { date: model.modified })}
                        </span>
                        <Check className={cn("h-4 w-4 text-primary", !selected && "invisible")} />
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </section>
        </div>

        <DialogFooter className="border-t border-border px-6 py-3">
          <Button onClick={() => onClose('modelSettings')}>{t('done')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    {/* Device settings */}
    <Dialog open={modals.deviceSettings} onOpenChange={closeOn('deviceSettings')}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t('audioDevices')}</DialogTitle>
        </DialogHeader>

        <DeviceSelection
          selectedDevices={selectedDevices}
          onDeviceChange={setSelectedDevices}
          disabled={isRecording}
        />

        <DialogFooter>
          <Button
            onClick={() => {
              const mic = selectedDevices.micDevice || t('defaultDevice');
              const system = selectedDevices.systemDevice || t('defaultDevice');
              toast.success(t('devicesSelected'), {
                description: t('devicesSelectedDesc', { mic, system })
              });
              onClose('deviceSettings');
            }}
          >
            {t('done')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    {/* Language settings */}
    <Dialog open={modals.languageSettings} onOpenChange={closeOn('languageSettings')}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t('transcriptionLanguage')}</DialogTitle>
        </DialogHeader>

        <LanguageSelection
          selectedLanguage={selectedLanguage}
          onLanguageChange={setSelectedLanguage}
          disabled={isRecording}
          provider={transcriptModelConfig.provider}
        />

        <DialogFooter>
          <Button onClick={() => onClose('languageSettings')}>{t('done')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    {/* Transcription model selection */}
    <Dialog open={modals.modelSelector} onOpenChange={closeOn('modelSelector')}>
      <DialogContent className="flex max-h-[85vh] max-w-3xl flex-col gap-0 p-0">
        <DialogHeader className="border-b border-border px-6 py-4">
          <DialogTitle>
            {messages.modelSelector ? t('modelSetupRequired') : t('transcriptionModel')}
          </DialogTitle>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-6 py-4">
          <TranscriptSettings
            transcriptModelConfig={transcriptModelConfig}
            setTranscriptModelConfig={setTranscriptModelConfig}
            onModelSelect={() => onClose('modelSelector')}
          />
        </div>

        <div className="flex items-center justify-between gap-4 border-t border-border px-6 py-3">
          <label className="flex cursor-pointer items-center gap-3">
            <Switch
              checked={showConfidenceIndicator}
              onCheckedChange={toggleConfidenceIndicator}
            />
            <span>
              <span className="block text-sm font-medium">{t('showConfidence')}</span>
              <span className="block text-xs text-muted-foreground">{t('showConfidenceHint')}</span>
            </span>
          </label>

          <Button variant="outline" onClick={() => onClose('modelSelector')}>
            {messages.modelSelector ? tApp('cancel') : t('done')}
          </Button>
        </div>
      </DialogContent>
    </Dialog>

    {/* Recording stopped by an error */}
    <Dialog open={modals.errorAlert} onOpenChange={closeOn('errorAlert')}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertCircle className="h-4 w-4 text-destructive" />
            {t('recordingStopped')}
          </DialogTitle>
          <DialogDescription>{messages.errorAlert}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onClose('errorAlert')}>{tApp('close')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    {/* Transcription falling behind */}
    <Dialog open={modals.chunkDropWarning} onOpenChange={closeOn('chunkDropWarning')}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertCircle className="h-4 w-4 text-primary" />
            {t('chunkDropTitle')}
          </DialogTitle>
          <DialogDescription>{messages.chunkDropWarning}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onClose('chunkDropWarning')}>{tApp('close')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </>
}
