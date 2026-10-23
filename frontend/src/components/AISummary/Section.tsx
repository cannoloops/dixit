'use client';

import { Section as SectionType, Block } from '@/types';
import { BlockComponent } from './Block';
import { EditableTitle } from '../EditableTitle';
import { useState } from 'react';
import { t } from '@/i18n/meeting';

interface SectionProps {
  section: SectionType;
  sectionKey: string;
  selectedBlocks: string[];
  onBlockTypeChange: (blockId: string, type: Block['type']) => void;
  onBlockChange: (blockId: string, content: string) => void;
  onBlockMouseDown: (blockId: string, e: React.MouseEvent<HTMLDivElement>) => void;
  onBlockMouseEnter: (blockId: string) => void;
  onBlockMouseUp: (blockId: string, e: React.MouseEvent<HTMLDivElement>) => void;
  onKeyDown: (e: React.KeyboardEvent, blockId: string, newBlockContent?: string) => void;
  onTitleChange?: (sectionKey: string, title: string) => void;
  onSectionDelete?: (sectionKey: string) => void;
  onBlockDelete: (blockId: string, mergeContent?: string) => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onBlockNavigate?: (blockId: string, direction: 'up' | 'down', cursorPosition: number) => void;
  onCreateNewBlock?: (blockId: string, newBlockContent: string, blockType: Block['type']) => void;
}

export const Section: React.FC<SectionProps> = ({
  section,
  sectionKey,
  selectedBlocks,
  onBlockTypeChange,
  onBlockChange,
  onBlockMouseDown,
  onBlockMouseEnter,
  onBlockMouseUp,
  onKeyDown,
  onTitleChange,
  onSectionDelete,
  onBlockDelete,
  onContextMenu,
  onBlockNavigate,
  onCreateNewBlock,
}) => {
  const [isEditingTitle, setIsEditingTitle] = useState(false);

  const handleTitleChange = (newTitle: string) => {
    if (onTitleChange) {
      onTitleChange(sectionKey, newTitle);
    }
  };

  return (
    <section className="mb-6">
      <div className="mb-2 flex items-center justify-between gap-2">
        <EditableTitle
          title={section.title}
          isEditing={isEditingTitle}
          onStartEditing={() => setIsEditingTitle(true)}
          onFinishEditing={() => setIsEditingTitle(false)}
          onChange={handleTitleChange}
          onDelete={onSectionDelete ? () => onSectionDelete(sectionKey) : undefined}
        />
        {onSectionDelete && (
          <button
            type="button"
            onClick={() => onSectionDelete(sectionKey)}
            className="rounded-sm px-1.5 text-xs text-muted-foreground transition-colors duration-150 hover:bg-accent hover:text-destructive focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            {t('delete')}
          </button>
        )}
      </div>
      <div>
        {(section.blocks || []).map((block) => (
          <BlockComponent
            key={block.id}
            block={block}
            isSelected={selectedBlocks.includes(block.id)}
            onTypeChange={(type) => onBlockTypeChange(block.id, type)}
            onChange={(content) => onBlockChange(block.id, content)}
            onMouseDown={(e) => onBlockMouseDown(block.id, e)}
            onMouseEnter={() => onBlockMouseEnter(block.id)}
            onMouseUp={(e) => onBlockMouseUp(block.id, e)}
            onKeyDown={(e) => {
              const newBlockContent = (e.currentTarget as HTMLTextAreaElement).dataset.newBlockContent;
              onKeyDown(e, block.id, newBlockContent);
            }}
            onDelete={() => {
              const textarea = document.querySelector(`[data-block-id="${block.id}"]`) as HTMLTextAreaElement;
              const mergeContent = textarea?.dataset.mergeContent;
              onBlockDelete(block.id, mergeContent);
            }}
            onContextMenu={onContextMenu}
            onNavigate={onBlockNavigate ?
              (direction, cursorPosition) => onBlockNavigate(block.id, direction, cursorPosition)
              : undefined}
            onCreateNewBlock={onCreateNewBlock}
          />
        ))}
      </div>
    </section>
  );
};
