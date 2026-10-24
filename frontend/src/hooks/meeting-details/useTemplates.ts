import { useState, useEffect, useCallback } from 'react';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import Analytics from '@/lib/analytics';
import { t } from '@/i18n/meeting';
import { lang } from '@/i18n';

export function useTemplates() {
  const [availableTemplates, setAvailableTemplates] = useState<Array<{
    id: string;
    name: string;
    description: string;
  }>>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string>('auto');

  // Fetch available templates on mount
  useEffect(() => {
    const fetchTemplates = async () => {
      try {
        const templates = await invokeTauri('api_list_templates') as Array<{
          id: string;
          name: string;
          description: string;
          name_it?: string | null;
          description_it?: string | null;
        }>;
        console.log('Available templates:', templates);
        // Dixit: "auto" lets the backend pick the template from the call type.
        setAvailableTemplates([
          { id: 'auto', name: t('templateAuto'), description: t('templateAutoDesc') },
          ...templates.map(({ id, name, description, name_it, description_it }) => ({
            id,
            name: (lang === 'it' && name_it) || name,
            description: (lang === 'it' && description_it) || description,
          })),
        ]);
      } catch (error) {
        console.error('Failed to fetch templates:', error);
      }
    };
    fetchTemplates();
  }, []);

  // Handle template selection
  const handleTemplateSelection = useCallback((templateId: string, templateName: string) => {
    setSelectedTemplate(templateId);
    toast.success(t('templateSelected'), {
      description: t('templateSelectedDesc', { name: templateName }),
    });
    Analytics.trackFeatureUsed('template_selected');
  }, []);

  return {
    availableTemplates,
    selectedTemplate,
    setSelectedTemplate,
    handleTemplateSelection,
  };
}
