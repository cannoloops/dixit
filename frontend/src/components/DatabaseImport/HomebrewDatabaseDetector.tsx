'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Loader2, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { locale } from '@/i18n';
import { t } from '@/i18n/models';

interface HomebrewDatabaseDetectorProps {
  onImportSuccess: () => void;
  onDecline: () => void;
}

// Homebrew paths differ between Intel and Apple Silicon Macs
const HOMEBREW_PATHS = [
  '/opt/homebrew/var/meetily/meeting_minutes.db',  // Apple Silicon (M1/M2/M3)
  '/usr/local/var/meetily/meeting_minutes.db',      // Intel Macs
];

export function HomebrewDatabaseDetector({ onImportSuccess, onDecline }: HomebrewDatabaseDetectorProps) {
  const [isChecking, setIsChecking] = useState(true);
  const [isImporting, setIsImporting] = useState(false);
  const [homebrewDbExists, setHomebrewDbExists] = useState(false);
  const [dbSize, setDbSize] = useState<number>(0);
  const [detectedPath, setDetectedPath] = useState<string>('');
  const [isDismissed, setIsDismissed] = useState(false);

  useEffect(() => {
    checkHomebrewDatabase();
  }, []);

  const checkHomebrewDatabase = async () => {
    try {
      setIsChecking(true);

      // Check all possible Homebrew locations
      for (const path of HOMEBREW_PATHS) {
        const result = await invoke<{ exists: boolean; size: number } | null>('check_homebrew_database', {
          path,
        });

        if (result && result.exists && result.size > 0) {
          setHomebrewDbExists(true);
          setDbSize(result.size);
          setDetectedPath(path);
          break; // Stop checking once we find a valid database
        }
      }
    } catch (error) {
      console.error('Error checking homebrew database:', error);
      // Silently fail - this is just auto-detection
    } finally {
      setIsChecking(false);
    }
  };

  const handleYes = async () => {
    try {
      setIsImporting(true);

      await invoke('import_and_initialize_database', {
        legacyDbPath: detectedPath,
      });

      toast.success(t('importSuccessReloading'));

      // Wait 1 second for user to see success, then reload window to refresh all data
      setTimeout(() => {
        window.location.reload();
      }, 1000);
    } catch (error) {
      console.error('Error importing database:', error);
      toast.error(t('importFailed', { error: String(error) }));
      setIsImporting(false);
    }
  };

  const handleNo = () => {
    setIsDismissed(true);
    onDecline();
  };

  if (isChecking || !homebrewDbExists || isDismissed) {
    return null;
  }

  const formatFileSize = (bytes: number): string => {
    const fmt = (n: number) => n.toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${fmt(bytes / 1024)} KB`;
    return `${fmt(bytes / (1024 * 1024))} MB`;
  };

  return (
    <section className="space-y-3 border-y border-border py-4">
      <div className="space-y-1">
        <h3 className="text-sm font-semibold text-foreground">{t('homebrewTitle')}</h3>
        <p className="text-sm text-muted-foreground">{t('homebrewBody')}</p>
      </div>

      <p className="break-all font-mono text-xs text-muted-foreground">
        {detectedPath}
        <span className="ml-2 tabular-nums">{formatFileSize(dbSize)}</span>
      </p>

      <p className="text-sm text-foreground">{t('homebrewQuestion')}</p>

      <div className="flex gap-2">
        <Button onClick={handleYes} disabled={isImporting} className="flex-1">
          {isImporting ? (
            <>
              <Loader2 className="animate-spin" />
              {t('importing')}
            </>
          ) : (
            <>
              <Check />
              {t('homebrewYes')}
            </>
          )}
        </Button>

        <Button variant="outline" onClick={handleNo} disabled={isImporting} className="flex-1">
          {t('homebrewNo')}
        </Button>
      </div>
    </section>
  );
}
