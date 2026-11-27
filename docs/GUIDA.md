# Guida di Dixit

Dixit registra, trascrive e riassume le riunioni sul tuo Mac. Nessun dato esce dal computer.

## Requisiti

- Mac con Apple Silicon (M1 o successivo)
- macOS 14.4 o superiore
- almeno 16 GB di RAM e circa 10 GB liberi

## Installazione

1. Scarica `Dixit_<versione>_aarch64.dmg` dall'[ultima versione](https://github.com/cannoloops/dixit/releases/latest).
2. Apri il `.dmg` e trascina **Dixit** in **Applicazioni**.
3. Apri Dixit da Applicazioni.

### «Apple non è in grado di verificare…»

Dixit non è notarizzato da Apple, quindi la prima volta macOS lo blocca con l'avviso *«… non è stata aperta. Apple non è in grado di verificare che … non contenga malware»*. Può succedere già all'apertura del `.dmg`, poi di nuovo al primo avvio dell'app. Da macOS 15 il vecchio «tasto destro › Apri» non basta più. Per sbloccarlo:

1. Nell'avviso premi **Fine**, non «Sposta nel Cestino».
2. Apri **Impostazioni di Sistema › Privacy e sicurezza** e scorri fino a **Sicurezza**.
3. Accanto al messaggio *«… è stato bloccato»* premi **Apri comunque** e conferma con password o Touch ID.
4. Riapri il file. Nel nuovo avviso premi **Apri comunque**.

Se l'avviso compare sia per il `.dmg` sia per l'app, ripeti i passi per entrambi.

In alternativa, dal Terminale, dopo aver copiato Dixit in Applicazioni:

```bash
xattr -dr com.apple.quarantine /Applications/Dixit.app
```

Va fatto una volta sola: gli aggiornamenti successivi arrivano da soli.

## Primo avvio

1. Dixit scarica i modelli: circa 9 GB, da Hugging Face, con verifica dell'integrità. Parte quando premi **Scarica i modelli** e mostra velocità e tempo che manca. Se il download si interrompe, o se esci con **Esci da Dixit**, al prossimo avvio riparte da dove era arrivato.
2. Poi chiede tre permessi di macOS:
   - **Microfono:** per la tua voce;
   - **Registrazione schermo e audio di sistema:** per l'audio degli altri partecipanti;
   - **Accessibilità:** per leggere i nomi dei partecipanti da Teams.
3. In Teams imposta la lingua parlata su italiano.

Se un permesso è stato negato, attivalo da **Impostazioni di Sistema › Privacy e sicurezza** e riavvia Dixit.

## Registrare una riunione

- **In automatico:** quando inizia una call su Teams, Zoom o Meet Dixit parte da solo e mostra un banner con **Annulla**. Si ferma quando la call finisce.
- **A mano:** avvia e ferma la registrazione dalla finestra di Dixit.
- Mentre registri vedi la trascrizione live e i nomi dei parlanti (da Teams, oppure dalle voci già riconosciute).
- **Google Meet nel browser:** per avere i nomi dei parlanti Dixit accende i sottotitoli di Meet a inizio call. Li vedi solo tu, non gli altri. Se li spegni, Dixit non li riaccende.
- **Muto su Teams:** quando sei in muto nella call, Dixit non registra né trascrive la tua voce, e accanto al tempo di registrazione compare «Microfono in muto». Su Zoom e Meet il microfono viene registrato anche in muto.

## Note durante la call

Scrivi nel campo note e premi **Invio**: la nota resta agganciata all'istante esatto, compare dentro la trascrizione e aiuta il riassunto. Le note si modificano e si eliminano dalla vista della riunione.

## A fine call

1. Dixit rifà la trascrizione con un modello più preciso e attribuisce le battute ai parlanti.
2. Riconosce il tipo di riunione (SAL, analisi, mix o altro) e scrive il riassunto adatto.
3. Se il tipo è sbagliato, cambialo nella vista della riunione: il riassunto si rigenera.

Finché il riassunto non è pronto, la barra a sinistra mostra «Riassunti in corso» con la fase di ognuno, compresa la percentuale della trascrizione definitiva. La X sulla riga ferma il riassunto. Se è automatico e non è ancora partito, lo annulla solo per quella riunione: la trascrizione e i parlanti vanno avanti, e il riassunto lo generi quando vuoi.

Per dare un nome a un «Parlante X», rinominalo nel pannello dei parlanti: Dixit ricorderà la voce nelle riunioni successive.

## Collegare un agente

Dixit può fare da memoria ai tuoi agenti (Claude Code, Claude Desktop, Codex, Antigravity, Gemini CLI, Cursor, VS Code e altri): chiedi cosa si è detto nella call in corso, cerca nelle riunioni passate o fai trascrivere un file audio o video.

1. Apri **Impostazioni › Automazione › Agenti** e accendi **Server MCP locale**.
2. Scegli il tuo agente dal menu e copia il comando o la configurazione che compare, seguendo l'indicazione sopra (di solito: incolla nel terminale, oppure aggiungi a un file e riavvia l'agente).
3. Chiedi all'agente, ad esempio: «Cosa ha detto Mario negli ultimi 5 minuti?» o «Quando abbiamo fissato la demo?».

Il server risponde solo dal tuo Mac; quello che l'agente fa poi con il testo dipende dall'agente.

## Dove trovo i file

Ogni riunione ha una cartella in `~/Documents/Dixit/<AAAA-MM-GG HH.MM Titolo>/` con l'audio, la trascrizione in Markdown con le tue note, il riassunto e i metadati.

Per eliminare più riunioni insieme selezionale nella barra laterale con **Cmd+clic** (una alla volta) o **Maiusc+clic** (un intervallo), poi premi il cestino accanto a «N selezionate»; **Esc** annulla la selezione. Alla conferma, la casella «Sposta nel Cestino anche la cartella» (attiva di default) sposta anche le cartelle nel Cestino di macOS, da cui si possono recuperare.

## Spazio su disco

L'audio è la parte che pesa: circa 30 MB per ora di riunione.

- **Compressione automatica:** una volta al giorno, in background, Dixit comprime l'audio delle riunioni registrate con le versioni precedenti (che pesava circa tre volte tanto). Non perdi niente: riascolto e trascrizione restano uguali. Si ferma da sola se inizia una call.
- **Eliminare l'audio vecchio:** in **Impostazioni › Registrazioni › Elimina l'audio delle riunioni vecchie** scegli dopo quanti mesi (3, 6 o 12) spostare l'audio nel Cestino. Trascrizione, riassunto, note e parlanti restano; non potrai più riascoltare né ritrascrivere quelle riunioni. Di default è spento. Lo spazio si libera quando svuoti il Cestino.

## Aggiornamenti

Dixit si aggiorna da solo; riunioni, impostazioni e modelli restano. Prima di aggiornare vedi cosa cambia, e al primo avvio della nuova versione un pannello riassume le novità. Per rivederlo: **Info** (in fondo alla barra laterale) › **Rivedi le novità**; dallo stesso pannello si apre anche la **Guida**. Dopo un aggiornamento macOS può chiedere di nuovo i permessi.
