"use client";
import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { MeetingSummary, SummaryProcessResponse } from '@/types';
import Analytics from '@/lib/analytics';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { TranscriptPanel } from '@/components/MeetingDetails/TranscriptPanel';
import { SummaryPanel } from '@/components/MeetingDetails/SummaryPanel';
import { MeetingDetailsSplitView, type MeetingDetailsTab } from '@/components/MeetingDetails/MeetingDetailsSplitView';
import { SpeakerBand } from '@/components/MeetingDetails/SpeakerBand';
import { GenerateSummaryButton } from '@/components/MeetingDetails/SummaryGeneratorButtonGroup';
import { EditableTitle } from '@/components/EditableTitle';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { hasVisibleSummaryContent } from '@/lib/summary-content';
import { locale } from '@/i18n';
import { t } from '@/i18n/meeting';

// Custom hooks
import { useMeetingData } from '@/hooks/meeting-details/useMeetingData';
import { useSummaryGeneration } from '@/hooks/meeting-details/useSummaryGeneration';
import { useTemplates } from '@/hooks/meeting-details/useTemplates';
import { MeetingTypePicker } from '@/components/MeetingDetails/MeetingTypePicker';
import { useCopyOperations } from '@/hooks/meeting-details/useCopyOperations';
import { useMeetingNotes } from '@/hooks/meeting-details/useMeetingNotes';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';
import { useMeetingTimeline, rosterOf } from '@/hooks/meeting-details/useMeetingTimeline';
import { useConfig } from '@/contexts/ConfigContext';

/** "23 ottobre 2026 · 14:30 · 1 h 05 min" (duration only when the transcript carries times). */
function meetingDateLine(createdAt: string, durationSeconds: number): string {
  const date = new Date(createdAt);
  const parts = Number.isNaN(date.getTime())
    ? []
    : [
        date.toLocaleDateString(locale, { day: 'numeric', month: 'long', year: 'numeric' }),
        date.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' }),
      ];
  if (durationSeconds > 0) {
    const minutes = Math.max(1, Math.round(durationSeconds / 60));
    parts.push(
      minutes >= 60
        ? t('durationHours', { h: Math.floor(minutes / 60), m: String(minutes % 60).padStart(2, '0') })
        : t('durationMinutes', { m: minutes })
    );
  }
  return parts.join(' · ');
}

