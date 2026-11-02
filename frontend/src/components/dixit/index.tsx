'use client';

// Dixit shared components (design system): speaker names, the recording light,
// and the fixed meeting-state vocabulary. Use these instead of ad-hoc labels.
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { cn } from '@/lib/utils';
import { t } from '@/i18n/app';

let meCache: Promise<string> | null = null;

/** The user's own speaker label ("Cognome Nome" from Settings → Automazione, or "Io"). */
export function useMe(): string {
  const [me, setMe] = useState('Io');
  useEffect(() => {
    meCache ??= invoke<{ user_display_name: string }>('autopilot_get_settings')
      .then((s) => s.user_display_name.trim() || 'Io')
      .catch(() => 'Io');
    meCache.then(setMe);
  }, []);
  return me;
}

/**
 * CSS color of a speaker: green for the user, a distinct colour per other person
 * (by position in `roster`, so a meeting never repeats a colour before the eighth voice),
 * stone grey for "Altri"/unknown.
 */
export function speakerColor(name: string | undefined, roster: string[], me: string): string {
  if (!name || name === 'Altri') return 'hsl(var(--speaker-other))';
  if (name === me || name === 'Io') return 'hsl(var(--speaker-me))';
  const others = roster.filter((n) => n && n !== me && n !== 'Io' && n !== 'Altri');
  let i = others.indexOf(name);
  if (i < 0) i = [...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 0);
  return `hsl(var(--speaker-${(i % 7) + 1}))`;
}

/** Display label: translates the backend's placeholders ("Parlante A", "Altri", "Io"). */
export function speakerLabel(name: string | undefined, me: string): string {
  if (!name || name === 'Altri') return t('others');
  if (name === 'Io' || name === me) return name === 'Io' ? t('me') : name;
  const placeholder = /^Parlante ([A-Z]+)$/.exec(name);
  return placeholder ? t('speaker', { letter: placeholder[1] }) : name;
}

/** A speaker's name in their colour (Montserrat semibold), the same everywhere in the app. */
export function SpeakerName({
  name,
  roster = [],
  className,
}: {
  name?: string;
  roster?: string[];
  className?: string;
}) {
  const me = useMe();
  const label = speakerLabel(name, me);
  return (
    <span
      title={label}
      className={cn('inline-block truncate align-baseline font-display text-[13px] font-semibold leading-5', className)}
      style={{ color: speakerColor(name, roster, me) }}
    >
      {label}
    </span>
  );
}

/** Recording light: red only while Dixit is recording. `onRail` for the sidebar. */
export function LiveIndicator({
  label = t('recording'),
  onRail = false,
  className,
}: {
  label?: string;
  onRail?: boolean;
  className?: string;
}) {
  const dot = onRail ? 'bg-rail-live' : 'bg-live';
  return (
    <span
      className={cn('inline-flex items-center gap-1.5 text-xs font-medium', onRail ? 'text-rail-live' : 'text-live', className)}
      role="status"
    >
      <span className="relative flex h-2 w-2">
        <span className={cn('relative inline-flex h-2 w-2 rounded-full', dot)} />
      </span>
      {label}
    </span>
  );
}

export type MeetingState = 'recording' | 'processing' | 'saved' | 'verify' | 'cancelled';

const STATE_STYLE: Record<MeetingState, string> = {
  recording: 'border-live/50 text-live',
  processing: 'border-border text-muted-foreground',
  saved: 'border-border text-muted-foreground',
  verify: 'border-primary/50 text-primary',
  cancelled: 'border-border text-muted-foreground line-through',
};

const STATE_LABEL: Record<MeetingState, Parameters<typeof t>[0]> = {
  recording: 'recording',
  processing: 'processing',
  saved: 'saved',
  verify: 'verifyNames',
  cancelled: 'cancelled',
};

/** Fixed state vocabulary: every meeting state has one label and one colour, everywhere. */
export function StateStamp({ state, className }: { state: MeetingState; className?: string }) {
  return (
    <span
      className={cn(
        'inline-flex h-5 items-center rounded-sm border px-1.5 text-[11px] font-medium leading-none',
        STATE_STYLE[state],
        className
      )}
    >
      {t(STATE_LABEL[state])}
    </span>
  );
}
