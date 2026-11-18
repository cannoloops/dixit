'use client';

import { useEffect, useState } from 'react';
import { t as tr } from '@/i18n/shell';

// The "mcp" scene of site/demo/scenes.html, same timings: Claude asks, Dixit answers through MCP.
const DURATION = 11;

type Line =
  | { kind: 'prompt' | 'ans'; text: string; at: number; cps: number }
  | { kind: 'tool'; tool: string; args: string; at: number }
  | { kind: 'res'; text: string; at: number };

const lines = (): Line[] => [
  { kind: 'prompt', text: tr('demoAsk'), at: 0.3, cps: 32 },
  { kind: 'tool', tool: 'search_transcripts', args: tr('demoSearchArgs'), at: 2.9 },
  { kind: 'res', text: tr('demoSearchRes'), at: 3.5 },
  { kind: 'tool', tool: 'get_meeting', args: '(format: markdown)', at: 4.2 },
  { kind: 'res', text: tr('demoMeetingRes'), at: 4.8 },
  { kind: 'ans', text: tr('demoAnswer'), at: 5.5, cps: 60 },
];

const fade = (t: number, at: number) => {
  const k = Math.min(1, Math.max(0, (t - at) / 0.25));
  return { opacity: k, transform: `translateY(${(1 - k) * 6}px)` };
};

function Typed({ text, t, at, cps }: { text: string; t: number; at: number; cps: number }) {
  const n = Math.max(0, Math.min(text.length, Math.floor((t - at) * cps)));
  const typing = t >= at && n < text.length;
  return (
    <>
      {text.slice(0, n)}
      {typing && <span className="ml-px inline-block h-[1.1em] w-[1.5px] bg-current align-[-0.15em]" />}
    </>
  );
}

export function AgentsDemo() {
  const [t, setT] = useState(0);

  useEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      setT(DURATION); // last frame, no motion
      return;
    }
    const t0 = performance.now();
    let id = requestAnimationFrame(function loop(now) {
      setT(((now - t0) / 1000) % DURATION);
      id = requestAnimationFrame(loop);
    });
    return () => cancelAnimationFrame(id);
  }, []);

  return (
    <div
      role="img"
      aria-label={tr('demoLabel')}
      className="h-[17rem] overflow-hidden rounded-[10px] bg-[#1F2023] font-mono text-[12.5px] leading-[1.6] text-[#E6E6E8] shadow-[0_8px_24px_rgba(0,0,0,.18)]"
    >
      <div className="flex items-center gap-1.5 border-b border-[#333438] px-3 py-2.5" aria-hidden>
        <i className="size-[11px] rounded-full bg-[#3A3B3F]" />
        <i className="size-[11px] rounded-full bg-[#3A3B3F]" />
        <i className="size-[11px] rounded-full bg-[#3A3B3F]" />
        <span className="mr-[50px] flex-1 text-center font-display text-xs font-semibold text-[#8C8E93]">claude</span>
      </div>
      <div className="space-y-2 px-4 py-3" aria-hidden>
        {lines().map((l, i) => {
          if (l.kind === 'prompt')
            return (
              <p key={i} className="min-h-[2.3em] rounded-md border border-[#3A3B3F] px-2.5 py-1.5">
                <span className="text-[#8C8E93]">&gt; </span>
                <Typed text={l.text} t={t} at={l.at} cps={l.cps} />
              </p>
            );
          if (l.kind === 'ans')
            return (
              <p key={i} style={{ opacity: t >= l.at ? 1 : 0 }}>
                ⏺ <Typed text={l.text} t={t} at={l.at} cps={l.cps} />
              </p>
            );
          if (l.kind === 'tool')
            return (
              <p key={i} style={fade(t, l.at)}>
                <span className="text-[#8DC828]">⏺ </span>
                <b className="font-semibold text-[#5BB7EC]">dixit</b> · {l.tool} {l.args}
              </p>
            );
          return (
            <p key={i} className="pl-[18px] text-[#9A9CA1]" style={fade(t, l.at)}>
              ⎿ {l.text}
            </p>
          );
        })}
      </div>
    </div>
  );
}
