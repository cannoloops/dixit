'use client';

import { useRef, useEffect } from 'react';
import { Pencil, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { t } from '@/i18n/meeting';

interface EditableTitleProps {
  title: string;
  isEditing: boolean;
  onStartEditing: () => void;
  onFinishEditing: () => void;
  onChange: (value: string) => void;
  onDelete?: () => void;
  /** Type of the title in both states, e.g. "incisa text-2xl" for a meeting title. */
  className?: string;
}

export const EditableTitle: React.FC<EditableTitleProps> = ({
  title,
  isEditing,
  onStartEditing,
  onFinishEditing,
  onChange,
  onDelete,
  className,
}) => {
  const titleInputRef = useRef<HTMLTextAreaElement>(null);
  const type = cn('text-base font-semibold', className);

  // Auto-resize textarea height based on content
  useEffect(() => {
    if (titleInputRef.current && isEditing) {
      titleInputRef.current.style.height = 'auto';
      titleInputRef.current.style.height = `${titleInputRef.current.scrollHeight}px`;
    }
  }, [title, isEditing]);

  return isEditing ? (
    <div className="min-w-0 flex-1">
      <textarea
        ref={titleInputRef}
        value={title}
        aria-label={t('editTitle')}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onFinishEditing}
        onKeyDown={(e) => {
          // Allow Enter for new line only with Shift key
          if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            onFinishEditing();
          }
        }}
        className={cn(
          type,
          'w-full resize-none overflow-hidden rounded-sm border border-input bg-background px-1 text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
        )}
        style={{ minWidth: '300px' }}
        autoFocus
        rows={1}
      />
    </div>
  ) : (
    <div className="group flex min-w-0 flex-1 items-center gap-1">
      <h1
        className={cn(type, 'min-w-0 cursor-text whitespace-pre-wrap rounded-sm px-1 transition-colors duration-150 hover:bg-accent')}
        onClick={onStartEditing}
      >
        {title}
      </h1>
      <button
        type="button"
        onClick={onStartEditing}
        className="shrink-0 rounded-sm p-1 text-muted-foreground opacity-0 transition-opacity duration-150 hover:bg-accent hover:text-foreground focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring group-hover:opacity-100"
        title={t('editTitle')}
        aria-label={t('editTitle')}
      >
        <Pencil className="h-4 w-4" />
      </button>
      {onDelete && (
        <button
          type="button"
          onClick={onDelete}
          className="shrink-0 rounded-sm p-1 text-destructive opacity-0 transition-opacity duration-150 hover:bg-accent focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring group-hover:opacity-100"
          title={t('deleteSection')}
          aria-label={t('deleteSection')}
        >
          <Trash2 className="h-4 w-4" />
        </button>
      )}
    </div>
  );
};
