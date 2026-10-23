"use client"

import { Switch } from "./ui/switch"
import { useConfig } from "@/contexts/ConfigContext"
import { BetaFeatureKey } from "@/types/betaFeatures"
import { SettingsSection, SettingRow } from "@/components/molecules/form-components/setting-row"
import { t } from "@/i18n/settings"

export function BetaSettings() {
  const { betaFeatures, toggleBetaFeature } = useConfig();

  // Define feature order for display (allows custom ordering)
  const featureOrder: BetaFeatureKey[] = ['importAndRetranscribe'];

  // User-facing copy for each feature, in the UI language
  const featureCopy: Record<BetaFeatureKey, { name: string; description: string }> = {
    importAndRetranscribe: { name: t('betaImportName'), description: t('betaImportDesc') },
  };

  return (
    <SettingsSection title={t('betaSection')} description={t('betaIntro')}>
      {/* Dynamic Feature Toggles - Automatically renders all features */}
      {featureOrder.map((featureKey) => (
        <SettingRow
          key={featureKey}
          htmlFor={`beta-${featureKey}`}
          label={featureCopy[featureKey].name}
          description={featureCopy[featureKey].description}
        >
          <span className="rounded-sm border border-border px-1.5 py-0.5 text-xs text-muted-foreground">
            {t('betaTag')}
          </span>
          <Switch
            id={`beta-${featureKey}`}
            checked={betaFeatures[featureKey]}
            onCheckedChange={(checked) => toggleBetaFeature(featureKey, checked)}
          />
        </SettingRow>
      ))}
    </SettingsSection>
  );
}
