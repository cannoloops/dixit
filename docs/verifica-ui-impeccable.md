# Verifica UI con impeccable (audit e critique)

Data: 27 novembre 2026. Branch: `claude/ui-impeccable-c10408`, partito da `develop` (0.3.0). Skill: `impeccable` 4.5.0.

## Come è stata fatta

- **Contesto:** `impeccable context` ha caricato PRODUCT.md e DESIGN.md senza segnalare `CONTEXT_STALE`. La piattaforma risolta è `web` (webview Tauri), quindi la skill usa l'audit web e non la variante native.
- **Critique:** `Method: dual-agent (A: design review in un sotto-agente isolato · B: detector in un sotto-agente separato)`. Altri due sotto-agenti hanno fatto l'audit tecnico: uno su accessibilità, tastiera, prestazioni e theming, l'altro su i18n, errori e caricamenti. Ho verificato nel codice ogni P1 prima di classificarlo.
- **Detector:** `impeccable detect` su `frontend/src` e `frontend/public` (200 file): 0 segnalazioni. Nel browser l'overlay `detect.js` sulla home mockata ha trovato 3 pattern: la transizione di `width` della sidebar (P3 sotto), il bordo sottile con ombra larga della pillola di registrazione (è `shadow-float` di DESIGN.md, falso positivo) e un terzo senza dettaglio in console.
- **Cosa ho visto davvero.** La sandbox Tauri (`com.vitomanuguerra.trascrivi.dev`) era già in esecuzione dal checkout principale su un altro branch, sulla porta 3118 e con i dati reali. Una seconda istanza avrebbe toccato le stesse cartelle in `~/Documents/Dixit`. Ho quindi avviato il Next del worktree sulla porta 3119 nel browser integrato, con un backend Tauri finto e dati inventati (riunioni, parlanti, riassunto). Le superfici viste, tutte a 1280×800:
  - **in chiaro e in scuro:** vista riunione (banda parlanti, trascrizione, «Riassunto automatico in arrivo»), sidebar con «Riassunti in corso», registrazione live con il campo note e la nota nella trascrizione, banner della call;
  - **solo in scuro:** Impostazioni (Generali, Automazione › Agenti, Avanzate › Modalità esperto spenta), Info, schermata iniziale dell'onboarding (senza backend);
  - **solo in chiaro:** home vuota, Novità, download dei modelli.

  Ho valutato **solo dal codice**:
  - i passi intermedi dell'onboarding;
  - `UpdateDialog`;
  - il riassunto già generato;
  - i dialoghi Importa e Ritrascrivi;
  - le schede Registrazioni, Trascrizione, Riassunto e Beta;
  - la modalità esperto accesa.

  Non ho verificato il rendering nella webview WebKit vera (font, scrollbar, finestra del banner) né altre dimensioni della finestra.

### Da segnalare a Vito (contesto, non corretto)

PRODUCT.md non è marcato stale, ma contraddice CLAUDE.md in due punti:
- dice «Next.js 14»: l'app è su Next 15;
- dice che la trascrizione nativa di Teams «Dixit starts automatically»: oggi parte solo dal pulsante del banner.

Va aggiornato a mano (o con `/impeccable init`).

## Punteggi

### Audit tecnico

| # | Dimensione | Punti | Problema principale |
|---|---|---|---|
| 1 | Accessibilità | 2 | Contrasto del petrolio in sidebar, barre di avanzamento mute, banda parlanti solo mouse, niente `prefers-reduced-motion` |
| 2 | Prestazioni | 3 | Polling dello stato a 500 ms che ri-renderizza 24 consumatori; trascrizione già virtualizzata |
| 3 | Responsive (finestra desktop) | 3 | Vista solo a 1280×800; dal codice, transizione di `width` e `margin` sulla sidebar |
| 4 | Theming | 3 | Token usati ovunque, chiaro e scuro completi; bordo dei campi a 1,46:1 |
| 5 | Integrità d'implementazione | 3 | Sistema coerente; il rosso è usato anche per errori generici (contro DESIGN.md) |
| **Totale** | | **14/20** | **Buono** |

**Verdetto d'integrità: passa.** L'interfaccia è specifica di Dixit e non intercambiabile. Esempi: i nomi dei parlanti nel loro colore, la banda dei turni, il verde per la propria voce, la nota intercalata con filetto petrolio, la pillola di registrazione. Il design system è applicato con disciplina: un solo pulsante petrolio per schermata, nessuna classe di colore hard-coded, filetti al posto delle card. Le eccezioni sono ai bordi e sono elencate sotto.

