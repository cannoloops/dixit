import React, { useContext, useState, useEffect } from 'react';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { Info, Loader2, Copy, Check } from 'lucide-react';
import { AnalyticsContext } from './AnalyticsProvider';
import { load } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';
import { Analytics } from '@/lib/analytics';
import AnalyticsDataModal from './AnalyticsDataModal';
import { t } from '@/i18n/shell';

const ANALYTICS_DEFAULT_OFF_MIGRATION_KEY = 'analyticsDefaultOffMigrationV1';

export default function AnalyticsConsentSwitch() {
  const { setIsAnalyticsOptedIn, isAnalyticsOptedIn } = useContext(AnalyticsContext);
  const [isProcessing, setIsProcessing] = useState(false);
  const [showModal, setShowModal] = useState(false);
  const [userId, setUserId] = useState<string>('');
  const [isCopied, setIsCopied] = useState(false);

  // Note: Store loading is handled by AnalyticsProvider to avoid race conditions

  useEffect(() => {
    const loadUserId = async () => {
      if (isAnalyticsOptedIn) {
        try {
          const id = await Analytics.getPersistentUserId();
          setUserId(id);
        } catch (error) {
          console.error('Failed to load user ID:', error);
        }
      } else {
        setUserId('');
      }
    };
    loadUserId();
  }, [isAnalyticsOptedIn]);

  const handleCopyUserId = async () => {
    if (!userId) return;

    try {
      await navigator.clipboard.writeText(userId);
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);

      // Track that user copied their ID
      await Analytics.track('user_id_copied', {
        user_id: userId
      });
    } catch (error) {
      console.error('Failed to copy user ID:', error);
    }
  };

  const handleToggle = async (enabled: boolean) => {
    // If user is trying to DISABLE, show the modal first
    if (!enabled) {
      setShowModal(true);
      // Track that user viewed the transparency modal
      try {
        await invoke('track_analytics_transparency_viewed');
      } catch (error) {
        console.error('Failed to track transparency view:', error);
      }
      return; // Don't disable yet, wait for modal confirmation
    }

    // If ENABLING, proceed immediately
    await performToggle(enabled);
  };

  const performToggle = async (enabled: boolean) => {
    // Optimistic update - immediately update UI state
    setIsAnalyticsOptedIn(enabled);
    setIsProcessing(true);

    try {
      const store = await load('analytics.json', {
        autoSave: false,
        defaults: {
          analyticsOptedIn: false
        }
      });
      await store.set('analyticsOptedIn', enabled);
      await store.set(ANALYTICS_DEFAULT_OFF_MIGRATION_KEY, true);
      await store.save();

      if (enabled) {
        // Full analytics initialization (same as AnalyticsProvider)
        const userId = await Analytics.getPersistentUserId();

        // Initialize analytics
        await Analytics.init();

        // Identify user with enhanced properties immediately after init
        await Analytics.identify(userId, {
          app_version: '0.4.1',
          platform: 'tauri',
          first_seen: new Date().toISOString(),
          os: navigator.platform,
        });

        // Start analytics session with the same user ID
        await Analytics.startSession(userId);

        // Track app started (re-enabled)
        await Analytics.trackAppStarted();

        // Track that user enabled analytics
        try {
          await invoke('track_analytics_enabled');
        } catch (error) {
          console.error('Failed to track analytics enabled:', error);
        }

        console.log('Analytics re-enabled successfully');
      } else {
        // Track that user disabled analytics BEFORE disabling
        try {
          await invoke('track_analytics_disabled');
        } catch (error) {
          console.error('Failed to track analytics disabled:', error);
        }

        await Analytics.disable();
        console.log('Analytics disabled successfully');
      }
    } catch (error) {
      console.error('Failed to toggle analytics:', error);
      // Revert the optimistic update on error
      setIsAnalyticsOptedIn(!enabled);
      // You could also show a toast notification here to inform the user
    } finally {
      setIsProcessing(false);
    }
  };

  const handleConfirmDisable = async () => {
    setShowModal(false);
    await performToggle(false);
  };

  const handleCancelDisable = () => {
    setShowModal(false);
    // Keep analytics enabled, no state change needed
  };

  const handlePrivacyPolicyClick = async () => {
    try {
      await invoke('open_external_url', { url: 'https://github.com/Zackriya-Solutions/meeting-minutes/blob/main/PRIVACY_POLICY.md' });
    } catch (error) {
      console.error('Failed to open privacy policy link:', error);
    }
  };

  return (
    <>
      <div className="space-y-3">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h3 className="text-sm font-semibold">{t('analyticsTitle')}</h3>
            <p className="text-xs text-muted-foreground">
              {isProcessing ? t('updating') : t('analyticsDesc')}
            </p>
          </div>
          <div className="flex items-center gap-2">
            {isProcessing && (
              <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
            )}
            <Switch
              checked={isAnalyticsOptedIn}
              onCheckedChange={handleToggle}
              disabled={isProcessing}
              aria-label={t('analyticsToggle')}
            />
          </div>
        </div>

        {/* User ID Display */}
        {isAnalyticsOptedIn && userId && (
          <div className="space-y-1 border-t border-border pt-3">
            <div className="text-sm font-medium">{t('userIdTitle')}</div>
            <p className="text-xs text-muted-foreground">{t('userIdHint')}</p>
            <div className="flex items-center gap-2">
              <code className="min-w-0 flex-1 truncate rounded-sm border border-border bg-muted px-2 py-1 font-mono text-xs">
                {userId}
              </code>
              <Button
                onClick={handleCopyUserId}
                variant="outline"
                size="sm"
                className="shrink-0"
                aria-label={t('copyUserId')}
              >
                {isCopied ? <Check /> : <Copy />}
                {isCopied ? t('copied') : t('copy')}
              </Button>
            </div>
          </div>
        )}

        <p className="flex items-start gap-2 text-xs text-muted-foreground">
          <Info className="mt-px h-4 w-4 shrink-0" />
          <span>
            {t('analyticsLocal')}{' '}
            <button
              type="button"
              onClick={handlePrivacyPolicyClick}
              className="rounded-sm text-primary underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              {t('privacyPolicy')}
            </button>
          </span>
        </p>
      </div>

      {/* 2-Step Opt-Out Modal */}
      <AnalyticsDataModal
        isOpen={showModal}
        onClose={handleCancelDisable}
        onConfirmDisable={handleConfirmDisable}
      />
    </>
  );
}
