'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { parseReleaseNotes, notesToShow, ReleaseNotes } from '@/lib/releaseNotes';
import { t } from '@/i18n/shell';

// ponytail: per-Mac flag in localStorage; if it gets wiped, the current version's news shows once more.
const SEEN_KEY = 'dixit-notes-seen-version';
const SHOW_EVENT = 'dixit-show-whats-new';

const setSeen = (version: string) => {
  try { localStorage.setItem(SEEN_KEY, version); } catch {}
};

/** After the onboarding of a fresh install: nothing is "new" yet. */
export const markWhatsNewSeen = () => getVersion().then(setSeen).catch(console.error);

/** Opens the news of the current version again (Info, at the bottom of the sidebar). */
export const showWhatsNew = () => window.dispatchEvent(new Event(SHOW_EVENT));

const load = async (seen: string | null | 'current') => {
  const [current, markdown] = await Promise.all([getVersion(), invoke<string>('release_notes')]);
  const all = parseReleaseNotes(markdown);
  return { current, notes: notesToShow(all, current, seen === 'current' ? null : seen) };
};

/** Once per update, at startup: what changed since the version the user last saw. */
export function WhatsNew() {
  const [notes, setNotes] = useState<ReleaseNotes[]>([]);
  const [current, setCurrent] = useState('');

  useEffect(() => {
    const reopen = () => load('current').then((r) => (setCurrent(r.current), setNotes(r.notes))).catch(console.error);
    window.addEventListener(SHOW_EVENT, reopen);
    return () => window.removeEventListener(SHOW_EVENT, reopen);
  }, []);

  useEffect(() => {
    let seen: string | null;
    try {
      // ponytail: the agents announcement goes first; the news waits for the next start.
      if (!localStorage.getItem('dixit-agents-announced')) return;
      seen = localStorage.getItem(SEEN_KEY);
    } catch {
      return;
    }
    load(seen)
      .then(({ current, notes }) => {
        setCurrent(current);
        if (notes.length) setNotes(notes);
        else setSeen(current);
      })
      .catch(console.error);
  }, []);

  const close = () => {
    if (current) setSeen(current);
    setNotes([]);
  };

  return (
    <Dialog open={notes.length > 0} onOpenChange={(open) => !open && close()}>
      {/* Opened at startup, often before the window has focus: losing focus must not dismiss it. */}
      <DialogContent className="gap-5 sm:max-w-lg" onFocusOutside={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle className="text-lg">{t('whatsNewTitle', { version: current })}</DialogTitle>
          <DialogDescription>{t('whatsNewBody')}</DialogDescription>
        </DialogHeader>
        <div className="max-h-[50vh] space-y-4 overflow-y-auto">
          {notes.map((n) => (
            <section key={n.version} className="space-y-1.5">
              {notes.length > 1 && <h3 className="text-xs font-medium tabular-nums text-muted-foreground">{n.version}</h3>}
              <ul className="list-disc space-y-1.5 pl-5 text-sm">
                {n.items.map((item) => <li key={item}>{item}</li>)}
              </ul>
            </section>
          ))}
        </div>
        <DialogFooter>
          <Button onClick={close}>{t('whatsNewOk')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
