interface OnboardingSummaryModelStatusInput {
  selectedModel: string;
  recommendedModel: string;
  selectedModelReady: boolean;
}

interface OnboardingSummaryModelStatus {
  selectedSummaryModel: string;
  summaryModelDownloaded: boolean;
}

const SUMMARY_MODEL_SIZES_MB: Record<string, number> = {
  'qwen3.5:2b': 1221,
  'qwen3.5:4b': 2614,
  'gemma3:1b': 1019,
  'gemma3:4b': 2374,
};

export function resolveOnboardingSummaryModelStatus({
  selectedModel,
  recommendedModel,
  selectedModelReady,
}: OnboardingSummaryModelStatusInput): OnboardingSummaryModelStatus {
  const selectedSummaryModel = selectedModel || recommendedModel;

  return {
    selectedSummaryModel,
    summaryModelDownloaded: Boolean(selectedSummaryModel && selectedModelReady),
  };
}

export function getSummaryModelSizeMb(model: string): number {
  return SUMMARY_MODEL_SIZES_MB[model] ?? 0;
}

export function getDownloadTotalMb(totalMb: number | null | undefined, model: string): number {
  return totalMb || getSummaryModelSizeMb(model);
}

export function formatSummaryModelSizeLabelFromMb(
  sizeMb: number,
  formatGiB: (gib: number) => string = (gib) => gib.toFixed(1)
): string {
  if (sizeMb === 0) {
    return '';
  }

  if (sizeMb >= 1024) {
    return `~${formatGiB(sizeMb / 1024)} GiB`;
  }

  return `~${sizeMb} MiB`;
}

/** `formatGiB` lets the UI print the decimal in the user's locale (1,2 instead of 1.2). */
export function getSummaryModelSizeLabel(model: string, formatGiB?: (gib: number) => string): string {
  return formatSummaryModelSizeLabelFromMb(getSummaryModelSizeMb(model), formatGiB);
}