### Critique (euristiche di Nielsen, 0-4)

| Superficie | H1 | H2 | H3 | H4 | H5 | H6 | H7 | H8 | H9 | H10 | Totale |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Home e fine registrazione | 3 | 4 | 3 | 3 | 3 | 3 | 3 | 4 | 2 | 4 | 32/40 |
| Sidebar e Riassunti in corso | 2 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 2 | 28/40 |
| Registrazione live e note | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 2 | 29/40 |
| Vista riunione | 3 | 3 | 2 | 3 | 2 | 3 | 3 | 2 | 3 | 2 | 26/40 |
| Onboarding e download modelli | 3 | 3 | 1 | 3 | 2 | 3 | n/a | 4 | 2 | 3 | 24/36 |
| Impostazioni | 3 | 3 | 3 | 4 | 2 | 3 | 3 | 3 | 3 | 3 | 30/40 |
| Banner della call | 4 | 4 | 3 | 3 | 3 | 3 | n/a | 4 | 2 | n/a | 26/32 |
| UpdateDialog e Novità | 3 | 3 | 3 | 3 | 1 | n/a | n/a | 4 | 3 | 3 | 23/32 |

Motivi degli n/a:
- l'onboarding si fa una volta sola, quindi le scorciatoie (H7) non si applicano;
- il banner dura 20 s e non ha scorciatoie né aiuto (H7, H10);
- nei dialoghi di aggiornamento non c'è nulla da ricordare né scorciatoie (H6, H7).

Le superfici stanno tra 65% e 80%: buone, con lacune puntuali.

## Problemi per gravità

### P1 corretti in questo branch

