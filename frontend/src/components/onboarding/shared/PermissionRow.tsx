import React from 'react';
import { Check, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { t } from '@/i18n/onboarding';
import type { PermissionRowProps } from '@/types/onboarding';

type Props = PermissionRowProps & {
  /** What to do when macOS denied the permission (shown in place of the description). */
  deniedHint?: string;
};

/** One permission: name, what it is for, state and the "Concedi" button. Granted = a green check. */
export function PermissionRow({ icon, title, description, status, isPending = false, onAction, deniedHint }: Props) {
  const isAuthorized = status === 'authorized';
  const isDenied = status === 'denied';
  const isChecking = isPending || status === 'checking';

  const buttonText = isChecking ? t('checking') : isDenied ? t('openSettings') : t('grant');

  return (
    <div className="flex items-center gap-3 py-3">
      <span className="flex h-4 w-4 shrink-0 items-center justify-center text-muted-foreground [&_svg]:h-4 [&_svg]:w-4">
        {icon}
      </span>

      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium">{title}</p>
        <p className={cn('text-xs', isDenied ? 'text-destructive' : 'text-muted-foreground')}>
          {isDenied ? (deniedHint ?? t('deniedDefault')) : description}
        </p>
      </div>

      <div className="shrink-0">
        {isAuthorized ? (
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-success">
            <Check className="h-4 w-4" aria-hidden />
            {t('granted')}
          </span>
        ) : (
          <Button variant="outline" size="sm" onClick={onAction} disabled={isChecking}>
            {isChecking && <Loader2 className="h-4 w-4 animate-spin" />}
            {buttonText}
          </Button>
        )}
      </div>
    </div>
  );
}
