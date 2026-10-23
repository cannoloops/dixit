'use client';

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import { Loader2, FolderOpen, Database, Check, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { t } from '@/i18n/models';
import { HomebrewDatabaseDetector } from './HomebrewDatabaseDetector';

interface LegacyDatabaseImportProps {
  isOpen: boolean;
  onComplete: () => void;
}

type ImportState = 'idle' | 'selecting' | 'detecting' | 'importing' | 'success' | 'error';

export function LegacyDatabaseImport({ isOpen, onComplete }: LegacyDatabaseImportProps) {
  const [importState, setImportState] = useState<ImportState>('idle');
  const [detectedPath, setDetectedPath] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string>('');

  const handleBrowse = async () => {
    try {
      setImportState('selecting');

      // Open file picker
      const selectedPath = await invoke<string | null>('select_legacy_database_path');

      if (!selectedPath) {
        setImportState('idle');
        return;
      }

      setImportState('detecting');

      // Detect database from selected path
      const dbPath = await invoke<string | null>('detect_legacy_database', {
        selectedPath,
      });

      if (dbPath) {
        setDetectedPath(dbPath);
        setImportState('idle');
      } else {
        setErrorMessage(t('importNotFound'));
        setDetectedPath(null);
        setImportState('error');
        setTimeout(() => setImportState('idle'), 3000);
      }
    } catch (error) {
      console.error('Error browsing for database:', error);
      setErrorMessage(String(error));
      setImportState('error');
      setTimeout(() => setImportState('idle'), 3000);
    }
  };

  const handleImport = async () => {
    if (!detectedPath) return;

    try {
      setImportState('importing');

      await invoke('import_and_initialize_database', {
        legacyDbPath: detectedPath,
      });

      setImportState('success');
      toast.success(t('importSuccessReloading'));

      // Wait 1 second for user to see success, then reload window to refresh all data
      setTimeout(() => {
        window.location.reload();
      }, 1000);
    } catch (error) {
      console.error('Error importing database:', error);
      setErrorMessage(String(error));
      setImportState('error');
      toast.error(t('importFailed', { error: String(error) }));
      setTimeout(() => setImportState('idle'), 3000);
    }
  };

  const handleStartFresh = async () => {
    try {
      setImportState('importing');

      await invoke('initialize_fresh_database');

      setImportState('success');
      toast.success(t('initSuccessStarting'));

      // Wait 1 second for user to see success, then reload window to start fresh
      setTimeout(() => {
        window.location.reload();
      }, 1000);
    } catch (error) {
      console.error('Error initializing database:', error);
      setErrorMessage(String(error));
      setImportState('error');
      toast.error(t('initFailed', { error: String(error) }));
      setTimeout(() => setImportState('idle'), 3000);
    }
  };

  const isLoading = ['selecting', 'detecting', 'importing'].includes(importState);
  const canImport = detectedPath && importState === 'idle';

  const handleHomebrewImportSuccess = () => {
    // The HomebrewDatabaseDetector handles the reload itself
    onComplete();
  };

  const handleHomebrewDecline = () => {
    // User declined homebrew import, they can continue with manual browse
  };

  return (
    <Dialog open={isOpen} onOpenChange={() => {}}>
      <DialogContent className="sm:max-w-[600px]" onPointerDownOutside={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>{t('welcomeTitle')}</DialogTitle>
          <DialogDescription>{t('welcomeQuestion')}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Homebrew Database Auto-Detection */}
          <HomebrewDatabaseDetector
            onImportSuccess={handleHomebrewImportSuccess}
            onDecline={handleHomebrewDecline}
          />

          {/* Browse Section */}
          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">{t('browseHint')}</p>

            <Button variant="outline" onClick={handleBrowse} disabled={isLoading} className="w-full">
              {importState === 'selecting' || importState === 'detecting' ? (
                <>
                  <Loader2 className="animate-spin" />
                  {importState === 'selecting' ? t('selecting') : t('detecting')}
                </>
              ) : (
                <>
                  <FolderOpen />
                  {t('browse')}
                </>
              )}
            </Button>
          </div>

          {/* Detection Result */}
          {detectedPath && (
            <div className="flex items-start gap-2">
              <Check className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium text-foreground">{t('databaseFound')}</p>
                <p className="break-all font-mono text-xs text-muted-foreground">{detectedPath}</p>
              </div>
            </div>
          )}

          {/* Error Message */}
          {importState === 'error' && errorMessage && (
            <div role="alert" className="flex items-start gap-2">
              <X className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
              <p className="text-sm text-destructive">{errorMessage}</p>
            </div>
          )}

          {/* Action Buttons */}
          <div className="flex flex-col gap-3 border-t border-border pt-4">
            <Button onClick={handleImport} disabled={!canImport || isLoading} className="w-full">
              {importState === 'importing' ? (
                <>
                  <Loader2 className="animate-spin" />
                  {t('importing')}
                </>
              ) : importState === 'success' ? (
                <>
                  <Check />
                  {t('importDone')}
                </>
              ) : (
                <>
                  <Database />
                  {t('importDatabase')}
                </>
              )}
            </Button>

            <Button variant="outline" onClick={handleStartFresh} disabled={isLoading} className="w-full">
              {t('startFresh')}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
