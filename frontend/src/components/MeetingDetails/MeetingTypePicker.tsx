"use client";

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Check, Tag } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { t } from '@/i18n/meeting';

export type MeetingType = 'sal' | 'analisi' | 'mix' | 'altro';
const TYPES: MeetingType[] = ['sal', 'analisi', 'mix', 'altro'];
const LABEL = { sal: 'typeSal', analisi: 'typeAnalisi', mix: 'typeMix', altro: 'typeAltro' } as const;

interface MeetingTypePickerProps {
  meetingId: string;
  /** Reloads the type when it changes (the classification runs with the summary). */
  refreshKey: string;
  /** Called after the new type is saved: regenerates the summary with it. */
  onChanged: () => void;
}

/** Dixit: call type (SAL, analysis, mix, other) that picks the summary template. */
export function MeetingTypePicker({ meetingId, refreshKey, onChanged }: MeetingTypePickerProps) {
  const [info, setInfo] = useState<{ type: MeetingType; reason: string | null } | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<{ type: MeetingType; reason: string | null } | null>('api_get_meeting_type', { meetingId })
      .then((value) => !cancelled && setInfo(value))
      .catch(console.error);
    return () => {
      cancelled = true;
    };
  }, [meetingId, refreshKey]);

  const choose = async (type: MeetingType) => {
    try {
      await invoke('api_set_meeting_type', { meetingId, meetingType: type });
      setInfo({ type, reason: null });
      onChanged();
    } catch (error) {
      toast.error(t('typeSaveFailed'), { description: String(error) });
    }
  };

  const reason = info ? info.reason ?? t('typeManual') : t('typeUnknown');
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" title={reason}>
          <Tag />
          <span>{info ? t(LABEL[info.type]) : t('typeAuto')}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="max-w-xs">
        <DropdownMenuLabel className="font-normal text-muted-foreground">{reason}</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {TYPES.map((type) => (
          <DropdownMenuItem
            key={type}
            onClick={() => void choose(type)}
            className="flex items-center justify-between gap-2"
          >
            <span>{t(LABEL[type])}</span>
            {info?.type === type && <Check className="h-4 w-4 text-primary" />}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
