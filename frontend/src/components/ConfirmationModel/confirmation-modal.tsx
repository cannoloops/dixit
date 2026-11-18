import React from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { t } from '@/i18n/shell';
import { t as tApp } from '@/i18n/app';

interface ConfirmationModalProps {
  onConfirm: () => void;
  onCancel: () => void;
  text: string;
  isOpen: boolean;
  /** Defaults to the single-meeting delete title. */
  title?: string;
  /** Extra controls between the text and the buttons (e.g. an option checkbox). */
  children?: React.ReactNode;
}

export function ConfirmationModal({ onConfirm, onCancel, text, isOpen, title, children }: ConfirmationModalProps) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onCancel(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{title ?? t('confirmDeleteTitle')}</DialogTitle>
          <DialogDescription>{text}</DialogDescription>
        </DialogHeader>
        {children}
        <DialogFooter>
          <Button variant="outline" onClick={onCancel}>
            {tApp('cancel')}
          </Button>
          <Button variant="destructive" onClick={onConfirm}>
            {t('delete')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