| # | Problema | Dove | Correzione |
|---|---|---|---|
| 1 | In chiaro il testo petrolio in sidebar non reggeva il contrasto AA: 4,21:1 su `rail`, 3,92:1 in hover su `rail-active`. Colpisce «Avvia registrazione», che è il pulsante più importante della sidebar | `globals.css` (`--rail-accent`), `Sidebar/index.tsx` | `--rail-accent` in chiaro da `190 65% 35%` (#1F8093) a `190 65% 30%` (#1B6E7E): 5,39:1 su `rail` e 5,03:1 su `rail-active`. **È l'unico cambio di palette.** Riguarda solo la sidebar in chiaro; `primary` e il modo scuro non cambiano |
| 2 | Con la sidebar chiusa (lo stato di partenza) nulla diceva che si stava registrando: il microfono era solo grigio e disabilitato | `Sidebar/index.tsx` (`renderCollapsed`) | Puntino `rail-live` statico accanto al microfono durante la registrazione. L'aria-label «Registrazione in corso» c'era già |
| 3 | I secondi di Automazione non si potevano digitare: ogni tasto applicava il minimo e salvava. Scrivendo «30» nello stop il «3» diventava 10, poi «0» dava 100 | `AutopilotSettings.tsx` | Il campo `SecondsInput` accetta qualunque testo mentre si scrive, poi applica il minimo e salva all'uscita dal campo o con Invio. Verificato: «30» resta 30 |
| 4 | «Scarica e installa» riavviava Dixit anche durante una registrazione | `UpdateDialog.tsx` | Pulsante disabilitato durante registrazione, stop o trascrizione definitiva, con la riga «Installa a registrazione finita: il riavvio la interromperebbe.» (IT/EN) |
| 5 | Le barre di avanzamento non comunicavano il valore: `Progress` scartava `value`, quindi `aria-valuenow` mancava. Colpisce il download dei modelli, l'aggiornamento e l'onboarding | `ui/progress.tsx` | `value` passato al componente Radix. Verificato nel DOM: `aria-valuenow="0"`, `aria-valuemax="100"` |
| 6 | Cambiare il tipo di call rigenerava il riassunto ignorando il contesto scritto dall'utente, mentre Genera lo usa | `meeting-details/page-content.tsx` | Anche il cambio di tipo passa `customPrompt` |
| 7 | Onboarding: se il completamento falliva, «Fine» e «Più tardi» non facevano nulla e l'errore restava in console. Il doppio clic non era bloccato | `onboarding/steps/PermissionsStep.tsx` | Toast «Impossibile completare la configurazione. Riprova.» (IT/EN) e pulsanti disabilitati durante il salvataggio |
| 8 | Nei dialoghi Importa audio e Ritrascrivi le lingue comparivano in inglese («Auto Detect (Original Language)», «German»…) anche con l'interfaccia in italiano. Import è attivo di default | `ImportAudioDialog.tsx`, `RetranscribeDialog.tsx`, `LanguageSelection.tsx` | Riuso di `displayName()`, che già localizza le lingue nelle Impostazioni |

Controlli fatti dopo le correzioni:
- `npx tsc --noEmit` passa;
- `impeccable detect` sui file cambiati: 0 segnalazioni;
- un giro di conferma nel browser mockato: puntino in rail, campo secondi, `aria-valuenow`, token calcolato.

Ho riprovato a runtime le correzioni 1, 2, 3 e 5. Per la 4, la 6, la 7 e l'8 c'è solo il controllo dei tipi: il brief prevede un solo giro di conferma. Il Rust non è stato toccato.

Note sulle correzioni:
- **Correzione 4.** Resta scoperto il caso in cui una call parte da sola mentre l'aggiornamento si sta già scaricando: `relaunch()` scatta comunque. Proposta P2: controllare `is_recording` subito prima del riavvio.
- **Correzione 1.** In DESIGN.md `button-rail-outline` punta ancora a `colors.primary`, ma in chiaro il petrolio della sidebar ora è più scuro. Va aggiornato DESIGN.md: non l'ho fatto, decide Vito.

### P1 non corretti: serve una decisione di Vito

Sono problemi gravi, ma la correzione cambia il flusso, il copy fattuale o una regola di DESIGN.md.

1. **Il download dei modelli (circa 9 GB) parte da solo al primo avvio.** Avviene senza consenso, senza stima dei tempi, senza pausa né uscita (`BaseModelsGate.tsx:39-43`). Se fallisce, mostra l'errore Rust grezzo in rosso, e senza rete resta solo «Riprova». Per un collega su hotspot è il momento peggiore dell'app. Proposta: una schermata con «Scarico circa 9 GB» e il pulsante «Inizia download», poi velocità e tempo residuo, un errore tradotto e una via d'uscita.
2. **Il bordo dei campi e il binario dello switch spento sono a 1,46:1** in chiaro (1,69:1 in scuro), sotto il 3:1 di WCAG 1.4.11 per i confini dei controlli. È in tensione con i «filetti quieti» di DESIGN.md. Proposta: un token `--control-border` solo per campi e switch (circa `210 8% 58%` in chiaro), lasciando invariati i filetti `--border`.
3. **Il banner della call non si usa da tastiera né con VoiceOver.** Non prende il focus, ed è voluto (principio 2), si chiude da solo dopo 20 s e non annuncia nulla. Il pulsante «Trascrizione Teams» non si può più chiedere dopo. Proposta: l'azione «Annulla registrazione» e la richiesta della trascrizione Teams anche nel menu della barra dei menu, e `role="status"` sul titolo del banner.
4. **Rigenera e il cambio di tipo sovrascrivono il riassunto senza conferma,** anche quando l'utente lo ha modificato a mano. Proposta: chiedere conferma quando il riassunto è stato modificato, oppure tenere la versione precedente con un «Annulla».
5. **Il percorso «Impostazioni › Info › Guida/Novità» non esiste.** Lo citano changelog, CLAUDE.md e GUIDA. Info è un dialogo aperto dalla sidebar. Inoltre la Guida apre GitHub nel browser, quindi serve la rete. Si possono correggere i testi o spostare Info in Impostazioni: è copy fattuale, quindi decide Vito.

### P2: proposte

**Accessibilità e tastiera**
- Nessun `prefers-reduced-motion` nell'app principale (c'è solo nel banner e in `StatusIndicator`). Una regola globale in `globals.css` basta, lasciando lo spinner.
- Banda dei parlanti (`SpeakerBand.tsx:82-91`): è `aria-hidden` e i segmenti sono `span` cliccabili alti 10 px, quindi saltare a un turno si può solo col mouse. Proposta: rendere la legenda dei nomi il controllo da tastiera.
- Il contenitore della trascrizione (`VirtualizedTranscriptView.tsx:434`) non ha nome, ruolo né `tabIndex`: in WebKit non si scorre con le frecce. Il testo live non è annunciato.
- Lo stato di registrazione è annunciato male: la live region nasce già piena. In sidebar, `aria-label` di «Apri live» sovrascrive il testo visibile (WCAG 2.5.3).
- La casella «Includi le mie note» resta senza nome sotto `@md` (label `hidden`). Il campo di modifica nota ha `aria-label="Esc"`, non tradotto.
- La barra del toast di download (`DownloadProgressToast.tsx:96`) non ha `role="progressbar"`.
- In scuro `text-destructive` su `card`/`popover` è a 4,15:1, su `accent` a 3,71:1: sono gli errori dentro i dialoghi. Va deciso insieme al punto sul rosso qui sotto.
- In scuro `text-primary` su `accent` è a 4,04:1: va verificato dove compare.

