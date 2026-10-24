import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Eye, EyeOff, Lock, Unlock } from 'lucide-react';
import { ModelManager } from './WhisperModelManager';
import { ParakeetModelManager } from './ParakeetModelManager';
import { SettingsSection, SettingRow } from '@/components/molecules/form-components/setting-row';
import { t } from '@/i18n/settings';
import { ParakeetAPI } from '@/lib/parakeet';


export interface TranscriptModelProps {
    provider: 'localWhisper' | 'parakeet' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai';
    model: string;
    apiKey?: string | null;
}

export interface TranscriptSettingsProps {
    transcriptModelConfig: TranscriptModelProps;
    setTranscriptModelConfig: (config: TranscriptModelProps) => void;
    onModelSelect?: () => void;
}

export function TranscriptSettings({ transcriptModelConfig, setTranscriptModelConfig, onModelSelect }: TranscriptSettingsProps) {
    const [apiKey, setApiKey] = useState<string | null>(transcriptModelConfig.apiKey || null);
    const [showApiKey, setShowApiKey] = useState<boolean>(false);
    const [isApiKeyLocked, setIsApiKeyLocked] = useState<boolean>(true);
    const [isLockButtonVibrating, setIsLockButtonVibrating] = useState<boolean>(false);
    const [uiProvider, setUiProvider] = useState<TranscriptModelProps['provider']>(transcriptModelConfig.provider);

    // With Whisper chosen, the live runs on Parakeet when a Parakeet model is downloaded.
    const [parakeetReady, setParakeetReady] = useState(false);
    useEffect(() => {
        ParakeetAPI.init()
            .then(() => ParakeetAPI.getAvailableModels())
            .then((models) => setParakeetReady(models.some((m) => m.status === 'Available')))
            .catch(() => setParakeetReady(false));
    }, [uiProvider]);

    // Sync uiProvider when backend config changes (e.g., after model selection or initial load)
    useEffect(() => {
        setUiProvider(transcriptModelConfig.provider);
    }, [transcriptModelConfig.provider]);

    useEffect(() => {
        if (transcriptModelConfig.provider === 'localWhisper' || transcriptModelConfig.provider === 'parakeet') {
            setApiKey(null);
        }
    }, [transcriptModelConfig.provider]);

    const fetchApiKey = async (provider: string) => {
        try {

            const data = await invoke('api_get_transcript_api_key', { provider }) as string;

            setApiKey(data || '');
        } catch (err) {
            console.error('Error fetching API key:', err);
            setApiKey(null);
        }
    };
    const modelOptions = {
        localWhisper: [], // Model selection handled by ModelManager component
        parakeet: [], // Model selection handled by ParakeetModelManager component
        deepgram: ['nova-2-phonecall'],
        elevenLabs: ['eleven_multilingual_v2'],
        groq: ['llama-3.3-70b-versatile'],
        openai: ['gpt-4o'],
    };
    const requiresApiKey = transcriptModelConfig.provider === 'deepgram' || transcriptModelConfig.provider === 'elevenLabs' || transcriptModelConfig.provider === 'openai' || transcriptModelConfig.provider === 'groq';

    const handleInputClick = () => {
        if (isApiKeyLocked) {
            setIsLockButtonVibrating(true);
            setTimeout(() => setIsLockButtonVibrating(false), 500);
        }
    };

    const handleWhisperModelSelect = (modelName: string) => {
        // Always update config when model is selected, regardless of current provider
        // This ensures the model is set when user switches back
        setTranscriptModelConfig({
            ...transcriptModelConfig,
            provider: 'localWhisper', // Ensure provider is set correctly
            model: modelName
        });
        // Close modal after selection
        if (onModelSelect) {
            onModelSelect();
        }
    };

    const handleParakeetModelSelect = (modelName: string) => {
        // Always update config when model is selected, regardless of current provider
        // This ensures the model is set when user switches back
        setTranscriptModelConfig({
            ...transcriptModelConfig,
            provider: 'parakeet', // Ensure provider is set correctly
            model: modelName
        });
        // Close modal after selection
        if (onModelSelect) {
            onModelSelect();
        }
    };

    return (
        <div className="space-y-8">
            <SettingsSection title={t('transcriptSection')} description={t('transcriptDesc')}>
                {uiProvider === 'localWhisper' && (
                    <SettingRow label={t('transcriptLive')} description={t('transcriptLiveDesc')}>
                        <p className="w-80 text-sm">
                            {parakeetReady ? t('transcriptLiveParakeet') : t('transcriptLiveWhisper')}
                        </p>
                    </SettingRow>
                )}
                <SettingRow
                    label={uiProvider === 'localWhisper' ? t('transcriptFinal') : t('transcriptModel')}
                    description={
                        uiProvider === 'localWhisper'
                            ? parakeetReady ? t('transcriptFinalDesc') : t('transcriptFinalSameDesc')
                            : uiProvider === 'parakeet' ? t('transcriptParakeetOnlyDesc') : undefined
                    }
                    htmlFor="transcript-provider"
                >
                    <Select
                        value={uiProvider}
                        onValueChange={(value) => {
                            const provider = value as TranscriptModelProps['provider'];
                            setUiProvider(provider);
                            if (provider !== 'localWhisper' && provider !== 'parakeet') {
                                fetchApiKey(provider);
                            }
                        }}
                    >
                        <SelectTrigger id="transcript-provider" className="w-80">
                            <SelectValue placeholder={t('selectProvider')} />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="parakeet">{t('providerParakeet')}</SelectItem>
                            <SelectItem value="localWhisper">{t('providerWhisper')}</SelectItem>
                        </SelectContent>
                    </Select>

                    {uiProvider !== 'localWhisper' && uiProvider !== 'parakeet' && (
                        <Select
                            value={transcriptModelConfig.model}
                            onValueChange={(value) => {
                                const model = value as TranscriptModelProps['model'];
                                setTranscriptModelConfig({ ...transcriptModelConfig, provider: uiProvider, model });
                            }}
                        >
                            <SelectTrigger className="w-56">
                                <SelectValue placeholder={t('selectModel')} />
                            </SelectTrigger>
                            <SelectContent>
                                {modelOptions[uiProvider].map((model) => (
                                    <SelectItem key={model} value={model}>{model}</SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    )}
                </SettingRow>

                {requiresApiKey && (
                    <SettingRow label={t('apiKey')} htmlFor="transcript-api-key">
                        <div className="relative w-80">
                            <Input
                                id="transcript-api-key"
                                type={showApiKey ? "text" : "password"}
                                className={`pr-20 ${isApiKeyLocked ? 'cursor-not-allowed bg-muted' : ''}`}
                                value={apiKey || ''}
                                onChange={(e) => setApiKey(e.target.value)}
                                disabled={isApiKeyLocked}
                                onClick={handleInputClick}
                                placeholder={t('apiKeyPlaceholder')}
                            />
                            {isApiKeyLocked && (
                                <div
                                    onClick={handleInputClick}
                                    className="absolute inset-0 cursor-not-allowed rounded-md"
                                />
                            )}
                            <div className="absolute inset-y-0 right-0 flex items-center pr-1">
                                <Button
                                    type="button"
                                    variant="ghost"
                                    size="icon"
                                    onClick={() => setIsApiKeyLocked(!isApiKeyLocked)}
                                    className={isLockButtonVibrating ? 'animate-vibrate text-destructive' : ''}
                                    title={isApiKeyLocked ? t('apiKeyUnlock') : t('apiKeyLock')}
                                    aria-label={isApiKeyLocked ? t('apiKeyUnlock') : t('apiKeyLock')}
                                >
                                    {isApiKeyLocked ? <Lock /> : <Unlock />}
                                </Button>
                                <Button
                                    type="button"
                                    variant="ghost"
                                    size="icon"
                                    onClick={() => setShowApiKey(!showApiKey)}
                                    title={showApiKey ? t('hideApiKey') : t('showApiKey')}
                                    aria-label={showApiKey ? t('hideApiKey') : t('showApiKey')}
                                >
                                    {showApiKey ? <EyeOff /> : <Eye />}
                                </Button>
                            </div>
                        </div>
                    </SettingRow>
                )}
            </SettingsSection>

            {uiProvider === 'localWhisper' && (
                <ModelManager
                    selectedModel={transcriptModelConfig.provider === 'localWhisper' ? transcriptModelConfig.model : undefined}
                    onModelSelect={handleWhisperModelSelect}
                    autoSave={true}
                />
            )}

            {uiProvider === 'parakeet' && (
                <ParakeetModelManager
                    selectedModel={transcriptModelConfig.provider === 'parakeet' ? transcriptModelConfig.model : undefined}
                    onModelSelect={handleParakeetModelSelect}
                    autoSave={true}
                />
            )}
        </div>
    )
}
