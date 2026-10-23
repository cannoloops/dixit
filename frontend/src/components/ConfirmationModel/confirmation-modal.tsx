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
}

export function ConfirmationModal({ onConfirm, onCancel, text, isOpen }: ConfirmationModalProps) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onCancel(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t('confirmDeleteTitle')}</DialogTitle>
          <DialogDescription>{text}</DialogDescription>
        </DialogHeader>
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
