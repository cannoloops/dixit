import React from 'react';
import { Upload } from 'lucide-react';
import { getAudioFormatsDisplayList } from '@/constants/audioFormats';
import { t } from '@/i18n/recording';

interface ImportDropOverlayProps {
  visible: boolean;
}

export function ImportDropOverlay({ visible }: ImportDropOverlayProps) {
  if (!visible) return null;

  return (
    <div className="pointer-events-none fixed inset-0 z-50 flex items-center justify-center bg-black/45">
      <div className="rounded-md border border-dashed border-primary bg-popover px-10 py-8 text-center text-popover-foreground shadow-[var(--shadow-float)]">
        <Upload className="mx-auto mb-3 h-4 w-4 text-primary" />
        <p className="text-base font-semibold">{t('dropToImport')}</p>
        <p className="mt-1 text-xs text-muted-foreground">{getAudioFormatsDisplayList()}</p>
      </div>
    </div>
  );
}
