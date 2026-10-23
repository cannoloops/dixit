import { Loader2 } from 'lucide-react';
import { t } from '@/i18n/recording';

interface StatusOverlaysProps {
  // Status flags
  isProcessing: boolean;      // Processing transcription after recording stops
  isSaving: boolean;          // Saving transcript to database
}

// Floating status line at the bottom of the live view (in place of the recording controls).
// Positioned against the page's relative container, so it follows the sidebar on its own.
function StatusOverlay({ show, message }: { show: boolean; message: string }) {
  if (!show) return null;

  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-6 z-10 flex justify-center px-6">
      <div
        className="flex h-11 items-center gap-2 rounded-md border border-border bg-popover px-4 text-sm text-popover-foreground shadow-[var(--shadow-float)]"
        role="status"
      >
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        <span>{message}</span>
      </div>
    </div>
  );
}

// Main exported component - renders multiple status overlays
export function StatusOverlays({ isProcessing, isSaving }: StatusOverlaysProps) {
  return (
    <>
      {/* Shown after recording stops while finalizing transcription */}
      <StatusOverlay show={isProcessing} message={t('finalizing')} />

      {/* Shown while saving the meeting */}
      <StatusOverlay show={isSaving} message={t('savingMeeting')} />
    </>
  );
}
