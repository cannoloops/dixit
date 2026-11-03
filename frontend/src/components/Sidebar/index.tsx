'use client';

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { Settings, Trash2, Mic, Pencil, Search, X, Upload, Archive, PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import { useRouter, usePathname } from 'next/navigation';
import { useSidebar } from './SidebarProvider';
import type { CurrentMeeting } from '@/components/Sidebar/SidebarProvider';
import { ConfirmationModal } from '../ConfirmationModel/confirmation-modal';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { TranscriptModelProps } from '@/components/TranscriptSettings';
import Analytics from '@/lib/analytics';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { toast } from 'sonner';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useImportDialog } from '@/contexts/ImportDialogContext';
import { useConfig } from '@/contexts/ConfigContext';
import { LiveIndicator, StateStamp } from '@/components/dixit';
import { cn, meetingDisplayTitle } from '@/lib/utils';
import { locale } from '@/i18n';
import { t } from '@/i18n/shell';
import { t as tApp } from '@/i18n/app';

import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

import Logo from '../Logo';
import Info from '../Info';

/** Compact meeting length for the archive: "14 min", "1h19". */
function formatDuration(seconds: number): string {
  const minutes = Math.max(1, Math.round(seconds / 60));
  return minutes < 60 ? `${minutes} min` : `${Math.floor(minutes / 60)}h${String(minutes % 60).padStart(2, '0')}`;
}

// Sidebar controls.
const railItem =
  'flex h-8 w-full items-center gap-2 rounded-sm px-2 text-sm text-rail-foreground transition-colors duration-150 hover:bg-rail-active focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-rail-accent [&_svg]:h-4 [&_svg]:w-4 [&_svg]:shrink-0 [&_svg]:text-rail-muted';
const railIcon =
  'flex h-8 w-8 items-center justify-center rounded-sm text-rail-muted transition-colors duration-150 hover:bg-rail-active hover:text-rail-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-rail-accent disabled:pointer-events-none disabled:opacity-50 [&_svg]:h-4 [&_svg]:w-4';

const isSameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();

/** Day heading of the dated archive: "Oggi", "Ieri", or "Mer 22 ott" (with the year when not the current one). */
function dayLabel(date: Date): string {
  const now = new Date();
  if (isSameDay(date, now)) return t('today');
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (isSameDay(date, yesterday)) return t('yesterday');
  const label = date.toLocaleDateString(locale, {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    year: date.getFullYear() === now.getFullYear() ? undefined : 'numeric',
  });
  return label.charAt(0).toUpperCase() + label.slice(1);
}

