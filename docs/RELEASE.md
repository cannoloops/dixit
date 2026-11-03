# Dixit – release, installazione e collaudo

## Una tantum (Vito)

1. **Certificato Developer ID** (account Apple Developer personale):
   - in Xcode → Settings → Accounts, crea un certificato *Developer ID Application*;
   - controlla che compaia con `security find-identity -v -p codesigning`.
2. **Password per app** per la notarizzazione: appleid.apple.com → Accesso e sicurezza → Password per le app.
3. **Release:** vanno su `cannoloops/dixit`. L'updater legge
   `https://github.com/cannoloops/dixit/releases/latest/download/latest.json`.
4. **Chiave dell'updater**:
   - privata in `~/.tauri/trascrivi.key`, senza password. Va fatto un backup: se si perde, i colleghi devono reinstallare a mano;
   - pubblica già inserita in `frontend/src-tauri/tauri.conf.json`.

## Fare una release

Senza account Apple Developer (oggi):

```bash
scripts/release.sh 0.2.0    # firma con APPLE_SIGNING_IDENTITY di ~/.zshrc, non notarizzata
```

Con il Developer ID, per una release notarizzata:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Vito Manuguerra (TEAMID)"
export APPLE_ID="tua-email@…"
export APPLE_PASSWORD="password-per-app"
export APPLE_TEAM_ID="TEAMID"
scripts/release.sh 0.2.0
```

Lo script:
1. aggiorna la versione in `tauri.conf.json`, `package.json` e `Cargo.toml`;
2. compila il sidecar `llama-helper`;
3. lancia `tauri build` (app e dmg, senza modelli: si scaricano al primo avvio);
4. scrive `latest.json`;
5. pubblica la release `v0.2.0` su `cannoloops/dixit` con dmg, `.app.tar.gz`, `.sig` e `latest.json`, con le note di `docs/release-notes.md`.

Fai il commit del cambio di versione.

`scripts/release.sh 0.2.0 --dry-run` fa la stessa build senza notarizzazione e senza pubblicare. Serve a verificare il bundle in locale.

### Note di build su questo Mac

`.cargo/config.toml` contiene due correzioni locali:
- `SDKROOT` punta all'SDK di Xcode, perché quello dei Command Line Tools non linka;
- `CMAKE_DISABLE_FIND_PACKAGE_OpenSSL`, perché llama.cpp non compila con OpenSSL 3.6 di Homebrew.

## Installazione

1. Scarica il `.dmg` dall'ultima release di `cannoloops/dixit` e trascina Dixit in Applicazioni.
2. La prima volta aprilo con tasto destro › Apri (non è notarizzato).
3. Al primo avvio scarica i modelli (circa 9 GB), poi l'onboarding chiede i permessi di Microfono, Audio di sistema e Accessibilità.
4. In Teams imposta la lingua parlata su italiano.
5. Gli aggiornamenti successivi arrivano da soli.

Dixit convive con Meetily: dati in `~/Library/Application Support/com.vitomanuguerra.trascrivi`, registrazioni in `~/Documents/Dixit` (una cartella per riunione: `AAAA-MM-GG HH.MM titolo`).

## Collaudo su call reali (criteri 1–7 delle specifiche)

Queste prove richiedono call vere e non sono automatizzate.

- [ ] **Riunione Teams in calendario:**
  - parte entro circa 10 s, compare il banner con **Annulla** e arriva la notifica;
  - si ferma entro 60 s dalla chiusura;
  - arriva la notifica "Trascrizione salvata".
- [ ] **Chiamata Teams 1:1:** stesso comportamento.
- [ ] **Robustezza:** con mute per 5 minuti, cambio di cuffie a metà call o condivisione schermo la trascrizione **non** si ferma.
- [ ] **Zoom (app) e Meet (Chrome):** avvio e stop automatici.
- [ ] **Annulla dal banner, oppure "Annulla registrazione" dal menu:**
  - la cartella della riunione sparisce;
  - la riunione non compare nell'elenco.
- [ ] **Teams con almeno 3 partecipanti:**
  - i sottotitoli live si attivano da soli;
  - i segmenti hanno nome e cognome;
  - i tuoi interventi hanno il tuo nome.
- [ ] **Trascrizione nativa di Teams:**
  - dove è consentita parte da sola, con la notifica "visibile a tutti";
  - dove non è consentita arriva la notifica "non avviata" e il resto funziona.
- [ ] **Import del `.docx`:** scaricando dal recap di Teams il `.docx` della trascrizione, entro circa 1 minuto arriva la notifica "Trascrizione Teams collegata".
- [ ] **Due riunioni Teams una dopo l'altra** (lasci A, entri in B entro 30 s): producono due riunioni distinte, con al massimo circa 30 s di B finiti in A.
- [ ] **Stop manuale in A, poi entri in B:** B viene registrata.
- [ ] **Meet:** se passi a un altro tab per più di un minuto o condividi un tab, la registrazione continua.
- [ ] **Annulla subito:** premuto entro 2 secondi dalla comparsa del banner, la call non viene registrata e la cartella non resta.
- [ ] **Trascrizione già avviata da un altro** (es. l'organizzatore):
  - Dixit non mostra "avviata";
  - con Annulla o con lo stop manuale non la ferma.
- [ ] **Minuta:** `parse_teams.py` della skill `minuta-call`, lanciato sul file `<titolo>-Trascrizione della riunione.md` nella cartella della riunione, riporta i nomi.

### Esito del collaudo del 23/10/2026

Tre call Teams vere (80 min di riunione, una 1:1 di 14 min, più una ripresa a metà call):

- **Funzionano:**
  - avvio e stop automatici;
  - una call nuova dopo la fine della precedente viene registrata a parte;
  - separazione Io/Altri;
  - `.docx` e salvataggio nel DB.
- **Corretto dopo il collaudo:**
  - parlanti divisi in troppi gruppi: soglia 0,6 → 0,35, più l'accorpamento dei gruppi piccoli; sulla call di 80 minuti i gruppi passano da 167 a 14;
  - titolo nella vista compatta di Teams.
- **Ancora da verificare in una call con la finestra della riunione ingrandita:**
  - sottotitoli live e trascrizione nativa (nella vista compatta di Teams manca il menu "Altro", quindi l'app aspetta l'ingrandimento);
  - Zoom e Meet;
  - Annulla immediato.
- **Da chiarire:** una registrazione partita all'avvio dell'app, a call già in corso, si è fermata dopo 5 s. Ora i log sono in `~/Library/Logs/Dixit/dixit.log` (avvio con `scripts/install-local.sh`).

**Se Teams cambia interfaccia** (sottotitoli o trascrizione nativa non si attivano più):
1. durante una call apri Impostazioni → Automazione → **Salva diagnostica**;
2. il JSON salvato in `diagnostics/` serve ad aggiornare i selettori in `frontend/src-tauri/src/teams/ax.rs`.

## Valori da tarare sulle call reali

| Valore | Dove | Ora |
|---|---|---|
| Secondi prima dell'avvio / dello stop | Impostazioni → Automazione | 5 / 45 |
| Soglia stessa voce | `speakers/cluster.rs` `SAME_SPEAKER_THRESHOLD` | 0.6 |
| Io/Altri (energia mic vs sistema) | `speakers/naming.rs` `MIC_FLOOR_RMS`, `MIC_OVER_SYSTEM` | 0.005 / 2.0 |
| Ritardo dei sottotitoli | `teams/captions.rs` `CAPTION_LAG_SECS` | 1.5 s |
