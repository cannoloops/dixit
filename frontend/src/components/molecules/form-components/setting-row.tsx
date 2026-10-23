import * as React from 'react';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

interface SettingsSectionProps {
  title: string;
  description?: string;
  className?: string;
  children: React.ReactNode;
}

/** Group of settings: a short title, an optional description, then rows split by hairlines (no card). */
export function SettingsSection({ title, description, className, children }: SettingsSectionProps) {
  return (
    <section className={cn('space-y-2', className)}>
      <div>
        <h3 className="text-sm font-semibold">{title}</h3>
        {description && <p className="text-xs text-muted-foreground">{description}</p>}
      </div>
      <div className="divide-y divide-border border-y border-border">{children}</div>
    </section>
  );
}

interface SettingRowProps {
  label: string;
  description?: React.ReactNode;
  /** id of the control on the right, so the label names it for assistive technology. */
  htmlFor?: string;
  className?: string;
  children?: React.ReactNode;
}

/** One setting: label and description on the left, the control on the right. */
export function SettingRow({ label, description, htmlFor, className, children }: SettingRowProps) {
  return (
    <div className={cn('flex items-center justify-between gap-6 py-2.5', className)}>
      <div className="min-w-0 flex-1">
        <Label htmlFor={htmlFor} className="block leading-snug">
          {label}
        </Label>
        {description && <div className="mt-0.5 text-xs text-muted-foreground">{description}</div>}
      </div>
      {children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
    </div>
  );
}
