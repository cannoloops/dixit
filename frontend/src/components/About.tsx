import React, { useState, useEffect } from "react";
import { getVersion } from '@tauri-apps/api/app';
import { DixitMark } from "@/components/dixit/DixitMark";
import { UpdateDialog } from "./UpdateDialog";
import { updateService, UpdateInfo } from '@/services/updateService';
import { Button } from './ui/button';
import { Loader2, RefreshCw } from 'lucide-react';
import { toast } from 'sonner';
import { t } from '@/i18n/shell';


export function About() {
    const [currentVersion, setCurrentVersion] = useState<string>('');
    const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
    const [isChecking, setIsChecking] = useState(false);
    const [showUpdateDialog, setShowUpdateDialog] = useState(false);

    useEffect(() => {
        // Get current version on mount
        getVersion().then(setCurrentVersion).catch(console.error);
    }, []);

    const handleCheckForUpdates = async () => {
        setIsChecking(true);
        try {
            const info = await updateService.checkForUpdates(true);
            setUpdateInfo(info);
            if (info.available) {
                setShowUpdateDialog(true);
            } else {
                toast.success(t('upToDate'));
            }
        } catch (error: any) {
            console.error('Failed to check for updates:', error);
            toast.error(t('updateCheckFailed', { error: error?.message || t('unknownError') }));
        } finally {
            setIsChecking(false);
        }
    };

    return (
        <div className="flex flex-col items-center gap-4 pt-2 text-center">
            <DixitMark className="h-16 w-16 text-4xl text-primary" />
            <div className="space-y-0.5">
                <p className="text-lg font-semibold">Dixit</p>
                {currentVersion && (
                    <p className="text-xs tabular-nums text-muted-foreground">
                        {t('version', { version: currentVersion })}
                    </p>
                )}
            </div>
            <p className="max-w-xs text-sm text-muted-foreground">{t('aboutDescription')}</p>

            <div className="flex flex-col items-center gap-1.5">
                <Button onClick={handleCheckForUpdates} disabled={isChecking} variant="outline" size="sm">
                    {isChecking ? <Loader2 className="animate-spin" /> : <RefreshCw />}
                    {isChecking ? t('checking') : t('checkUpdates')}
                </Button>
                {updateInfo?.available && (
                    <Button variant="link" size="sm" className="h-auto p-0" onClick={() => setShowUpdateDialog(true)}>
                        {t('versionAvailable', { version: updateInfo.version ?? '' })}
                    </Button>
                )}
            </div>

            {/* Update Dialog */}
            <UpdateDialog
                open={showUpdateDialog}
                onOpenChange={setShowUpdateDialog}
                updateInfo={updateInfo}
            />
        </div>
    )
}
