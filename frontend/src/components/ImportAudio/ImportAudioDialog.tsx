import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import {
  Upload,
  Globe,
  Loader2,
  AlertCircle,
  CheckCircle2,
  X,
  Cpu,
  FileAudio,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select';
import { toast } from 'sonner';
import { useConfig } from '@/contexts/ConfigContext';
import { useImportAudio, ImportResult } from '@/hooks/useImportAudio';
import { useRouter } from 'next/navigation';
import { useSidebar } from '../Sidebar/SidebarProvider';
import { LANGUAGES } from '@/constants/languages';
import { useTranscriptionModels, ModelOption } from '@/hooks/useTranscriptionModels';
import { Progress } from '../ui/progress';
import { t } from '@/i18n/recording';
import { t as tApp } from '@/i18n/app';


interface ImportAudioDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  preselectedFile?: string | null;
  onComplete?: () => void;
}

function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

export function ImportAudioDialog({
  open,
  onOpenChange,
  preselectedFile,
  onComplete,
}: ImportAudioDialogProps) {
  const router = useRouter();
  const { refetchMeetings } = useSidebar();
  const { selectedLanguage, transcriptModelConfig } = useConfig();

  const [title, setTitle] = useState('');
  const [selectedLang, setSelectedLang] = useState(selectedLanguage || 'auto');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [titleModifiedByUser, setTitleModifiedByUser] = useState(false);

  // Always start as false — represents "dialog has not yet been opened".
  // Do NOT initialize from the `open` prop: if the component mounts with open=true
  // (e.g. drag-drop path), we still need the initialization effect to run.
  const prevOpenRef = useRef(false);

  // Use centralized model fetching hook
  const {
    availableModels,
    selectedModelKey,
    setSelectedModelKey,
    loadingModels,
    fetchModels,
    resetSelection,
  } = useTranscriptionModels(transcriptModelConfig);

  const handleImportComplete = useCallback((result: ImportResult) => {
    toast.success(t('importComplete', { count: result.segments_count }));

    // Refresh meetings list then navigate to the imported meeting
    refetchMeetings();
    onComplete?.();
    onOpenChange(false);
    router.push(`/meeting-details?id=${result.meeting_id}`);
  }, [router, refetchMeetings, onComplete, onOpenChange]);

  const handleImportError = useCallback((error: string) => {
    toast.error(t('importFailed'), { description: error });
  }, []);

  const {
    status,
    fileInfo,
    progress,
    error,
    isProcessing,
    isBusy,
    selectFile,
    validateFile,
    startImport,
    cancelImport,
    reset,
  } = useImportAudio({
    onComplete: handleImportComplete,
    onError: handleImportError,
  });

  // Reset state only when dialog transitions from closed to open
  // This prevents re-initialization when config changes while dialog is already open (Bug #4 & #5)
  useEffect(() => {
    const wasOpen = prevOpenRef.current;
    prevOpenRef.current = open;

    // Only initialize when transitioning from closed (false) to open (true)
    if (open && !wasOpen) {
      reset();
      resetSelection();
      setTitle('');
      setTitleModifiedByUser(false);
      setSelectedLang(selectedLanguage || 'auto');
      setShowAdvanced(false);

      // Validate preselected file if provided
      if (preselectedFile) {
        validateFile(preselectedFile).then((info) => {
          if (info) {
            setTitle(info.filename);
          }
        });
      }

      // Fetch available models using centralized hook
      fetchModels();
    }
  }, [open, preselectedFile, selectedLanguage, transcriptModelConfig, reset, resetSelection, validateFile, fetchModels]);

  // Update title when fileInfo changes
  useEffect(() => {
    if (fileInfo && !title && !titleModifiedByUser) {
      setTitle(fileInfo.filename);
    }
  }, [fileInfo, title, titleModifiedByUser]);

  const selectedModel = useMemo((): ModelOption | undefined => {
    if (!selectedModelKey) return undefined;
    const colonIndex = selectedModelKey.indexOf(':');
    if (colonIndex === -1) return undefined;
    const provider = selectedModelKey.slice(0, colonIndex);
    const name = selectedModelKey.slice(colonIndex + 1);
    return availableModels.find((m) => m.provider === provider && m.name === name);
  }, [selectedModelKey, availableModels]);
  const isParakeetModel = selectedModel?.provider === 'parakeet';

  useEffect(() => {
    if (isParakeetModel && selectedLang !== 'auto') {
      setSelectedLang('auto');
    }
  }, [isParakeetModel, selectedLang]);

  const handleSelectFile = async () => {
    const info = await selectFile();
    if (info) {
      setTitle(info.filename);
    }
  };

  const handleStartImport = async () => {
    if (!fileInfo) return;

    await startImport(
      fileInfo.path,
      title || fileInfo.filename,
      isParakeetModel ? null : selectedLang === 'auto' ? null : selectedLang,
      selectedModel?.name || null,
      selectedModel?.provider || null
    );
  };

  const handleCancel = async () => {
    if (isProcessing) {
      await cancelImport();
      toast.info(t('importCancelled'));
    }
    onOpenChange(false);
  };

  // Prevent closing during processing
  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen && isProcessing) {
      return;
    }
    onOpenChange(newOpen);
  };

  const handleEscapeKeyDown = (event: KeyboardEvent) => {
    if (isProcessing) {
      event.preventDefault();
    }
  };

  const handleInteractOutside = (event: Event) => {
    if (isProcessing) {
      event.preventDefault();
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        className="sm:max-w-[500px]"
        onEscapeKeyDown={handleEscapeKeyDown}
        onInteractOutside={handleInteractOutside}
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {isProcessing ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin text-primary" />
                {t('importing')}
              </>
            ) : error ? (
              <>
                <AlertCircle className="h-4 w-4 text-destructive" />
                {t('importFailed')}
              </>
            ) : status === 'complete' ? (
              <>
                <CheckCircle2 className="h-4 w-4 text-primary" />
                {t('importDone')}
              </>
            ) : (
              <>
                <Upload className="h-4 w-4 text-primary" />
                {t('importTitle')}
              </>
            )}
          </DialogTitle>
          <DialogDescription>
            {isProcessing
              ? progress?.message || t('processingAudio')
              : error
              ? t('importErrorDesc')
              : t('importDesc')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* File selection / info */}
          {!isProcessing && !error && (
            <>
              {fileInfo ? (
                <div className="space-y-3">
                  <div className="flex items-start gap-3">
                    <FileAudio className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium">{fileInfo.filename}</p>
                      <p className="mt-0.5 text-xs tabular-nums text-muted-foreground">
                        {formatDuration(fileInfo.duration_seconds)} · {formatFileSize(fileInfo.size_bytes)} · {fileInfo.format}
                      </p>
                    </div>
                    <Button variant="ghost" size="sm" onClick={handleSelectFile}>
                      {t('chooseOtherFile')}
                    </Button>
                  </div>

                  {/* Editable title */}
                  <div className="space-y-1">
                    <label htmlFor="import-title" className="text-sm font-medium">{t('meetingTitle')}</label>
                    <Input
                      id="import-title"
                      value={title}
                      onChange={(e) => {
                        setTitle(e.target.value);
                        setTitleModifiedByUser(true);
                      }}
                      placeholder={t('meetingTitlePlaceholder')}
                    />
                  </div>
                </div>
              ) : (
                <div className="rounded-md border border-dashed border-input px-6 py-8 text-center">
                  <Button onClick={handleSelectFile} disabled={status === 'validating'}>
                    {status === 'validating' ? <Loader2 className="animate-spin" /> : <Upload />}
                    {status === 'validating' ? t('validating') : t('selectAudioFile')}
                  </Button>
                  <p className="mt-2 text-xs text-muted-foreground">MP4, WAV, MP3, FLAC, OGG, MKV, WebM, WMA</p>
                </div>
              )}

              {/* Advanced options (collapsible) */}
              {fileInfo && (
                <div className="border-t border-border pt-2">
                  <button
                    type="button"
                    onClick={() => setShowAdvanced(!showAdvanced)}
                    aria-expanded={showAdvanced}
                    className="flex h-8 w-full items-center justify-between rounded-sm px-1 text-sm font-medium transition-colors hover:bg-accent"
                  >
                    <span>{t('advancedOptions')}</span>
                    {showAdvanced ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                  </button>

                  {showAdvanced && (
                    <div className="space-y-4 px-1 pt-3">
                      {/* Language selector */}
                      <div className="space-y-2">
                        <div className="flex items-center gap-2">
                          <Globe className="h-4 w-4 text-muted-foreground" />
                          <span className="text-sm font-medium">{t('language')}</span>
                        </div>
                        {!isParakeetModel ? (
                          <Select value={selectedLang} onValueChange={setSelectedLang}>
                            <SelectTrigger className="w-full">
                              <SelectValue placeholder={t('selectLanguage')} />
                            </SelectTrigger>
                            <SelectContent className="max-h-60">
                              {LANGUAGES.map((lang) => (
                                <SelectItem key={lang.code} value={lang.code}>
                                  {lang.name}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        ) : (
                          <p className="text-xs text-muted-foreground">{t('parakeetNoLanguage')}</p>
                        )}
                      </div>

                      {/* Model selector */}
                      {availableModels.length > 0 && (
                        <div className="space-y-2">
                          <div className="flex items-center gap-2">
                            <Cpu className="h-4 w-4 text-muted-foreground" />
                            <span className="text-sm font-medium">{t('model')}</span>
                          </div>
                          <Select
                            value={selectedModelKey}
                            onValueChange={setSelectedModelKey}
                            disabled={loadingModels}
                          >
                            <SelectTrigger className="w-full">
                              <SelectValue placeholder={loadingModels ? t('loadingModels') : t('selectModel')} />
                            </SelectTrigger>
                            <SelectContent>
                              {availableModels.map((model) => (
                                <SelectItem
                                  key={`${model.provider}:${model.name}`}
                                  value={`${model.provider}:${model.name}`}
                                >
                                  {model.displayName} ({Math.round(model.size_mb)} MB)
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              )}
            </>
          )}

          {/* Progress display */}
          {isProcessing && progress && (
            <div className="space-y-1">
              <Progress value={Math.min(progress.progress_percentage, 100)} className="h-1.5" />
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>{progress.stage}</span>
                <span className="tabular-nums">{Math.round(progress.progress_percentage)}%</span>
              </div>
            </div>
          )}

          {/* Error display */}
          {error && (
            <p className="rounded-md border border-destructive/40 px-3 py-2 text-sm text-destructive">{error}</p>
          )}
        </div>

        <DialogFooter>
          {!isProcessing && !error && (
            <>
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                {tApp('cancel')}
              </Button>
              <Button onClick={handleStartImport} disabled={!fileInfo}>
                <Upload />
                {t('import')}
              </Button>
            </>
          )}
          {isProcessing && (
            <Button variant="outline" onClick={handleCancel}>
              <X />
              {tApp('cancel')}
            </Button>
          )}
          {error && (
            <>
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                {tApp('close')}
              </Button>
              <Button onClick={reset}>
                {t('tryAgain')}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
