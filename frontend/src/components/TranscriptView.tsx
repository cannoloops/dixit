'use client';

import { useMemo } from 'react';
import { Transcript } from '@/types';
import { useConfig } from '@/contexts/ConfigContext';
import { VirtualizedTranscriptView } from './VirtualizedTranscriptView';

interface TranscriptViewProps {
  transcripts: Transcript[];
  isRecording?: boolean;
  isPaused?: boolean; // Is recording paused (affects UI indicators)
  isProcessing?: boolean; // Is processing/finalizing transcription
  isStopping?: boolean; // Is recording being stopped
  enableStreaming?: boolean; // Enable streaming effect for live transcription UX
  roster?: string[]; // Distinct speakers in order of appearance (derived from transcripts when omitted)
}

/** Non-paginated transcript: same epigraph layout as VirtualizedTranscriptView, fed with raw transcripts. */
export const TranscriptView: React.FC<TranscriptViewProps> = ({
  transcripts,
  isRecording = false,
  isPaused = false,
  isProcessing = false,
  isStopping = false,
  enableStreaming = false,
  roster,
}) => {
  const { showConfidenceIndicator } = useConfig();

  const segments = useMemo(
    () =>
      transcripts.map((t) => ({
        id: t.id,
        timestamp: t.audio_start_time ?? 0,
        endTime: t.audio_end_time,
        text: t.text,
        confidence: t.confidence,
        speaker: t.speaker,
      })),
    [transcripts]
  );

  return (
    <VirtualizedTranscriptView
      segments={segments}
      roster={roster}
      isRecording={isRecording}
      isPaused={isPaused}
      isProcessing={isProcessing}
      isStopping={isStopping}
      enableStreaming={enableStreaming}
      showConfidence={showConfidenceIndicator}
    />
  );
};
