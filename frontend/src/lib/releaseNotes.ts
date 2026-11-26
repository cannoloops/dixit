// Changelog of docs/release-notes.md ("**Novità in 0.2.11**" followed by "- " lines).

export interface ReleaseNotes {
  version: string;
  items: string[];
}

export function parseReleaseNotes(markdown: string): ReleaseNotes[] {
  const notes: ReleaseNotes[] = [];
  let current: ReleaseNotes | null = null;
  for (const line of markdown.split('\n')) {
    const header = line.match(/^\*\*Novità in (\d+(?:\.\d+)*)\*\*\s*$/);
    if (header) {
      current = { version: header[1], items: [] };
      notes.push(current);
    } else if (current && line.startsWith('- ')) {
      current.items.push(line.slice(2).trim());
    } else if (!line.trim()) {
      current = null;
    }
  }
  return notes;
}

/** -1, 0, 1 comparing dotted numeric versions. */
export function compareVersions(a: string, b: string): number {
  const pa = a.split('.').map(Number);
  const pb = b.split('.').map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d) return Math.sign(d);
  }
  return 0;
}

/** Releases after `seen` up to `current`, newest first; just `current` when `seen` is unknown. */
export function notesToShow(all: ReleaseNotes[], current: string, seen: string | null): ReleaseNotes[] {
  return all.filter((n) =>
    seen === null
      ? compareVersions(n.version, current) === 0
      : compareVersions(n.version, seen) > 0 && compareVersions(n.version, current) <= 0,
  );
}
