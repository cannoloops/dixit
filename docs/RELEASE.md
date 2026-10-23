# Trascrivi – release, installazione e collaudo

## Una tantum (Vito)

1. **Certificato Developer ID** (account Apple Developer personale):
   - in Xcode → Settings → Accounts, crea un certificato *Developer ID Application*;
   - controlla che compaia con `security find-identity -v -p codesigning`.
2. **Password per app** per la notarizzazione: appleid.apple.com → Accesso e sicurezza → Password per le app.
3. **Repo pubblico delle sole release**:
   ```bash
   gh repo create cannoloops/trascrivi-releases --public --description "Trascrivi releases"
   ```
   Il codice resta nel repo privato. L'updater legge
   `https://github.com/cannoloops/trascrivi-releases/releases/latest/download/latest.json`.
4. **Chiave dell'updater**:
   - privata in `~/.tauri/trascrivi.key`, senza password. Va fatto un backup: se si perde, i colleghi devono reinstallare a mano;
   - pubblica già inserita in `frontend/src-tauri/tauri.conf.json`.

## Fare una release

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
3. lancia `tauri build` (app e dmg, firmati e notarizzati);
4. scrive `latest.json`;
5. pubblica la release `v0.2.0` su `cannoloops/trascrivi-releases` con dmg, `.app.tar.gz`, `.sig` e `latest.json`.

Fai il commit del cambio di versione.

`scripts/release.sh 0.1.0 --dry-run` fa la stessa build con firma ad-hoc, senza notarizzazione e senza pubblicare. Serve a verificare il bundle in locale.

### Note di build su questo Mac

`.cargo/config.toml` contiene due correzioni locali:
- `SDKROOT` punta all'SDK di Xcode, perché quello dei Command Line Tools non linka;
- `CMAKE_DISABLE_FIND_PACKAGE_OpenSSL`, perché llama.cpp non compila con OpenSSL 3.6 di Homebrew.

## Installazione (colleghi)

1. Scarica il `.dmg` dall'ultima release di `cannoloops/trascrivi-releases` e trascina Trascrivi in Applicazioni.
2. Al primo avvio l'onboarding:
   - chiede i permessi di Microfono, Audio di sistema e Accessibilità;
   - chiede il tuo nome come appare in Teams;
   - scarica i modelli.
3. In Teams imposta la lingua parlata su italiano.
4. Gli aggiornamenti successivi arrivano da soli.

Trascrivi convive con Meetily: dati in `~/Library/Application Support/com.vitomanuguerra.trascrivi`, registrazioni in `~/Movies/trascrivi-recordings`.

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
- [ ] **Minuta:** `parse_teams.py` della skill `minuta-call`, lanciato sul file `<titolo>-Trascrizione della riunione.docx` nella cartella della riunione, riporta i nomi.

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
