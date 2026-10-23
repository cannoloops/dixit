import { toast } from 'sonner';
import Analytics from '@/lib/analytics';
import { Button } from '@/components/ui/button';
import { t } from '@/i18n/recording';

/**
 * Shows the recording notification toast with compliance message.
 * Checks user preferences and displays a dismissible toast with:
 * - notice to inform participants
 * - "Don't show again" checkbox
 * - Acknowledgment button
 *
 * @returns Promise<void> - Resolves when notification is shown or skipped
 */
export async function showRecordingNotification(): Promise<void> {
  try {
    const { Store } = await import('@tauri-apps/plugin-store');
    const store = await Store.load('preferences.json');
    const showNotification = await store.get<boolean>('show_recording_notification') ?? true;

    if (showNotification) {
      let dontShowAgain = false;

      const toastId = toast.info(t('notifyTitle'), {
        description: (
          <div className="min-w-[280px] space-y-3">
            <p className="text-sm text-foreground">{t('notifyBody')}</p>
            <label className="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
              <input
                type="checkbox"
                onChange={(e) => {
                  dontShowAgain = e.target.checked;
                }}
                className="h-4 w-4 rounded-sm border-input accent-primary"
              />
              <span className="select-none">{t('dontShowAgain')}</span>
            </label>
            <Button
              size="sm"
              className="w-full"
              onClick={async () => {
                if (dontShowAgain) {
                  const { Store } = await import('@tauri-apps/plugin-store');
                  const store = await Store.load('preferences.json');
                  await store.set('show_recording_notification', false);
                  await store.save();
                }
                Analytics.trackButtonClick('recording_notification_acknowledged', 'toast');
                toast.dismiss(toastId);
              }}
            >
              {t('notifiedParticipants')}
            </Button>
          </div>
        ),
        duration: 10000,
        position: 'bottom-right',
      });
    }
  } catch (notificationError) {
    console.error('Failed to show recording notification:', notificationError);
    // Don't fail the recording if notification fails
  }
}
