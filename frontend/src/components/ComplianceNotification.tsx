'use client';

import React, { useState, useEffect, useRef } from 'react';
import { Button } from './ui/button';
import { AlertTriangle, Check, X } from 'lucide-react';
import { t } from '@/i18n/shell';
import { t as tApp } from '@/i18n/app';

interface ComplianceNotificationProps {
  isOpen: boolean;
  onClose: () => void;
  onAcknowledge: () => void;
  recordingButtonRef?: React.RefObject<HTMLElement | HTMLButtonElement>;
}

export const ComplianceNotification: React.FC<ComplianceNotificationProps> = ({
  isOpen,
  onClose,
  onAcknowledge,
  recordingButtonRef,
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0, width: 192 }); // Default width

  useEffect(() => {
    if (isOpen) {
      setIsVisible(true);
      
      // Calculate position relative to recording button
      if (recordingButtonRef?.current) {
        const buttonRect = recordingButtonRef.current.getBoundingClientRect();
        const buttonWidth = buttonRect.width;
        const notificationWidth = buttonWidth * 1.5; // 1.5x the button width
        
        setPosition({
          top: buttonRect.top - 100, // 100px above the button
          left: buttonRect.left + (buttonWidth - notificationWidth) / 2, // Center the notification relative to button
          width: notificationWidth,
        });
      } else {
        // Fallback position if no button ref
        setPosition({
          top: window.innerHeight - 200, // Near bottom of screen
          left: window.innerWidth - 250, // Near right edge
          width: 192, // Default width
        });
      }
    }
  }, [isOpen, recordingButtonRef]);

  const handleClose = () => {
    setIsVisible(false);
    setTimeout(() => {
      onClose();
    }, 200);
  };

  const handleAcknowledge = () => {
    onAcknowledge();
    handleClose();
  };

  if (!isOpen) return null;

  return (
    <div 
      className={`fixed z-50 transition-opacity duration-150 ${
        isVisible ? 'opacity-100' : 'opacity-0'
      }`}
      style={{
        top: `${position.top}px`,
        left: `${position.left}px`,
        width: `${position.width}px`,
      }}
    >
      <div className="rounded-md border border-border bg-popover p-3 text-popover-foreground shadow-(--shadow-float)">
        {/* Header with close button */}
        <div className="flex items-start justify-between mb-2">
          <div className="flex items-center gap-1">
            <AlertTriangle className="h-4 w-4 shrink-0 text-muted-foreground" />
            <h3 className="text-xs font-semibold">
              {t('recordingNotice')}
            </h3>
          </div>
          <button
            onClick={handleClose}
            className="rounded-sm p-0.5 text-muted-foreground transition-colors duration-150 hover:bg-accent hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
            aria-label={tApp('close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Content */}
        <div className="mb-2">
          <p className="text-xs text-muted-foreground">
            {t('informParticipants')}
          </p>
          <p className="mt-1 text-xs font-medium">
            {t('usCompliance')}
          </p>
        </div>

        {/* Actions */}
        <div className="flex gap-1">
          <Button
            variant="outline"
            size="sm"
            onClick={handleClose}
            className="flex-1"
          >
            {t('later')}
          </Button>
          <Button
            size="sm"
            onClick={handleAcknowledge}
            className="flex-1"
          >
            <Check />
            {t('done')}
          </Button>
        </div>
      </div>
    </div>
  );
};