export default function PageContent({
  meeting,
  summaryData,
  initialSummary,
  onMeetingUpdated,
  onRefetchTranscripts,
  // Pagination props for efficient transcript loading
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
}: {
  meeting: any;
  summaryData: MeetingSummary | null;
  initialSummary: SummaryProcessResponse | null;
  onMeetingUpdated?: () => Promise<void>;
  onRefetchTranscripts?: () => Promise<void>;
  // Pagination props
  segments?: any[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;
}) {
  console.log('📄 PAGE CONTENT: Initializing with data:', {
    meetingId: meeting.id,
    summaryDataKeys: summaryData ? Object.keys(summaryData) : null,
    transcriptsCount: meeting.transcripts?.length
  });

  // State
  const [customPrompt, setCustomPrompt] = useState<string>('');
  const isRecording = false;
  const [activeTab, setActiveTab] = useState<MeetingDetailsTab>('transcript');
  const [isEditingTitle, setIsEditingTitle] = useState(false);
  const titleBeforeEdit = useRef<string | null>(null);
  const [jumpTo, setJumpTo] = useState<{ id: string; nonce: number } | null>(null);

  // Ref to store the modal open function from SummaryGeneratorButtonGroup
  const openModelSettingsRef = useRef<(() => void) | null>(null);
  const autoSwitchedSummaryMeetingIdsRef = useRef(new Set<string>());
  const manuallySelectedTabMeetingIdsRef = useRef(new Set<string>());

  // Get model config from ConfigContext
  const { modelConfig, setModelConfig, isModelConfigLoading } = useConfig();

  // Custom hooks
  const meetingData = useMeetingData({ meeting, summaryData, onMeetingUpdated });
  const templates = useTemplates();

  // Callback to register the modal open function
  const handleRegisterModalOpen = (openFn: () => void) => {
    console.log('📝 Registering modal open function in PageContent');
    openModelSettingsRef.current = openFn;
  };

  // Callback to trigger modal open (called from error handler)
  const handleOpenModelSettings = () => {
    console.log('🔔 Opening model settings from PageContent');
    if (openModelSettingsRef.current) {
      openModelSettingsRef.current();
    } else {
      console.warn('⚠️ Modal open function not yet registered');
    }
  };

  // Save model config to backend database and sync via event
  const handleSaveModelConfig = async (config?: ModelConfig) => {
    if (!config) return;
    try {
      await invoke('api_save_model_config', {
        provider: config.provider,
        model: config.model,
        whisperModel: config.whisperModel,
        apiKey: config.apiKey ?? null,
        ollamaEndpoint: config.ollamaEndpoint ?? null,
      });

      // Emit event so ConfigContext and other listeners stay in sync
      const { emit } = await import('@tauri-apps/api/event');
      await emit('model-config-updated', config);

      toast.success(t('modelSaved'));
    } catch (error) {
      console.error('Failed to save model config:', error);
      toast.error(t('modelSaveFailed'), { description: String(error) });
    }
  };

  const summaryGeneration = useSummaryGeneration({
    initialSummary,
    meeting,
    transcripts: meetingData.transcripts,
    modelConfig: modelConfig,
    isModelConfigLoading,
    selectedTemplate: templates.selectedTemplate,
    onMeetingUpdated,
    updateMeetingTitle: meetingData.updateMeetingTitle,
    setAiSummary: meetingData.setAiSummary,
    onOpenModelSettings: handleOpenModelSettings,
  });

  const copyOperations = useCopyOperations({
    meeting,
    transcripts: meetingData.transcripts,
    meetingTitle: meetingData.meetingTitle,
    aiSummary: meetingData.aiSummary,
    blockNoteSummaryRef: meetingData.blockNoteSummaryRef,
  });

  const meetingNotes = useMeetingNotes(meeting.id);

  const meetingOperations = useMeetingOperations({
    meeting,
  });

  // All lines of the meeting (the transcript list is paginated): speaker band, roster, duration.
  const timeline = useMeetingTimeline(meeting.id);
  const roster = useMemo(
    () => rosterOf(timeline.lines?.length ? timeline.lines : segments ?? []),
    [timeline.lines, segments]
  );
  const refreshTimeline = timeline.refresh;

  // Renames, retranscription and late speaker attribution reload both the list and the band.
  const refreshTranscripts = useCallback(async () => {
    refreshTimeline();
    await onRefetchTranscripts?.();
  }, [refreshTimeline, onRefetchTranscripts]);

  // Attribution finishes after the meeting is saved, and Teams transcripts can arrive later.
  useEffect(() => {
    const unlisten = listen('speakers-updated', () => void refreshTranscripts());
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refreshTranscripts]);

  const handleSelectTurn = useCallback((lineId: string) => {
    manuallySelectedTabMeetingIdsRef.current.add(meeting.id);
    setActiveTab('transcript');
    setJumpTo({ id: lineId, nonce: Date.now() });
  }, [meeting.id]);

  const handleStartEditingTitle = () => {
    titleBeforeEdit.current = meetingData.meetingTitle;
    setIsEditingTitle(true);
  };

  // Enter and blur both end editing: only the first call saves.
  const handleFinishEditingTitle = async () => {
    const previous = titleBeforeEdit.current;
    if (previous === null) return;
    titleBeforeEdit.current = null;
    setIsEditingTitle(false);
    const next = meetingData.meetingTitle.trim();
    if (!next || next === previous) {
      meetingData.setMeetingTitle(previous);
      return;
    }
    try {
      await invoke('api_save_meeting_title', { meetingId: meeting.id, title: next });
      meetingData.updateMeetingTitle(next);
      Analytics.trackButtonClick('edit_meeting_title', 'meeting_details');
      await onMeetingUpdated?.();
    } catch (error) {
      console.error('Failed to save meeting title:', error);
      meetingData.setMeetingTitle(previous);
      toast.error(t('titleSaveFailed'), { description: String(error) });
    }
  };

  // Track page view
  useEffect(() => {
    Analytics.trackPageView('meeting_details');
  }, []);

  useEffect(() => {
    if (
      (meetingData.aiSummary || summaryGeneration.summaryStatus === 'completed')
      && !autoSwitchedSummaryMeetingIdsRef.current.has(meeting.id)
      && !manuallySelectedTabMeetingIdsRef.current.has(meeting.id)
    ) {
      autoSwitchedSummaryMeetingIdsRef.current.add(meeting.id);
      setActiveTab('summary');
    }
  }, [meeting.id, meetingData.aiSummary, summaryGeneration.summaryStatus]);

  return (
    <div className="flex h-screen min-w-0 flex-col bg-background">
      <header className="shrink-0 px-3 pb-3 pt-4">
        <div className="flex items-start gap-4">
          <div className="min-w-0 flex-1">
            <EditableTitle
              title={meetingData.meetingTitle}
              isEditing={isEditingTitle}
              onStartEditing={handleStartEditingTitle}
              onFinishEditing={handleFinishEditingTitle}
              onChange={meetingData.setMeetingTitle}
              className="font-display text-2xl font-bold leading-tight"
            />
            <p className="mt-1 px-1 text-xs tabular-nums text-muted-foreground">
              {meetingDateLine(meeting.created_at, timeline.duration)}
            </p>
          </div>
          <div className="shrink-0 pt-0.5">
            <GenerateSummaryButton
              summaryStatus={summaryGeneration.summaryStatus}
              hasSummary={hasVisibleSummaryContent(meetingData.aiSummary)}
              hasTranscripts={meetingData.transcripts.length > 0}
              isModelConfigLoading={isModelConfigLoading}
              onGenerate={() => void summaryGeneration.handleGenerateSummary(customPrompt)}
              onStop={summaryGeneration.handleStopGeneration}
            />
          </div>
        </div>
        <SpeakerBand
          className="mt-4 px-1"
          meetingId={meeting.id}
          lines={timeline.lines}
          duration={timeline.duration}
          roster={roster}
          onChanged={refreshTranscripts}
          onSelectTurn={handleSelectTurn}
        />
      </header>

      <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden border-t border-border">
        <MeetingDetailsSplitView
          activeTab={activeTab}
          onTabChange={(tab) => {
            manuallySelectedTabMeetingIdsRef.current.add(meeting.id);
            setActiveTab(tab);
          }}
          transcript={
            <TranscriptPanel
              transcripts={meetingData.transcripts}
              customPrompt={customPrompt}
              onPromptChange={setCustomPrompt}
              onCopyTranscript={copyOperations.handleCopyTranscript}
              onOpenMeetingFolder={meetingOperations.handleOpenMeetingFolder}
              isRecording={isRecording}
              disableAutoScroll={true}
              usePagination={true}
              segments={segments}
              hasMore={hasMore}
              isLoadingMore={isLoadingMore}
              totalCount={totalCount}
              loadedCount={loadedCount}
              onLoadMore={onLoadMore}
              meetingId={meeting.id}
              meetingFolderPath={meeting.folder_path}
              onRefetchTranscripts={refreshTranscripts}
              roster={roster}
              jumpTo={jumpTo}
              notes={meetingNotes.notes}
              onEditNote={meetingNotes.edit}
              onDeleteNote={meetingNotes.remove}
              includeNotes={copyOperations.includeNotes}
              onIncludeNotesChange={copyOperations.setIncludeNotes}
            />
          }
          summary={
            <SummaryPanel
              meeting={meeting}
              meetingTitle={meetingData.meetingTitle}
              summaryRef={meetingData.blockNoteSummaryRef}
              isSaving={meetingData.isSaving}
              isSummaryDirty={meetingData.isSummaryDirty}
              onSaveAll={meetingData.saveAllChanges}
              onCopySummary={copyOperations.handleCopySummary}
              aiSummary={meetingData.aiSummary}
              summaryStatus={summaryGeneration.summaryStatus}
              transcripts={meetingData.transcripts}
              modelConfig={modelConfig}
              setModelConfig={setModelConfig}
              onSaveModelConfig={handleSaveModelConfig}
              onSaveSummary={meetingData.handleSaveSummary}
              onSummaryChange={meetingData.handleSummaryChange}
              onDirtyChange={meetingData.setIsSummaryDirty}
              summaryError={summaryGeneration.summaryError}
              onRegenerateSummary={summaryGeneration.handleRegenerateSummary}
              getSummaryStatusMessage={summaryGeneration.getSummaryStatusMessage}
              availableTemplates={templates.availableTemplates}
              selectedTemplate={templates.selectedTemplate}
              onTemplateSelect={templates.handleTemplateSelection}
              isModelConfigLoading={isModelConfigLoading}
              onOpenModelSettings={handleRegisterModalOpen}
              onChooseModel={handleOpenModelSettings}
              typeSlot={
                <MeetingTypePicker
                  meetingId={meeting.id}
                  refreshKey={summaryGeneration.summaryStatus}
                  onChanged={() => {
                    templates.setSelectedTemplate('auto');
                    void summaryGeneration.handleGenerateSummary('', 'auto');
                  }}
                />
              }
            />
          }
        />
      </div>
    </div>
  );
}
