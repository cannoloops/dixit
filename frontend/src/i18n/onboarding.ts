import { makeT } from '.';

// Onboarding copy: welcome, setup overview, model downloads, macOS permissions.
export const t = makeT({
  it: {
    // Shared
    back: 'Indietro',
    next: 'Avanti',
    continue: 'Continua',
    progress: 'Avanzamento della configurazione',
    stepOf: 'Passo {current} di {total}',
    goToStep: 'Vai al passo {step}',

    // Welcome
    welcomeTagline:
      'Si accende da solo quando inizia una call su Teams, Zoom o Meet, trascrive sul Mac e scrive chi ha detto cosa.',
    welcomeNote: 'I modelli sono pronti: ora Dixit chiede solo tre permessi di macOS.',
    welcomeStart: 'Inizia',

    // Setup overview
    overviewTitle: 'Prima configurazione',
    overviewDescription:
      'Trascrizione e riassunti girano sul tuo Mac con due modelli di intelligenza artificiale già scaricati sul tuo Mac.',
    transcriptionEngine: 'Motore di trascrizione',
    transcriptionPurpose: 'Trasforma le voci della call in testo.',
    summaryEngine: 'Motore di riassunto',
    summaryPurpose: 'Scrive il riassunto della riunione.',
    summaryInfo:
      'Per i riassunti puoi anche scegliere un servizio esterno come OpenAI, Claude o Ollama, dalle impostazioni avanzate.',
    summaryInfoLabel: 'Altre opzioni per i riassunti',

    // Base models (first launch)
    modelsTitle: 'Scarico i modelli',
    modelsDescription:
      'Trascrizione e riassunti girano sul tuo Mac. Al primo avvio Dixit scarica i suoi modelli di intelligenza artificiale, circa {size}: tienilo aperto e collegato a internet.',
    modelsResume: 'Se il download si interrompe, riparte da dove era arrivato.',

    // Downloads
    downloadsTitle: 'Download dei modelli',
    downloadsDescription: 'Puoi iniziare a usare Dixit appena il motore di trascrizione è scaricato.',
    downloading: 'Download in corso',
    statusWaiting: 'In attesa',
    statusReady: 'Pronto',
    statusFailed: 'Non riuscito',
    statusCancelled: 'Annullato',
    downloadFailed: 'Download non riuscito.',
    downloadFailedRemedy: 'Controlla la connessione e riprova.',
    downloadCancelled: 'Download annullato.',
    downloadCancelledRemedy: 'Riprova per ripartire da capo.',
    tryAgain: 'Riprova',
    retryFailed: 'Nuovo tentativo non riuscito',
    retryFailedSummary: 'Nuovo tentativo non riuscito per il motore di riassunto',
    summaryNotReady: 'Il modello per i riassunti non è ancora pronto.',
    engineRequiredTitle: 'Serve il motore di trascrizione',
    engineRequiredBody: 'Riprova il download prima di continuare.',
    backgroundTitle: 'I download proseguono in background',
    backgroundBody:
      'Puoi iniziare a usare Dixit. La registrazione sarà disponibile appena il riconoscimento vocale è pronto.',
    completeFailedTitle: 'Impossibile completare la configurazione',
    completeFailedBody: 'Riprova.',
    continueWhileDownloading: 'Puoi continuare: il download del riassunto prosegue in background.',

    // Permissions
    permissionsTitle: 'Permessi',
    permissionsDescription: 'Dixit usa tre permessi di macOS per registrare le call e capire chi parla.',
    micName: 'Microfono',
    micPurpose: 'Per registrare la tua voce.',
    micDenied: 'Accesso negato. Attivalo in Impostazioni di Sistema > Privacy e sicurezza > Microfono.',
    systemName: 'Registrazione schermo e audio di sistema',
    systemPurpose: 'Per sentire gli altri partecipanti della call.',
    systemDenied:
      'Accesso negato. Attivalo in Impostazioni di Sistema > Privacy e sicurezza > Registrazione schermo e audio di sistema.',
    accessibilityName: 'Accessibilità',
    accessibilityPurpose: 'Per riconoscere l’inizio di una call e leggere i nomi da Teams.',
    accessibilityDenied:
      'Non ancora concesso. Attivalo in Impostazioni di Sistema > Privacy e sicurezza > Accessibilità.',
    deniedDefault: 'Accesso negato. Attivalo in Impostazioni di Sistema, poi torna qui.',
    grant: 'Concedi',
    granted: 'Concesso',
    checking: 'Verifica…',
    openSettings: 'Apri Impostazioni',
    openSettingsFailed: 'Impossibile aprire Impostazioni di Sistema',
    displayNameLabel: 'Il tuo nome (come appare in Teams, es. Rossi Mario)',
    displayNamePlaceholder: 'Cognome Nome',
    displayNameHint:
      'In Teams imposta la lingua parlata su italiano, così sottotitoli e trascrizione riportano bene i nomi.',
    permissionsNote: 'Senza i permessi la registrazione non funziona. Puoi concederli più tardi dalle impostazioni.',
    finishSetup: 'Termina configurazione',
    doLater: 'Lo farò dopo',
  },
  en: {
    // Shared
    back: 'Back',
    next: 'Next',
    continue: 'Continue',
    progress: 'Setup progress',
    stepOf: 'Step {current} of {total}',
    goToStep: 'Go to step {step}',

    // Welcome
    welcomeTagline:
      'Starts by itself when a call begins on Teams, Zoom or Meet, transcribes on your Mac and writes down who said what.',
    welcomeNote: 'The models are ready: now Dixit only asks for three macOS permissions.',
    welcomeStart: 'Get started',

    // Setup overview
    overviewTitle: 'First-time setup',
    overviewDescription:
      'Transcription and summaries run on your Mac with two artificial intelligence models already downloaded to your Mac.',
    transcriptionEngine: 'Transcription engine',
    transcriptionPurpose: 'Turns the voices on the call into text.',
    summaryEngine: 'Summary engine',
    summaryPurpose: 'Writes the meeting summary.',
    summaryInfo:
      'For summaries you can also choose an external provider such as OpenAI, Claude or Ollama in advanced settings.',
    summaryInfoLabel: 'Other summary options',

    // Base models (first launch)
    modelsTitle: 'Downloading the models',
    modelsDescription:
      'Transcription and summaries run on your Mac. On first launch Dixit downloads its artificial intelligence models, about {size}: keep it open and online.',
    modelsResume: 'If the download stops, it picks up where it left off.',

    // Downloads
    downloadsTitle: 'Downloading models',
    downloadsDescription: 'You can start using Dixit as soon as the transcription engine is downloaded.',
    downloading: 'Downloading',
    statusWaiting: 'Waiting',
    statusReady: 'Ready',
    statusFailed: 'Failed',
    statusCancelled: 'Cancelled',
    downloadFailed: 'Download failed.',
    downloadFailedRemedy: 'Check your connection and try again.',
    downloadCancelled: 'Download cancelled.',
    downloadCancelledRemedy: 'Try again to start over.',
    tryAgain: 'Try again',
    retryFailed: 'Retry failed',
    retryFailedSummary: 'Retry failed for the summary engine',
    summaryNotReady: 'The summary model is not ready yet.',
    engineRequiredTitle: 'Transcription engine required',
    engineRequiredBody: 'Retry the download before continuing.',
    backgroundTitle: 'Downloads will continue in the background',
    backgroundBody: 'You can start using Dixit. Recording will be available once speech recognition is ready.',
    completeFailedTitle: 'Could not complete setup',
    completeFailedBody: 'Please try again.',
    continueWhileDownloading: 'You can continue: the summary download keeps running in the background.',

    // Permissions
    permissionsTitle: 'Permissions',
    permissionsDescription: 'Dixit uses three macOS permissions to record calls and tell who is speaking.',
    micName: 'Microphone',
    micPurpose: 'To record your voice.',
    micDenied: 'Access denied. Turn it on in System Settings > Privacy & Security > Microphone.',
    systemName: 'Screen & system audio recording',
    systemPurpose: 'To hear the other people on the call.',
    systemDenied:
      'Access denied. Turn it on in System Settings > Privacy & Security > Screen & System Audio Recording.',
    accessibilityName: 'Accessibility',
    accessibilityPurpose: 'To detect when a call starts and read names from Teams.',
    accessibilityDenied: 'Not granted yet. Turn it on in System Settings > Privacy & Security > Accessibility.',
    deniedDefault: 'Access denied. Turn it on in System Settings, then come back here.',
    grant: 'Grant',
    granted: 'Granted',
    checking: 'Checking…',
    openSettings: 'Open Settings',
    openSettingsFailed: 'Could not open System Settings',
    displayNameLabel: 'Your name (as it appears in Teams, e.g. Rossi Mario)',
    displayNamePlaceholder: 'Last name First name',
    displayNameHint:
      'In Teams, set the spoken language to Italian so captions and transcription get names right.',
    permissionsNote: "Recording won't work without permissions. You can grant them later in settings.",
    finishSetup: 'Finish setup',
    doLater: "I'll do this later",
  },
});
