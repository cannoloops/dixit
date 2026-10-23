// Dixit UI language: Italian or English, from the system language (anything not Italian → English).
// ponytail: no i18n library; each area keeps its own message file built with makeT (see app.ts).
export type Lang = 'it' | 'en';

export const lang: Lang =
  typeof navigator === 'undefined' || navigator.language.toLowerCase().startsWith('it') ? 'it' : 'en';

type Vars = Record<string, string | number>;

/** Builds a typed `t(key, vars)` for one area; `en` must define every key of `it`. */
export function makeT<K extends string>(messages: { it: Record<K, string>; en: Record<K, string> }) {
  return (key: K, vars?: Vars): string => {
    let text = messages[lang][key] ?? messages.it[key] ?? key;
    if (vars) for (const [k, v] of Object.entries(vars)) text = text.split(`{${k}}`).join(String(v));
    return text;
  };
}

/** Locale for dates and numbers (Intl / toLocaleString). */
export const locale = lang === 'it' ? 'it-IT' : 'en-GB';
