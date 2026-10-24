import { useCallback, useState, RefObject } from 'react';
import { MeetingNote } from '@/lib/notes';
import { MeetingSummary, Transcript } from '@/types';
import { BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { toast } from 'sonner';
import Analytics from '@/lib/analytics';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { hasVisibleSummaryContent } from '@/lib/summary-content';
import { locale } from '@/i18n';
import { t } from '@/i18n/meeting';

const INCLUDE_NOTES_KEY = 'dixit.copy.includeNotes';

interface UseCopyOperationsProps {
  meeting: any;
  transcripts: Transcript[];
  meetingTitle: string;
  aiSummary: MeetingSummary | null;
  blockNoteSummaryRef: RefObject<BlockNoteSummaryViewRef>;
}

export function useCopyOperations({
  meeting,
  transcripts,
  meetingTitle,
  aiSummary,
  blockNoteSummaryRef,
}: UseCopyOperationsProps) {
  // "Include my notes": on by default, remembers the last choice.
  const [includeNotes, setIncludeNotesState] = useState<boolean>(() => {
    try {
      return localStorage.getItem(INCLUDE_NOTES_KEY) !== '0';
    } catch {
      return true;
    }
  });
  const setIncludeNotes = useCallback((value: boolean) => {
    setIncludeNotesState(value);
    try {
      localStorage.setItem(INCLUDE_NOTES_KEY, value ? '1' : '0');
    } catch {
      // The choice just isn't remembered.
    }
  }, []);

  // Helper function to fetch ALL transcripts for copying (not just paginated data)
  const fetchAllTranscripts = useCallback(async (meetingId: string): Promise<Transcript[]> => {
    try {
      console.log('📊 Fetching all transcripts for copying:', meetingId);

      // First, get total count by fetching first page
      const firstPage = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: 1,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      const totalCount = firstPage.total_count;
      console.log(`📊 Total transcripts in database: ${totalCount}`);

      if (totalCount === 0) {
        return [];
      }

      // Fetch all transcripts in one call
      const allData = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: totalCount,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      console.log(`✅ Fetched ${allData.transcripts.length} transcripts from database for copying`);
      return allData.transcripts;
    } catch (error) {
      console.error('❌ Error fetching all transcripts:', error);
      toast.error(t('copyFetchFailed'));
      return [];
    }
  }, []);

  // Copy transcript to clipboard
  const handleCopyTranscript = useCallback(async () => {
    // CHANGE: Fetch ALL transcripts from database, not from pagination state
    console.log('📊 Fetching all transcripts for copying...');
    const allTranscripts = await fetchAllTranscripts(meeting.id);

    if (!allTranscripts.length) {
      toast.error(t('nothingToCopy'));
      return;
    }

    console.log(`✅ Copying ${allTranscripts.length} transcripts to clipboard`);

    // Format timestamps as recording-relative [MM:SS] instead of wall-clock time
    const formatTime = (seconds: number | undefined, fallbackTimestamp: string): string => {
      if (seconds === undefined) {
        // For old transcripts without audio_start_time, use wall-clock time
        return fallbackTimestamp;
      }
      const totalSecs = Math.floor(seconds);
      const mins = Math.floor(totalSecs / 60);
      const secs = totalSecs % 60;
      return `[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
    };

    const header = `# ${t('mdTranscriptTitle', { title: meetingTitle ?? meeting.title })}\n\n`;
    const date = `## ${t('mdDate')}: ${new Date(meeting.created_at).toLocaleDateString(locale)}\n\n`;
    // Notes go between the lines, as `> Nota [mm:ss] text` (same as the .md in the meeting folder).
    const notes = includeNotes
      ? await invokeTauri<MeetingNote[]>('meeting_notes_list', { meetingId: meeting.id }).catch(() => [])
      : [];
    const rows: { at: number; line: string }[] = [
      ...allTranscripts.map(t => ({ at: t.audio_start_time ?? 0, line: `${formatTime(t.audio_start_time, t.timestamp)} ${t.text}  ` })),
      ...notes.map(n => ({
        at: n.offset_ms / 1000,
        line: `> ${t('noteLabel')} ${formatTime(n.offset_ms / 1000, '')} ${n.text.replace(/\s+/g, ' ')}  `,
      })),
    ];
    // Stable sort: a note goes after the lines that start at or before it.
    const fullTranscript = rows.sort((a, b) => a.at - b.at).map(r => r.line).join('\n');

    await navigator.clipboard.writeText(header + date + fullTranscript);
    toast.success(t('transcriptCopied'));

    // Track copy analytics
    const wordCount = allTranscripts
      .map(t => t.text.split(/\s+/).length)
      .reduce((a, b) => a + b, 0);

    await Analytics.trackCopy('transcript', {
      meeting_id: meeting.id,
      transcript_length: allTranscripts.length.toString(),
      word_count: wordCount.toString()
    });
  }, [meeting, meetingTitle, fetchAllTranscripts, includeNotes]);

  // Copy summary to clipboard
  const handleCopySummary = useCallback(async () => {
    if (!hasVisibleSummaryContent(aiSummary)) {
      toast.error(t('noSummaryToCopy'));
      return;
    }
    try {
      let summaryMarkdown = '';

      console.log('🔍 Copy Summary - Starting...');

      // Try to get markdown from BlockNote editor first
      if (blockNoteSummaryRef.current?.getMarkdown) {
        console.log('📝 Trying to get markdown from ref...');
        summaryMarkdown = await blockNoteSummaryRef.current.getMarkdown();
        console.log('📝 Got markdown from ref, length:', summaryMarkdown.length);
      }

      // Fallback: Check if aiSummary has markdown property
      if (!summaryMarkdown && aiSummary && typeof aiSummary.markdown === 'string') {
        console.log('📝 Using markdown from aiSummary');
        summaryMarkdown = aiSummary.markdown;
        console.log('📝 Markdown from aiSummary, length:', summaryMarkdown.length);
      }

      // Fallback: Check for legacy format
      if (!summaryMarkdown && aiSummary) {
        console.log('📝 Converting legacy format to markdown');
        const sections = Object.entries(aiSummary)
          .filter(([key]) => {
            // Skip non-section keys
            return key !== 'markdown' && key !== 'summary_json' && key !== '_section_order' && key !== 'MeetingName';
          })
          .map(([, section]) => {
            if (section && typeof section === 'object' && 'title' in section && 'blocks' in section) {
              const sectionTitle = `## ${section.title}\n\n`;
              const sectionContent = section.blocks
                .map((block: any) => `- ${block.content}`)
                .join('\n');
              return sectionTitle + sectionContent;
            }
            return '';
          })
          .filter(s => s.trim())
          .join('\n\n');
        summaryMarkdown = sections;
        console.log('📝 Converted legacy format, length:', summaryMarkdown.length);
      }

      // If still no summary content, show message
      if (!summaryMarkdown.trim()) {
        console.error('❌ No summary content available to copy');
        toast.error(t('noSummaryToCopy'));
        return;
      }

      // Build metadata header
      const dateFormat: Intl.DateTimeFormatOptions = {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      };
      const header = `# ${t('mdSummaryTitle', { title: meetingTitle })}\n\n`;
      const metadata = `**${t('mdMeetingId')}:** ${meeting.id}\n**${t('mdDate')}:** ${new Date(meeting.created_at).toLocaleDateString(locale, dateFormat)}\n**${t('mdCopiedOn')}:** ${new Date().toLocaleDateString(locale, dateFormat)}\n\n---\n\n`;

      const fullMarkdown = header + metadata + summaryMarkdown;
      await navigator.clipboard.writeText(fullMarkdown);

      console.log('✅ Successfully copied to clipboard!');
      toast.success(t('summaryCopied'));

      // Track copy analytics
      await Analytics.trackCopy('summary', {
        meeting_id: meeting.id,
        has_markdown: (!!aiSummary && 'markdown' in aiSummary).toString()
      });
    } catch (error) {
      console.error('❌ Failed to copy summary:', error);
      toast.error(t('copyFailed'));
    }
  }, [aiSummary, meetingTitle, meeting, blockNoteSummaryRef]);

  return {
    handleCopyTranscript,
    handleCopySummary,
    includeNotes,
    setIncludeNotes,
  };
}
