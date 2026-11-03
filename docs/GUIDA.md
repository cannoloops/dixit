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

1. Dixit scarica i modelli: circa 9 GB, da Hugging Face, con verifica dell'integrità. Se il download si interrompe, riparte da dove era arrivato.
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

## Note durante la call

Scrivi nel campo note e premi **Invio**: la nota resta agganciata all'istante esatto, compare dentro la trascrizione e aiuta il riassunto. Le note si modificano e si eliminano dalla vista della riunione.

## A fine call

1. Dixit rifà la trascrizione con un modello più preciso e attribuisce le battute ai parlanti.
2. Riconosce il tipo di riunione (SAL, analisi, mix o altro) e scrive il riassunto adatto.
3. Se il tipo è sbagliato, cambialo nella vista della riunione: il riassunto si rigenera.

Per dare un nome a un «Parlante X», rinominalo nel pannello dei parlanti: Dixit ricorderà la voce nelle riunioni successive.

## Dove trovo i file

Ogni riunione ha una cartella in `~/Documents/Dixit/<AAAA-MM-GG HH.MM Titolo>/` con l'audio, la trascrizione in Markdown con le tue note, il riassunto e i metadati.

## Aggiornamenti

Dixit si aggiorna da solo; riunioni, impostazioni e modelli restano. Dopo un aggiornamento macOS può chiedere di nuovo i permessi.
