'use client';

// Dixit signature components ("Epigrafe romana"): carved speaker names, the recording light,
// and the fixed meeting-state vocabulary. Use these instead of ad-hoc labels.
import { useEffect, useRef, useState } from 'react';
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
 * CSS color of a speaker: bronze for the user, a Roman pigment per other person
 * (by order of appearance in `roster`, so a meeting never repeats a colour before the sixth voice),
 * stone grey for "Altri"/unknown.
 */
export function speakerColor(name: string | undefined, roster: string[], me: string): string {
  if (!name || name === 'Altri') return 'hsl(var(--speaker-other))';
  if (name === me || name === 'Io') return 'hsl(var(--speaker-me))';
  const others = roster.filter((n) => n && n !== me && n !== 'Io' && n !== 'Altri');
  let i = others.indexOf(name);
  if (i < 0) i = [...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 0);
  return `hsl(var(--speaker-${(i % 5) + 1}))`;
}

/** Display label: translates the backend's placeholders ("Parlante A", "Altri", "Io"). */
export function speakerLabel(name: string | undefined, me: string): string {
  if (!name || name === 'Altri') return t('others');
  if (name === 'Io' || name === me) return name === 'Io' ? t('me') : name;
  const placeholder = /^Parlante ([A-Z]+)$/.exec(name);
  return placeholder ? t('speaker', { letter: placeholder[1] }) : name;
}

/**
 * A speaker's name carved in inscriptional capitals with interpuncts (ROSSI·MARIO), in its pigment.
 * Signature move: when the name changes (e.g. "Parlante A" resolves to a real name) it is chiselled in once.
 */
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
  const previous = useRef(name);
  const [chisel, setChisel] = useState(false);
  useEffect(() => {
    if (previous.current !== name) setChisel(true);
    previous.current = name;
  }, [name]);

  const label = speakerLabel(name, me);
  return (
    <span
      key={label}
      title={label}
      className={cn('incisa inline-block truncate align-baseline text-[12px] leading-5', chisel && 'chisel', className)}
      style={{ color: speakerColor(name, roster, me) }}
    >
      {label.trim().split(/\s+/).join('·')}
    </span>
  );
}

/** The only use of minium red: Dixit is recording right now. `onRail` for the dark stone rail. */
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
        <span className={cn('absolute inline-flex h-full w-full animate-ping rounded-full opacity-60 motion-reduce:animate-none', dot)} />
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
