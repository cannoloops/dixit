import { makeT } from '.';

// Shell area: rail (sidebar archive), About, updates, permission and analytics notices, shell toasts.
// Multi-line values are lists: split them on '\n'.
export const t = makeT({
  it: {
    // Rail
    home: 'Pagina iniziale',
    searchPlaceholder: 'Cerca nelle riunioni',
    clearSearch: 'Cancella ricerca',
    meetings: 'Riunioni',
    searching: 'Ricerca…',
    noMeetings: 'Nessuna riunione. Quando registri una chiamata, la trovi qui.',
    noResults: 'Nessun risultato per «{query}».',
    today: 'Oggi',
    yesterday: 'Ieri',
    newRecording: 'Nuova registrazione',
    openLive: 'Apri la registrazione in corso',
    renameMeeting: 'Rinomina riunione',
    deleteMeeting: 'Elimina riunione',
    startRecording: 'Avvia registrazione',
    recordingInProgress: 'Registrazione in corso',
    importAudio: 'Importa audio',
    settings: 'Impostazioni',
    info: 'Info',
    collapseSidebar: 'Comprimi barra laterale',
    expandSidebar: 'Espandi barra laterale',
    meetingTitle: 'Titolo',
    meetingTitlePlaceholder: 'Titolo della riunione',
    titleEmpty: 'Il titolo non può essere vuoto',
    titleUpdated: 'Titolo aggiornato',
    titleUpdateFailed: 'Impossibile aggiornare il titolo',
    meetingDeleted: 'Riunione eliminata',
    meetingDeletedDesc: "Tolta dall'archivio di Dixit. I file su disco non vengono toccati.",
    deleteFailed: 'Impossibile eliminare la riunione',
    deleteConfirm:
      "Trascrizione e riassunto di questa riunione verranno tolti dall'archivio di Dixit. I file su disco restano. L'operazione non si può annullare.",
    confirmDeleteTitle: 'Eliminare la riunione?',
    delete: 'Elimina',
    unknownError: 'Errore sconosciuto',
    summaryTimeout: 'Il riassunto ha superato i 15 minuti. Riprova o controlla il modello scelto.',
    dialog: 'Finestra',

    // About
    aboutTitle: 'Informazioni su Dixit',
    version: 'Versione {version}',
    aboutDescription: 'Registra e trascrive le tue riunioni in locale: audio e testo restano sul Mac.',
    credit: 'Basato su Meetily (open source, MIT)',
    checkUpdates: 'Cerca aggiornamenti',
    checking: 'Ricerca in corso…',
    upToDate: "Hai già l'ultima versione",
    updateCheckFailed: 'Impossibile cercare aggiornamenti: {error}',

    // Updates
    updateAvailable: 'Aggiornamento disponibile',
    versionAvailable: 'È disponibile la versione {version}',
    viewDetails: 'Dettagli',
    downloadingUpdate: "Download dell'aggiornamento",
    downloadingDesc: "Scarico l'ultima versione…",
    updateFailed: 'Aggiornamento non riuscito',
    updateFailedDesc: "Si è verificato un errore durante l'aggiornamento.",
    currentVersion: 'Versione attuale',
    newVersion: 'Nuova versione',
    releaseDate: 'Data di rilascio',
    percentComplete: '{percent}% completato',
    restartNote: "Dixit si riavvia da solo dopo l'installazione.",
    later: 'Più tardi',
    downloadInstall: 'Scarica e installa',
    updateGone: "L'aggiornamento non è più disponibile.",
    updatePrepareFailed: "Impossibile preparare l'aggiornamento: {error}",
    updateInstalled: 'Aggiornamento installato. Dixit si riavvia…',
    updateInstallFailed: 'Download o installazione non riusciti.',
    updateFailedToast: 'Aggiornamento non riuscito: {error}',

    // Shell toasts
    setupFirst: 'Completa prima la configurazione',
    setupFirstDesc: 'Termina la configurazione iniziale, poi potrai registrare.',
    importOff: "L'importazione audio è disattivata",
    importOffDesc: 'Attivala in Impostazioni, nella sezione Beta.',
    dropAudio: 'Trascina un file audio',
    supportedFormats: 'Formati supportati: {formats}',

    // Recording notice (compliance)
    recordingNotice: 'Avviso di registrazione',
    informParticipants: 'Avvisa i partecipanti che stai registrando.',
    usCompliance: 'Richiesto dalla normativa USA',
    done: 'Fatto',

    // Permissions
    permissionsRequired: 'Servono dei permessi',
    micPermissionRequired: 'Serve il permesso per il microfono',
    systemAudioPermissionRequired: "Serve il permesso per l'audio di sistema",
    openMicSettings: 'Apri impostazioni microfono',
    openScreenSettings: 'Apri impostazioni registrazione schermo',
    recheck: 'Ricontrolla',
    micMissing: 'Dixit ha bisogno del microfono per registrare le riunioni. Non è stato trovato nessun microfono.',
    pleaseCheck: 'Controlla che:',
    micChecks:
      "il microfono sia collegato e acceso\nil permesso per il microfono sia concesso in Impostazioni di Sistema\nnessun'altra app usi il microfono in modo esclusivo",
    systemAudioMissing:
      "L'audio di sistema non è disponibile. Puoi registrare con il microfono, ma l'audio del computer non verrà catturato.",
    systemAudioMissingToo: "Anche l'audio di sistema non è disponibile.",
    enableSystemAudio: "Per attivare l'audio di sistema su macOS:",
    systemAudioSteps:
      "installa un dispositivo audio virtuale (ad esempio BlackHole 2ch)\nconcedi a Dixit il permesso di registrazione dello schermo\nconfigura l'instradamento audio in Configurazione MIDI Audio",

    // Bluetooth playback
    bluetoothTitle: 'Riproduzione tramite Bluetooth',
    bluetoothBody:
      'Stai ascoltando con {device}. Con i dispositivi Bluetooth le registrazioni possono suonare distorte o accelerate: per riascoltarle usa gli altoparlanti del computer o cuffie con filo.',
    dismiss: 'Chiudi avviso',

    // Developer console
    devConsole: 'Console sviluppatore',
    toggleConsole: 'Mostra o nascondi',
    devConsoleDesc:
      'Mostra o nasconde la finestra della console. Su Windows è la finestra di console, su macOS apre il Terminale con i log di Dixit.',

    // Analytics
    analyticsTitle: "Statistiche d'uso",
    analyticsDesc:
      'Spente per impostazione predefinita. Se le attivi invii dati anonimi su uso e prestazioni, mai contenuti personali.',
    analyticsToggle: "Attiva statistiche d'uso",
    updating: 'Aggiornamento…',
    userIdTitle: 'Il tuo ID utente',
    userIdHint: 'Indicalo quando segnali un problema, così possiamo trovare i tuoi log.',
    copy: 'Copia',
    copied: 'Copiato',
    copyUserId: 'Copia ID utente',
    analyticsLocal: 'Riunioni, trascrizioni e registrazioni restano private, sul tuo Mac.',
    privacyPolicy: 'Informativa privacy di Meetily',
    adTitle: 'Cosa raccolgono le statistiche',
    adPrivacyBody:
      'Le statistiche sono spente per impostazione predefinita. Se le attivi raccogliamo solo dati d\'uso anonimi: mai contenuti delle riunioni, nomi, percorsi di file o dati personali.',
    adCollected: 'Dati raccolti quando sono attive',
    adModelsTitle: 'Preferenze dei modelli',
    adModels:
      'Modello di trascrizione (es. «Whisper large-v3», «Parakeet»)\nModello per i riassunti (es. «Llama 3.2», «Claude Sonnet»)\nFornitore del modello (es. «Locale», «Ollama», «OpenRouter»)',
    adModelsWhy: 'Serve a capire quali modelli si usano di più.',
    adMetricsTitle: 'Metriche anonime delle riunioni',
    adMetrics:
      'Durata della registrazione (es. «125 secondi»)\nDurata delle pause (es. «5 secondi»)\nNumero di segmenti di trascrizione\nNumero di blocchi audio elaborati',
    adMetricsWhy: 'Serve a migliorare le prestazioni e capire come si usa Dixit.',
    adDevicesTitle: 'Tipi di dispositivo (non i nomi)',
    adDevices:
      'Tipo di microfono: «Bluetooth», «Wired» o «Unknown»\nTipo di audio di sistema: «Bluetooth», «Wired» o «Unknown»',
    adDevicesWhy: 'Serve alla compatibilità. I nomi dei dispositivi non vengono raccolti.',
    adUsageTitle: "Uso dell'app",
    adUsage:
      "Avvio e chiusura dell'app\nDurata della sessione\nFunzioni usate (es. «impostazioni modificate»)\nErrori (per correggere i bug)",
    adUsageWhy: "Serve a migliorare l'esperienza d'uso.",
    adPlatformTitle: 'Piattaforma',
    adPlatform:
      "Sistema operativo (es. «macOS», «Windows»)\nVersione dell'app (inclusa in ogni evento)\nArchitettura (es. «x86_64», «aarch64»)",
    adPlatformWhy: 'Serve a decidere quali piattaforme supportare.',
    adNotCollectedTitle: 'Cosa non raccogliamo',
    adNotCollected:
      'Nomi o titoli delle riunioni\nNomi e percorsi di file o cartelle\nTrascrizioni o contenuti delle riunioni\nRegistrazioni audio\nNomi dei dispositivi (solo il tipo)\nDati personali\nQualsiasi dato che ti identifichi',
    adExample: 'Esempio di evento',
    adKeep: 'Lasciale attive',
    adDisable: 'Disattiva statistiche',
  },
  en: {
    // Rail
    home: 'Home',
    searchPlaceholder: 'Search meetings',
    clearSearch: 'Clear search',
    meetings: 'Meetings',
    searching: 'Searching…',
    noMeetings: 'No meetings yet. Recorded calls show up here.',
    noResults: 'No results for “{query}”.',
    today: 'Today',
    yesterday: 'Yesterday',
    newRecording: 'New recording',
    openLive: 'Open the current recording',
    renameMeeting: 'Rename meeting',
    deleteMeeting: 'Delete meeting',
    startRecording: 'Start recording',
    recordingInProgress: 'Recording in progress',
    importAudio: 'Import audio',
    settings: 'Settings',
    info: 'About',
    collapseSidebar: 'Collapse sidebar',
    expandSidebar: 'Expand sidebar',
    meetingTitle: 'Title',
    meetingTitlePlaceholder: 'Meeting title',
    titleEmpty: 'The title cannot be empty',
    titleUpdated: 'Title updated',
    titleUpdateFailed: 'Could not update the title',
    meetingDeleted: 'Meeting deleted',
    meetingDeletedDesc: 'Removed from the Dixit archive. Files on disk are not touched.',
    deleteFailed: 'Could not delete the meeting',
    deleteConfirm:
      "This meeting's transcript and summary will be removed from the Dixit archive. Files on disk stay. This cannot be undone.",
    confirmDeleteTitle: 'Delete this meeting?',
    delete: 'Delete',
    unknownError: 'Unknown error',
    summaryTimeout: 'The summary took longer than 15 minutes. Try again or check the selected model.',
    dialog: 'Dialog',

    // About
    aboutTitle: 'About Dixit',
    version: 'Version {version}',
    aboutDescription: 'Records and transcribes your meetings locally: audio and text stay on your Mac.',
    credit: 'Based on Meetily (open source, MIT)',
    checkUpdates: 'Check for updates',
    checking: 'Checking…',
    upToDate: 'You have the latest version',
    updateCheckFailed: 'Could not check for updates: {error}',

    // Updates
    updateAvailable: 'Update available',
    versionAvailable: 'Version {version} is available',
    viewDetails: 'View details',
    downloadingUpdate: 'Downloading update',
    downloadingDesc: 'Downloading the latest version…',
    updateFailed: 'Update failed',
    updateFailedDesc: 'Something went wrong while updating.',
    currentVersion: 'Current version',
    newVersion: 'New version',
    releaseDate: 'Release date',
    percentComplete: '{percent}% complete',
    restartNote: 'Dixit restarts automatically after installing.',
    later: 'Later',
    downloadInstall: 'Download and install',
    updateGone: 'The update is no longer available.',
    updatePrepareFailed: 'Could not prepare the update: {error}',
    updateInstalled: 'Update installed. Dixit is restarting…',
    updateInstallFailed: 'Download or installation failed.',
    updateFailedToast: 'Update failed: {error}',

    // Shell toasts
    setupFirst: 'Finish setup first',
    setupFirstDesc: 'Complete the initial setup, then you can record.',
    importOff: 'Audio import is off',
    importOffDesc: 'Turn it on in Settings, in the Beta section.',
    dropAudio: 'Drop an audio file',
    supportedFormats: 'Supported formats: {formats}',

    // Recording notice (compliance)
    recordingNotice: 'Recording notice',
    informParticipants: 'Let participants know you are recording.',
    usCompliance: 'Required for US compliance',
    done: 'Done',

    // Permissions
    permissionsRequired: 'Permissions required',
    micPermissionRequired: 'Microphone permission required',
    systemAudioPermissionRequired: 'System audio permission required',
    openMicSettings: 'Open microphone settings',
    openScreenSettings: 'Open screen recording settings',
    recheck: 'Check again',
    micMissing: 'Dixit needs your microphone to record meetings. No microphone was found.',
    pleaseCheck: 'Please check that:',
    micChecks:
      'your microphone is connected and turned on\nmicrophone permission is granted in System Settings\nno other app is using the microphone exclusively',
    systemAudioMissing:
      "System audio is not available. You can still record with your microphone, but computer audio won't be captured.",
    systemAudioMissingToo: 'System audio is not available either.',
    enableSystemAudio: 'To enable system audio on macOS:',
    systemAudioSteps:
      'install a virtual audio device (for example BlackHole 2ch)\ngrant Dixit the Screen Recording permission\nset up audio routing in Audio MIDI Setup',

    // Bluetooth playback
    bluetoothTitle: 'Bluetooth playback detected',
    bluetoothBody:
      "You're listening through {device}. Recordings may sound distorted or sped up on Bluetooth devices: for accurate review use your computer speakers or wired headphones.",
    dismiss: 'Dismiss warning',

    // Developer console
    devConsole: 'Developer console',
    toggleConsole: 'Show or hide',
    devConsoleDesc:
      'Shows or hides the console window. On Windows this is the console window; on macOS it opens Terminal with the Dixit logs.',

    // Analytics
    analyticsTitle: 'Usage analytics',
    analyticsDesc:
      'Off by default. If you turn it on you share anonymous usage and performance data, never personal content.',
    analyticsToggle: 'Turn on usage analytics',
    updating: 'Updating…',
    userIdTitle: 'Your user ID',
    userIdHint: 'Share it when reporting an issue so your logs can be found.',
    copy: 'Copy',
    copied: 'Copied',
    copyUserId: 'Copy user ID',
    analyticsLocal: 'Meetings, transcripts and recordings stay private, on your Mac.',
    privacyPolicy: 'Meetily privacy policy',
    adTitle: 'What analytics collects',
    adPrivacyBody:
      'Analytics is off by default. If you turn it on we collect anonymous usage data only: never meeting content, names, file paths or personal information.',
    adCollected: 'Data collected when it is on',
    adModelsTitle: 'Model preferences',
    adModels:
      'Transcription model (e.g. “Whisper large-v3”, “Parakeet”)\nSummary model (e.g. “Llama 3.2”, “Claude Sonnet”)\nModel provider (e.g. “Local”, “Ollama”, “OpenRouter”)',
    adModelsWhy: 'Helps us understand which models people prefer.',
    adMetricsTitle: 'Anonymous meeting metrics',
    adMetrics:
      'Recording duration (e.g. “125 seconds”)\nPause duration (e.g. “5 seconds”)\nNumber of transcript segments\nNumber of audio chunks processed',
    adMetricsWhy: 'Helps us improve performance and understand usage.',
    adDevicesTitle: 'Device types (not names)',
    adDevices:
      'Microphone type: “Bluetooth”, “Wired” or “Unknown”\nSystem audio type: “Bluetooth”, “Wired” or “Unknown”',
    adDevicesWhy: 'Helps with compatibility. Device names are never collected.',
    adUsageTitle: 'App usage',
    adUsage:
      'App started and stopped\nSession duration\nFeatures used (e.g. “settings changed”)\nErrors (to fix bugs)',
    adUsageWhy: 'Helps us improve the experience.',
    adPlatformTitle: 'Platform',
    adPlatform:
      'Operating system (e.g. “macOS”, “Windows”)\nApp version (included in every event)\nArchitecture (e.g. “x86_64”, “aarch64”)',
    adPlatformWhy: 'Helps us decide which platforms to support.',
    adNotCollectedTitle: 'What we never collect',
    adNotCollected:
      'Meeting names or titles\nFile names, file paths or meeting folders\nTranscripts or meeting content\nAudio recordings\nDevice names (only the type)\nPersonal information\nAny identifying data',
    adExample: 'Example event',
    adKeep: 'Keep it on',
    adDisable: 'Turn off analytics',
  },
});
