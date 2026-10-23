"use client";

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ChevronDown, ChevronRight, Users } from 'lucide-react';
import { toast } from 'sonner';
import { Input } from '@/components/ui/input';

interface SpeakersPanelProps {
  meetingId: string;
  /** Speaker labels with how many segments each has. */
  speakers: [string, number][];
  onChanged: () => unknown;
}

/** Trascrivi: speakers of a meeting, each renamable (updates DB, transcripts.json and .md). */
export function SpeakersPanel({ meetingId, speakers, onChanged }: SpeakersPanelProps) {
  const [open, setOpen] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);

  // Attribution finishes after the meeting is saved, and Teams transcripts can arrive later.
  useEffect(() => {
    const unlisten = listen('speakers-updated', () => onChanged());
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [onChanged]);

  if (speakers.length === 0) return null;

  const rename = async (from: string) => {
    const to = (drafts[from] ?? '').trim();
    if (!to || to === from) return;
    setBusy(from);
    try {
      await invoke('speakers_rename', { meetingId, from, to });
      setDrafts((d) => ({ ...d, [from]: '' }));
      onChanged();
    } catch (error) {
      toast.error('Rinomina non riuscita', { description: String(error) });
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="border-b border-gray-200 px-4 py-2">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 text-sm font-medium text-gray-700"
        aria-expanded={open}
      >
        {open ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
        <Users className="w-4 h-4" />
        Parlanti ({speakers.length})
      </button>
      {open && (
        <ul className="mt-2 space-y-2">
          {speakers.map(([name, count]) => (
            <li key={name} className="flex items-center gap-2">
              <span className="text-sm text-gray-800 w-48 truncate" title={name}>
                {name} <span className="text-gray-400">({count})</span>
              </span>
              <Input
                aria-label={`Nuovo nome per ${name}`}
                className="h-8 flex-1"
                placeholder="Nuovo nome"
                value={drafts[name] ?? ''}
                onChange={(e) => setDrafts((d) => ({ ...d, [name]: e.target.value }))}
                onKeyDown={(e) => e.key === 'Enter' && rename(name)}
              />
              <button
                onClick={() => rename(name)}
                disabled={busy !== null || !(drafts[name] ?? '').trim()}
                className="px-3 py-1 text-sm border border-gray-300 rounded-md hover:bg-gray-50 transition-colors disabled:opacity-50"
              >
                {busy === name ? 'Rinomino…' : 'Rinomina'}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