**Coerenza con DESIGN.md**
- Il rosso è usato per errori generici in una decina di punti, contro la Recording Red Rule: `EmptyStateSummary`, `BaseModelsGate`, `PermissionRow` («Negato»), `UpdateDialog`, `AutopilotSettings`, `MessageToast`, `SummaryPanel`, `ConfidenceIndicator`, `DownloadProgressStep`. Serve una decisione: un token neutro o ambra per gli errori, oppure aggiornare la regola.
- Il banner ha un puntino rosso che pulsa, mentre DESIGN.md vuole il LiveIndicator statico. Rispetta `prefers-reduced-motion`.

**Flussi**
- Fine registrazione:
  - il toast «N segmenti di trascrizione salvati» non dice cosa succede dopo (trascrizione definitiva, nomi, riassunto) e offre «Apri riunione» mentre la vista sta già cambiando;
  - «Completamento della trascrizione…» non mostra la percentuale, che il backend ha già;
  - mancano i plurali («1 segmenti», «+1 altri»).
- Barra del riassunto affollata: fino a 6 controlli (Tipo, Lingua, Modello, Template, Salva, Copia). Tipo e Template sono due strade per lo stesso risultato.
- Sette schede nelle Impostazioni: Trascrizione e Riassunto sono in sola lettura fuori dalla modalità esperto. Proposta: unirle in «Modelli» e portare Beta in Avanzate.
- Note live non modificabili e nessuna conferma di salvataggio nel campo. Stop è a un clic accanto a Pausa.
- Errori Rust in inglese mostrati grezzi in toast e `alert()`, circa 20 punti (`String(error)`). Proposta: un helper `userError(e)`.
- Errori silenziosi:
  - errore nel catch esterno dello stop (`useRecordingStop.ts:415`);
  - pulsanti che aprono le Impostazioni di Sistema in `PermissionWarning`;
  - salvataggio del nome nell'onboarding;
  - annullo del riassunto non riuscito, mostrato come «già concluso».
- Il titolo «+ New Call» compare in inglese nella testata della registrazione live quando il titolo non è ancora arrivato dal backend (`TranscriptPanel.tsx:88`). La sidebar ha già il ripiego tradotto (`Sidebar/index.tsx:470`).
- Il titolo di default di una registrazione manuale è `Meeting 27_11_26_14_30_05`, in inglese (frontend e Rust).
- `UpdateDialog` mostra il changelog come testo grezzo con i trattini. `WhatsNew` lo formatta già con `parseReleaseNotes`.
- Possibile falso «Negato» per l'audio di sistema nell'onboarding se in quel momento non c'è audio (`PermissionsStep.tsx:115-124`): da verificare dal vivo.

**Prestazioni**
- `RecordingStateContext` interroga il backend ogni 500 ms e crea sempre un nuovo stato, quindi 24 consumatori si ri-renderizzano due volte al secondo durante la registrazione, con un `console.log` a ogni giro. `useRecordingStateSync` aggiunge un altro polling a 1 s.

### P3: rifiniture

