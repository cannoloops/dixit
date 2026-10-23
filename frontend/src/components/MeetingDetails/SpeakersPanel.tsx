"use client";

import { useId, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { SpeakerName, speakerColor, speakerLabel, useMe } from '@/components/dixit';
import { t } from '@/i18n/meeting';

interface SpeakersPanelProps {
  meetingId: string;
  /** Speaker labels with how many turns each has. */
  speakers: [string, number][];
  /** Ordered distinct speakers of the meeting, so pigments match the rest of the screen. */
  roster?: string[];
  onChanged: () => unknown;
}

/**
 * Legend of the speaker band: each speaker carved in its pigment with its number of turns.
 * One click on a name opens the rename field (updates DB, transcripts.json and .md).
 */
export function SpeakersPanel({ meetingId, speakers, roster = [], onChanged }: SpeakersPanelProps) {
  if (speakers.length === 0) return null;

  return (
    <ul className="flex flex-wrap items-center gap-x-1 gap-y-1">
      {speakers.map(([name, count]) => (
        <SpeakerItem
          key={name}
          meetingId={meetingId}
          name={name}
          count={count}
          roster={roster}
          onChanged={onChanged}
        />
      ))}
      <li className="ml-auto pl-2 text-xs text-muted-foreground">{t('renameHint')}</li>
    </ul>
  );
}

function SpeakerItem({
  meetingId,
  name,
  count,
  roster,
  onChanged,
}: {
  meetingId: string;
  name: string;
  count: number;
  roster: string[];
  onChanged: () => unknown;
}) {
  const me = useMe();
  const inputId = useId();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const label = speakerLabel(name, me);
  const next = draft.trim();

  const rename = async () => {
    if (!next || next === name) return;
    setBusy(true);
    try {
      await invoke('speakers_rename', { meetingId, from: name, to: next });
      setOpen(false);
      setDraft('');
      onChanged();
    } catch (error) {
      toast.error(t('renameFailed'), { description: String(error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <li>
      <Popover
        open={open}
        onOpenChange={(value) => {
          setOpen(value);
          if (!value) setDraft('');
        }}
      >
        <PopoverTrigger asChild>
          <button
            type="button"
            title={count === 1 ? t('turnTitle') : t('turnsTitle', { n: count })}
            className="flex h-7 max-w-[16rem] items-center gap-1.5 rounded-sm px-1.5 transition-colors duration-150 hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring data-[state=open]:bg-accent"
          >
            <span
              aria-hidden
              className="h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: speakerColor(name, roster, me) }}
            />
            <SpeakerName name={name} roster={roster} />
            <span className="text-xs tabular-nums text-muted-foreground">{count}</span>
          </button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-80 p-3">
          <form
            className="space-y-2"
            onSubmit={(event) => {
              event.preventDefault();
              void rename();
            }}
          >
            <label htmlFor={inputId} className="block text-xs text-muted-foreground">
              {t('newNameFor', { name: label })}
            </label>
            <div className="flex gap-2">
              <Input
                id={inputId}
                autoFocus
                className="h-8 flex-1"
                placeholder={t('newName')}
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
              />
              <Button type="submit" disabled={busy || !next || next === name}>
                {busy ? t('renaming') : t('rename')}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">{t('renameScope')}</p>
          </form>
        </PopoverContent>
      </Popover>
    </li>
  );
}
