'use client';

// Dense model list shared by the transcription and summary model managers:
// hairline-separated rows with name, size, state and action. No cards, no coloured badges.
import { Check, Download, Loader2, RefreshCw, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { t } from '@/i18n/models';

export type ModelRowState =
  | { kind: 'missing' }
  | { kind: 'installed'; active: boolean }
  | { kind: 'downloading'; progress: number; cancelling?: boolean; info?: string }
  | { kind: 'error'; message: string }
  | { kind: 'corrupted'; message: string };

/** Error sentence with its remedy: what happened, then what to do. */
export function modelErrorMessage(reason: string): string {
  return t('downloadFailedReason', { reason: reason.trim().replace(/[.\s]+$/, '') });
}

export function ModelList({ children, className }: { children: React.ReactNode; className?: string }) {
  return <ul className={cn('divide-y divide-border border-y border-border', className)}>{children}</ul>;
}

export function DownloadBar({
  progress,
  info,
  className,
}: {
  progress: number;
  info?: string;
  className?: string;
}) {
  const width = Math.min(100, Math.max(0, progress));
  return (
    <div className={cn('flex items-center gap-3', className)}>
      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(width)}
        className="h-1.5 flex-1 overflow-hidden rounded-sm bg-muted"
      >
        <div className="h-full bg-primary transition-[width] duration-200" style={{ width: `${width}%` }} />
      </div>
      <span className="w-9 shrink-0 text-right text-xs tabular-nums text-foreground">{Math.round(width)}%</span>
      {info && <span className="shrink-0 text-xs tabular-nums text-muted-foreground">{info}</span>}
    </div>
  );
}

export function ModelListLoading() {
  return (
    <p className="flex items-center gap-2 py-2 text-sm text-muted-foreground" role="status">
      <Loader2 className="h-4 w-4 animate-spin" />
      {t('loadingModels')}
    </p>
  );
}

export function ModelListError({ message, className }: { message: string; className?: string }) {
  return (
    <div role="alert" className={cn('space-y-1', className)}>
      <p className="text-sm font-medium text-destructive">{t('loadModelsFailed')}</p>
      <p className="text-xs text-destructive">
        {message.trim().replace(/[.\s]+$/, '')}. {t('loadModelsRemedy')}
      </p>
    </div>
  );
}

interface ModelRowProps {
  name: string;
  /** Short description, truncated on one line (full text in the tooltip). */
  detail?: string;
  size?: string;
  /** Sober text labels next to the name (e.g. "Consigliato"). */
  labels?: string[];
  state: ModelRowState;
  onSelect?: () => void;
  onDownload?: () => void;
  onCancel?: () => void;
  onDelete?: () => void;
}

export function ModelRow({ name, detail, size, labels, state, onSelect, onDownload, onCancel, onDelete }: ModelRowProps) {
  const failed = state.kind === 'error' || state.kind === 'corrupted';
  const cancelling = state.kind === 'downloading' && state.cancelling;

  const deleteButton = onDelete && (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className={cn(
        'h-7 w-7 text-muted-foreground hover:text-destructive',
        state.kind === 'installed' && 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'
      )}
      onClick={onDelete}
      title={t('deleteModelTitle')}
      aria-label={t('deleteModelOf', { name })}
    >
      <Trash2 />
    </Button>
  );

  return (
    <li className="group py-0.5">
      <div className="flex min-h-7 items-center gap-3">
        <div className="flex min-w-0 flex-1 items-baseline gap-2" title={detail ? `${name}: ${detail}` : name}>
          <span className="max-w-[65%] shrink-0 truncate text-sm font-medium text-foreground">{name}</span>
          {labels?.map((label) => (
            <span
              key={label}
              className="inline-flex h-5 shrink-0 items-center self-center rounded-sm border border-border px-1.5 text-xs text-muted-foreground"
            >
              {label}
            </span>
          ))}
          {detail && <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">{detail}</span>}
        </div>

        <span className="w-16 shrink-0 text-right text-xs tabular-nums text-muted-foreground">{size}</span>

        <span className="w-24 shrink-0 text-xs">
          {state.kind === 'installed' && state.active ? (
            <span className="inline-flex items-center gap-1 font-medium text-success">
              <Check className="h-4 w-4" />
              {t('inUse')}
            </span>
          ) : failed ? (
            <span className="font-medium text-destructive">
              {state.kind === 'corrupted' ? t('stateCorrupted') : t('stateError')}
            </span>
          ) : (
            <span className="text-muted-foreground">
              {state.kind === 'missing'
                ? t('stateAvailable')
                : state.kind === 'installed'
                  ? t('stateInstalled')
                  : cancelling
                    ? t('stateCancelling')
                    : t('stateDownloading')}
            </span>
          )}
        </span>

        <div className="flex w-40 shrink-0 items-center justify-end gap-1">
          {state.kind === 'missing' && onDownload && (
            <Button type="button" variant="outline" size="sm" onClick={onDownload} aria-label={t('downloadModelOf', { name })}>
              <Download />
              {t('download')}
            </Button>
          )}
          {state.kind === 'installed' && (
            <>
              {!state.active && onSelect && (
                <Button type="button" variant="outline" size="sm" onClick={onSelect} aria-label={t('useModelOf', { name })}>
                  {t('use')}
                </Button>
              )}
              {deleteButton}
            </>
          )}
          {state.kind === 'downloading' &&
            (cancelling ? (
              <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" aria-hidden />
            ) : (
              onCancel && (
                <Button type="button" variant="ghost" size="sm" onClick={onCancel} aria-label={t('cancelDownloadOf', { name })}>
                  {t('cancel')}
                </Button>
              )
            ))}
          {state.kind === 'error' && onDownload && (
            <Button type="button" variant="outline" size="sm" onClick={onDownload} aria-label={t('retryOf', { name })}>
              <RefreshCw />
              {t('retry')}
            </Button>
          )}
          {state.kind === 'corrupted' && (
            <>
              {onDownload && (
                <Button type="button" variant="outline" size="sm" onClick={onDownload} aria-label={t('redownloadOf', { name })}>
                  <RefreshCw />
                  {t('redownload')}
                </Button>
              )}
              {deleteButton}
            </>
          )}
        </div>
      </div>

      {state.kind === 'downloading' && <DownloadBar progress={state.progress} info={state.info} className="pb-1.5 pt-0.5" />}
      {failed && (
        <p role="alert" className="pb-1.5 text-xs text-destructive">
          {state.message}
        </p>
      )}
    </li>
  );
}
