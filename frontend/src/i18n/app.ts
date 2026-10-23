import { makeT } from '.';

// Shared words used across areas. Area-specific copy lives in its own file (sidebar.ts, settings.ts, ...).
export const t = makeT({
  it: {
    recording: 'In registrazione',
    saved: 'Salvata',
    verifyNames: 'Nomi da verificare',
    cancelled: 'Annullata',
    processing: 'In elaborazione',
    me: 'Tu',
    others: 'Altri',
    speaker: 'Parlante {letter}',
    cancel: 'Annulla',
    save: 'Salva',
    close: 'Chiudi',
  },
  en: {
    recording: 'Recording',
    saved: 'Saved',
    verifyNames: 'Check names',
    cancelled: 'Cancelled',
    processing: 'Processing',
    me: 'You',
    others: 'Others',
    speaker: 'Speaker {letter}',
    cancel: 'Cancel',
    save: 'Save',
    close: 'Close',
  },
});