- La sidebar anima `width` e il contenuto anima `margin-left` (`transition-all` per 300 ms): ri-layout della trascrizione a ogni frame. Le barre usano `transition-[width]`.
- L'anello di focus reale è di 1 px (`outline-hidden` + `ring-1`), contro i 2 px di DESIGN.md. Dialog e select usano `focus:` invece di `focus-visible:`.
- Gerarchia dei titoli: in Impostazioni si passa da `h1` a `h3`; i giorni della sidebar sono `h3` prima di qualsiasi `h1`, ed ereditano Montserrat anche se sono etichette.
- Ombre fuori da `shadow-float`: `AgentsDemo.tsx:59`, il pollice di `ui/switch.tsx`, `ui/sheet.tsx`. Riquadri riempiti: `SummaryPanel` (`bg-card` in scuro), `AgentsAnnouncement` (`bg-muted`).
- `cleanStopWords` toglie «ah/eh/oh» dalla trascrizione mostrata, che in italiano sono esclamazioni.
- Codice e asset morti: `useModelConfiguration.ts` (toast in inglese, mai importato), `useAudioPlayer`/`AudioPlayer`, gli SVG demo di Next in `public/`. Ci sono anche dipendenze non importate: `framer-motion`, `lodash`, `react-markdown`, `remark-gfm`, `@heroicons/react`, `radix-ui`.
- Il riassunto in formato legacy intercetta `Cmd+Z` su tutto il documento e ha un menu contestuale solo da clic destro.
- Numeri con `toFixed` e punto decimale in italiano (Importa audio, gestione modelli esperto).
- `banner.html` copia a mano i token: rischio di deriva quando cambia la palette.

## Carico cognitivo e persone

Le decisioni con più di quattro opzioni visibili sono cinque:
- le sette schede delle Impostazioni;
- la barra del riassunto, con 6-7 controlli;
- la legenda dei parlanti, con 8 nomi e «+N»;
- il menu Template, con fino a 9 voci;
- il selettore del client MCP, con 8 voci.

L'empty state della home invece toglie carico: una sola azione e due righe di spiegazione.

- **Vito (utente esperto):**
  - i secondi dell'Automazione non si potevano digitare (corretto);
  - Rigenera e il cambio di tipo cancellano le modifiche fatte a mano al riassunto;
  - non ci sono scorciatoie per avviare, fermare o aggiungere una nota;
  - le note live non si correggono.
- **Collega al primo avvio, non tecnico:**
  - 9 GB scaricati senza consenso né stima dei tempi;
  - l'annuncio «Dixit lavora con i tuoi agenti» parla di MCP a chi non lo usa;
  - i permessi Accessibilità e Registrazione schermo sono spiegati in modo tecnico;
  - il percorso «Impostazioni › Info» citato nei testi non esiste.
- **Chi usa la tastiera o VoiceOver:**
  - la banda dei turni si usa solo col mouse;
  - il banner non si raggiunge;
  - le barre di avanzamento erano mute (corretto);
  - l'anello di focus è di 1 px.

## Cosa funziona

1. **I parlanti sono il prodotto, e si vede.** `SpeakerName`, la banda dei turni, la legenda con rinomina e i colori stabili per roster. Il vocabolario `StateStamp`/`LiveIndicator` è coerente ovunque.
2. **Disciplina sul colore e sulla struttura.** C'è un solo pulsante petrolio per schermata, lo stop usa la variante `live`, filetti al posto delle card. Le Impostazioni sono sobrie e leggibili in chiaro e in scuro.
3. **L'attesa del riassunto è ben gestita.** Fase, percentuale reale e X per annullare compaiono sia in sidebar sia nella vista riunione.
4. **Cura nei dettagli.** L'empty state spiega il modello mentale in due righe. L'eliminazione chiede conferma e offre il Cestino. Il banner non ruba il focus e dichiara quando Teams è visibile a tutti. L'i18n è completo: 0 chiavi mancanti tra IT e EN su 7 file, e nessun residuo «Meetily» visibile.

## Domande per Vito

1. Il primo avvio deve restare automatico (download subito) o diventare un passaggio con consenso e stima?
2. Il rosso per gli errori generici è voluto, o serve un colore di errore distinto da «sta registrando»?
3. Il percorso «Impostazioni › Info» va creato (spostando Info nelle Impostazioni) o vanno corretti i testi?
4. Tipo di call e Template: va esposto solo il Tipo, con il Template nella modalità esperto?

Questions skipped: l'ambito della critique era già fissato dal brief (correggere i P1, lasciare P2 e P3 come proposte). Le domande qui sopra restano aperte per Vito. Lo snapshot `.impeccable/critique/` non è stato salvato, perché il report è questo file.

Comandi impeccable suggeriti per il seguito:
- `/impeccable onboard` per il primo avvio;
- `/impeccable harden` per errori e plurali;
- `/impeccable distill` per la barra del riassunto e le schede delle Impostazioni;
- `/impeccable polish` alla fine.