const Sidebar: React.FC = () => {
  const router = useRouter();
  const pathname = usePathname();
  const {
    currentMeeting,
    setCurrentMeeting,
    isCollapsed,
    toggleCollapse,
    handleRecordingToggle,
    searchTranscripts,
    searchResults,
    isSearching,
    meetings,
    setMeetings,
    serverAddress
  } = useSidebar();

  // Get recording state from RecordingStateContext (single source of truth)
  const { isRecording, isStopping, isProcessing, isSaving } = useRecordingState();
  const { meetingTitle } = useTranscripts();
  const { openImportDialog } = useImportDialog();
  const { betaFeatures } = useConfig();
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [showModelSettings, setShowModelSettings] = useState(false);
  const [modelConfig, setModelConfig] = useState<ModelConfig>({
    provider: 'ollama',
    model: '',
    whisperModel: '',
    apiKey: null,
    ollamaEndpoint: null
  });
  const [transcriptModelConfig, setTranscriptModelConfig] = useState<TranscriptModelProps>({
    provider: 'parakeet',
    model: 'parakeet-tdt-0.6b-v3-int8',
  });
  const [settingsSaveSuccess, setSettingsSaveSuccess] = useState<boolean | null>(null);

  // State for edit modal
  const [editModalState, setEditModalState] = useState<{ isOpen: boolean; meetingId: string | null; currentTitle: string }>({
    isOpen: false,
    meetingId: null,
    currentTitle: ''
  });
  const [editingTitle, setEditingTitle] = useState<string>('');
  const [appVersion, setAppVersion] = useState<string>('');

  useEffect(() => {
    getVersion().then(setAppVersion).catch(console.error);
  }, []);

  // Meeting dates for the dated archive (created_at = when the meeting was saved).
  const meetingDates = useMemo(
    () => Object.fromEntries(meetings.map((m) => [m.id, m.created_at ?? ''])),
    [meetings]
  );

  const [deleteModalState, setDeleteModalState] = useState<{ isOpen: boolean; itemId: string | null }>({ isOpen: false, itemId: null });

  useEffect(() => {
    // Note: Don't set hardcoded defaults - let DB be the source of truth
    const fetchModelConfig = async () => {
      // Only make API call if serverAddress is loaded
      if (!serverAddress) {
        console.log('Waiting for server address to load before fetching model config');
        return;
      }

      try {
        const data = await invoke('api_get_model_config') as any;
        if (data && data.provider !== null) {
          // Fetch API key if not included and provider requires it
          if (data.provider !== 'ollama' && !data.apiKey) {
            try {
              const apiKeyData = await invoke('api_get_api_key', {
                provider: data.provider
              }) as string;
              data.apiKey = apiKeyData;
            } catch (err) {
              console.error('Failed to fetch API key:', err);
            }
          }
          setModelConfig(data);
        }
      } catch (error) {
        console.error('Failed to fetch model config:', error);
      }
    };

    fetchModelConfig();
  }, [serverAddress]);


  useEffect(() => {
    // Note: Don't set hardcoded defaults - let DB be the source of truth
    const fetchTranscriptSettings = async () => {
      // Only make API call if serverAddress is loaded
      if (!serverAddress) {
        console.log('Waiting for server address to load before fetching transcript settings');
        return;
      }

      try {
        const data = await invoke('api_get_transcript_config') as any;
        if (data && data.provider !== null) {
          setTranscriptModelConfig(data);
        }
      } catch (error) {
        console.error('Failed to fetch transcript settings:', error);
      }
    };
    fetchTranscriptSettings();
  }, [serverAddress]);

  // Listen for model config updates from other components
  useEffect(() => {
    const setupListener = async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<ModelConfig>('model-config-updated', (event) => {
        console.log('Sidebar received model-config-updated event:', event.payload);
        setModelConfig(event.payload);
      });

      return unlisten;
    };

    let cleanup: (() => void) | undefined;
    setupListener().then(fn => cleanup = fn);

    return () => {
      cleanup?.();
    };
  }, []);



  // Handle model config save
  const handleSaveModelConfig = async (config: ModelConfig) => {
    try {
      await invoke('api_save_model_config', {
        provider: config.provider,
        model: config.model,
        whisperModel: config.whisperModel,
        apiKey: config.apiKey,
        ollamaEndpoint: config.ollamaEndpoint,
      });

      setModelConfig(config);
      console.log('Model config saved successfully');
      setSettingsSaveSuccess(true);

      // Emit event to sync other components
      const { emit } = await import('@tauri-apps/api/event');
      await emit('model-config-updated', config);

      // Track settings change
      await Analytics.trackSettingsChanged('model_config', `${config.provider}_${config.model}`);
    } catch (error) {
      console.error('Error saving model config:', error);
      setSettingsSaveSuccess(false);
    }
  };

  const handleSaveTranscriptConfig = async (updatedConfig?: TranscriptModelProps) => {
    try {
      const configToSave = updatedConfig || transcriptModelConfig;
      const payload = {
        provider: configToSave.provider,
        model: configToSave.model,
        apiKey: configToSave.apiKey ?? null
      };
      console.log('Saving transcript config with payload:', payload);

      await invoke('api_save_transcript_config', {
        provider: payload.provider,
        model: payload.model,
        apiKey: payload.apiKey,
      });


      setSettingsSaveSuccess(true);

      // Track settings change
      const transcriptConfigToSave = updatedConfig || transcriptModelConfig;
      await Analytics.trackSettingsChanged('transcript_config', `${transcriptConfigToSave.provider}_${transcriptConfigToSave.model}`);
    } catch (error) {
      console.error('Failed to save transcript config:', error);
      setSettingsSaveSuccess(false);
    }
  };

  // Handle search input changes
  const handleSearchChange = useCallback(async (value: string) => {
    setSearchQuery(value);

    // If search query is empty, just return to normal view
    if (!value.trim()) return;

    // Search through transcripts
    await searchTranscripts(value);
  }, [searchTranscripts]);

  // Meetings shown in the archive: all of them, or those matching the title or the transcript search.
  const query = searchQuery.trim().toLowerCase();
  const matchedMeetingIds = new Set(searchResults.map(result => result.id));
  const visibleMeetings = query
    ? meetings.filter(m => matchedMeetingIds.has(m.id) || m.title.toLowerCase().includes(query))
    : meetings;


  const handleDelete = async (itemId: string) => {
    console.log('Deleting item:', itemId);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('api_delete_meeting', {
        meetingId: itemId,
      });
      console.log('Meeting deleted successfully');
      const updatedMeetings = meetings.filter((m: CurrentMeeting) => m.id !== itemId);
      setMeetings(updatedMeetings);

      // Track meeting deletion
      Analytics.trackMeetingDeleted(itemId);

      // Show success toast
      toast.success(t('meetingDeleted'), {
        description: t('meetingDeletedDesc')
      });

      // If deleting the active meeting, navigate to home
      if (currentMeeting?.id === itemId) {
        setCurrentMeeting({ id: 'intro-call', title: '+ New Call' });
        router.push('/');
      }
    } catch (error) {
      console.error('Failed to delete meeting:', error);
      toast.error(t('deleteFailed'), {
        description: error instanceof Error ? error.message : String(error)
      });
    }
  };

  const handleDeleteConfirm = () => {
    if (deleteModalState.itemId) {
      handleDelete(deleteModalState.itemId);
    }
    setDeleteModalState({ isOpen: false, itemId: null });
  };

  // Handle modal editing of meeting names
  const handleEditStart = (meetingId: string, currentTitle: string) => {
    setEditModalState({
      isOpen: true,
      meetingId: meetingId,
      currentTitle: currentTitle
    });
    setEditingTitle(currentTitle);
  };

  const handleEditConfirm = async () => {
    const newTitle = editingTitle.trim();
    const meetingId = editModalState.meetingId;

    if (!meetingId) return;

    // Prevent empty titles
    if (!newTitle) {
      toast.error(t('titleEmpty'));
      return;
    }

    try {
      await invoke('api_save_meeting_title', {
        meetingId: meetingId,
        title: newTitle,
      });

      // Update local state
      const updatedMeetings = meetings.map((m: CurrentMeeting) =>
        m.id === meetingId ? { ...m, title: newTitle } : m
      );
      setMeetings(updatedMeetings);

      // Update current meeting if it's the one being edited
      if (currentMeeting?.id === meetingId) {
        setCurrentMeeting({ id: meetingId, title: newTitle });
      }

      // Track the edit
      Analytics.trackButtonClick('edit_meeting_title', 'sidebar');

      toast.success(t('titleUpdated'));

      // Close modal and reset state
      setEditModalState({ isOpen: false, meetingId: null, currentTitle: '' });
      setEditingTitle('');
    } catch (error) {
      console.error('Failed to update meeting title:', error);
      toast.error(t('titleUpdateFailed'), {
        description: error instanceof Error ? error.message : String(error)
      });
    }
  };

  const handleEditCancel = () => {
    setEditModalState({ isOpen: false, meetingId: null, currentTitle: '' });
    setEditingTitle('');
  };

  // Expose setShowModelSettings to window for Rust tray to call
  useEffect(() => {
    (window as any).openSettings = () => {
      setShowModelSettings(true);
    };

    // Cleanup on unmount
    return () => {
      delete (window as any).openSettings;
    };
  }, []);

  const isSettingsPage = pathname === '/settings';
  const isMeetingPage = pathname?.includes('/meeting-details');

  // The meeting being recorded is not in the archive until it is saved: it gets its own row on top.
  const isFinishing = isStopping || isProcessing || isSaving;
  const liveTitle = meetingTitle && meetingTitle !== '+ New Call' ? meetingTitle : t('newRecording');

  const openMeeting = (meeting: CurrentMeeting) => {
    setCurrentMeeting({ id: meeting.id, title: meeting.title });
    router.push(`/meeting-details?id=${meeting.id}`);
  };

  const renderCollapsed = () => (
    <div className="flex h-full flex-col items-center gap-1 py-3">
      <Logo isCollapsed />
      <Tooltip>
        <TooltipTrigger asChild>
          <button type="button" onClick={toggleCollapse} className={railIcon} aria-label={t('expandSidebar')}>
            <PanelLeftOpen />
          </button>
        </TooltipTrigger>
        <TooltipContent side="right">{t('expandSidebar')}</TooltipContent>
      </Tooltip>

      <div className="my-2 h-px w-8 bg-rail-border" />

      <Tooltip>
        <TooltipTrigger asChild>
          {/* span keeps the tooltip working while the button is disabled */}
          <span>
            <Button
              variant="ghost"
              size="icon"
              className="text-rail-accent hover:bg-rail-active hover:text-rail-accent"
              onClick={handleRecordingToggle}
              disabled={isRecording}
              aria-label={isRecording ? t('recordingInProgress') : t('startRecording')}
            >
              <Mic />
            </Button>
          </span>
        </TooltipTrigger>
        <TooltipContent side="right">{isRecording ? t('recordingInProgress') : t('startRecording')}</TooltipContent>
      </Tooltip>

      {betaFeatures.importAndRetranscribe && (
        <Tooltip>
          <TooltipTrigger asChild>
            <button type="button" onClick={() => openImportDialog()} className={railIcon} aria-label={t('importAudio')}>
              <Upload />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{t('importAudio')}</TooltipContent>
        </Tooltip>
      )}

      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            onClick={toggleCollapse}
            className={cn(railIcon, isMeetingPage && 'bg-rail-active text-rail-foreground')}
            aria-label={t('meetings')}
          >
            <Archive />
          </button>
        </TooltipTrigger>
        <TooltipContent side="right">{t('meetings')}</TooltipContent>
      </Tooltip>

      <div className="mt-auto flex flex-col items-center gap-1">
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              type="button"
              onClick={() => router.push('/settings')}
              className={cn(railIcon, isSettingsPage && 'bg-rail-active text-rail-foreground')}
              aria-label={t('settings')}
            >
              <Settings />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{t('settings')}</TooltipContent>
        </Tooltip>
        <Info isCollapsed />
      </div>
    </div>
  );

  const renderMeetingRow = (meeting: CurrentMeeting, iso: string | undefined) => {
    const isActive = currentMeeting?.id === meeting.id;
    const snippet = query ? searchResults.find(result => result.id === meeting.id)?.matchContext : undefined;
    const time = iso
      ? new Date(iso).toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })
      : '';

    return (
      <div key={meeting.id} className="group relative">
        <button
          type="button"
          onClick={() => openMeeting(meeting)}
          aria-current={isActive ? 'page' : undefined}
          className={cn(
            'flex h-7 w-full items-center gap-2 rounded-sm px-2 text-left text-sm text-rail-foreground transition-colors duration-150 group-hover:pr-16 group-has-focus-visible:pr-16 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-rail-accent',
            isActive ? 'bg-rail-active font-medium' : 'hover:bg-rail-active'
          )}
        >
          <span className="w-10 shrink-0 text-xs tabular-nums text-rail-muted">{time}</span>
          <span className="min-w-0 flex-1 truncate" title={meeting.title}>{meetingDisplayTitle(meeting.title)}</span>
          {meeting.unnamed_speakers && (
            <span
              className="h-1.5 w-1.5 shrink-0 rounded-full bg-rail-accent"
              title={tApp('verifyNames')}
              aria-label={tApp('verifyNames')}
            />
          )}
          {meeting.duration_secs ? (
            <span className="shrink-0 text-xs tabular-nums text-rail-muted group-hover:hidden group-has-focus-visible:hidden">
              {formatDuration(meeting.duration_secs)}
            </span>
          ) : null}
        </button>
        {snippet && (
          <p className="line-clamp-2 pb-1 pl-14 pr-2 text-xs text-rail-muted">{snippet}</p>
        )}
        <div className="absolute right-1 top-0.5 hidden items-center group-hover:flex group-has-focus-visible:flex">
          <button
            type="button"
            onClick={() => handleEditStart(meeting.id, meeting.title)}
            className={cn(railIcon, 'h-6 w-6')}
            aria-label={t('renameMeeting')}
            title={t('renameMeeting')}
          >
            <Pencil />
          </button>
          <button
            type="button"
            onClick={() => setDeleteModalState({ isOpen: true, itemId: meeting.id })}
            className={cn(railIcon, 'h-6 w-6')}
            aria-label={t('deleteMeeting')}
            title={t('deleteMeeting')}
          >
            <Trash2 />
          </button>
        </div>
      </div>
    );
  };

  const renderArchive = () => {
    let lastDay: string | null = null;
    return visibleMeetings.map((meeting) => {
      const iso = meetingDates[meeting.id] || undefined;
      const date = iso ? new Date(iso) : null;
      const day = date ? date.toDateString() : null;
      const heading = date && day !== lastDay ? (
        <h3 key={`${meeting.id}-day`} className="px-2 pb-1 pt-3 text-xs font-medium text-rail-muted first:pt-1">
          {dayLabel(date)}
        </h3>
      ) : null;
      if (day) lastDay = day;
      return (
        <React.Fragment key={meeting.id}>
          {heading}
          {renderMeetingRow(meeting, iso)}
        </React.Fragment>
      );
    });
  };

  const renderExpanded = () => (
    <>
      {/* Mark and collapse */}
      <div className="flex h-12 shrink-0 items-center justify-between px-2">
        <Logo isCollapsed={false} />
        <Tooltip>
          <TooltipTrigger asChild>
            <button type="button" onClick={toggleCollapse} className={railIcon} aria-label={t('collapseSidebar')}>
              <PanelLeftClose />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{t('collapseSidebar')}</TooltipContent>
        </Tooltip>
      </div>

      {/* Search */}
      <div className="relative shrink-0 px-2 pb-2">
        <Search className="pointer-events-none absolute left-4 top-1.5 h-4 w-4 text-rail-muted" />
        <Input
          value={searchQuery}
          onChange={(e) => handleSearchChange(e.target.value)}
          placeholder={t('searchPlaceholder')}
          aria-label={t('searchPlaceholder')}
          className="h-7 border-rail-border bg-rail-active pl-7 pr-7 text-sm text-rail-foreground placeholder:text-rail-muted focus-visible:ring-rail-accent"
        />
        {searchQuery && (
          <button
            type="button"
            onClick={() => handleSearchChange('')}
            className={cn(railIcon, 'absolute right-3 top-0.5 h-6 w-6')}
            aria-label={t('clearSearch')}
          >
            <X />
          </button>
        )}
      </div>

      {/* Archive */}
      <div className="flex h-7 shrink-0 items-center justify-between border-t border-rail-border px-4 text-xs text-rail-muted">
        <span className="font-medium">{t('meetings')}</span>
        <span className="tabular-nums">
          {query && isSearching ? t('searching') : visibleMeetings.length}
        </span>
      </div>
      <nav aria-label={t('meetings')} className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
        {(isRecording || isFinishing) && (
          <button
            type="button"
            onClick={() => router.push('/')}
            aria-label={t('openLive')}
            className={cn(
              'mb-1 flex w-full flex-col items-start gap-1 rounded-sm px-2 py-1.5 text-left transition-colors duration-150 hover:bg-rail-active focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-rail-accent',
              pathname === '/' && 'bg-rail-active'
            )}
          >
            {isRecording ? (
              <LiveIndicator onRail className="text-rail-foreground" />
            ) : (
              <StateStamp state="processing" className="border-rail-border text-rail-muted" />
            )}
            <span className="w-full truncate text-sm text-rail-foreground">{liveTitle}</span>
          </button>
        )}

        {renderArchive()}

        {visibleMeetings.length === 0 && !(query && isSearching) && (
          <p className="px-2 py-3 text-xs text-rail-muted">
            {query ? t('noResults', { query: searchQuery.trim() }) : t('noMeetings')}
          </p>
        )}
      </nav>

      {/* Actions */}
      <div className="shrink-0 space-y-0.5 border-t border-rail-border p-2">
        <Button variant="outline" className="mb-1.5 w-full border-rail-accent/50 bg-transparent text-rail-accent hover:bg-rail-active hover:text-rail-accent" onClick={handleRecordingToggle} disabled={isRecording}>
          <Mic />
          {isRecording ? t('recordingInProgress') : t('startRecording')}
        </Button>

        {betaFeatures.importAndRetranscribe && (
          <button type="button" onClick={() => openImportDialog()} className={railItem}>
            <Upload />
            {t('importAudio')}
          </button>
        )}

        <button
          type="button"
          onClick={() => router.push('/settings')}
          aria-current={isSettingsPage ? 'page' : undefined}
          className={cn(railItem, isSettingsPage && 'bg-rail-active font-medium')}
        >
          <Settings />
          {t('settings')}
        </button>
        <Info isCollapsed={false} />
        {appVersion && (
          <p className="px-2 pt-1 text-xs tabular-nums text-rail-muted">v{appVersion}</p>
        )}
      </div>
    </>
  );

  return (
    <div className="fixed top-0 left-0 h-screen z-40">
      <aside
        className={cn(
          'flex h-screen flex-col border-r border-rail-border bg-rail text-rail-foreground transition-[width] duration-300',
          isCollapsed ? 'w-16' : 'w-64'
        )}
      >
        {isCollapsed ? renderCollapsed() : renderExpanded()}
      </aside>

      {/* Confirmation Modal for Delete */}
      <ConfirmationModal
        isOpen={deleteModalState.isOpen}
        text={t('deleteConfirm')}
        onConfirm={handleDeleteConfirm}
        onCancel={() => setDeleteModalState({ isOpen: false, itemId: null })}
      />

      {/* Edit Meeting Title Modal */}
      <Dialog open={editModalState.isOpen} onOpenChange={(open) => {
        if (!open) handleEditCancel();
      }}>
        <DialogContent className="sm:max-w-[425px]" aria-describedby={undefined}>
          <DialogHeader>
            <DialogTitle>{t('renameMeeting')}</DialogTitle>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="meeting-title">{t('meetingTitle')}</Label>
            <Input
              id="meeting-title"
              value={editingTitle}
              onChange={(e) => setEditingTitle(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleEditConfirm();
                }
              }}
              placeholder={t('meetingTitlePlaceholder')}
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={handleEditCancel}>
              {tApp('cancel')}
            </Button>
            <Button onClick={handleEditConfirm}>
              {tApp('save')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

export default Sidebar;
