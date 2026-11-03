# CLAUDE.md

Guida per lavorare su questo repository con Claude Code.

## Cos'è Dixit

**Dixit** (repo `trascrivi-app`) è un'app desktop macOS che registra, trascrive e riassume le riunioni interamente in locale. È un fork di [Meetily](https://github.com/Zackriya-Solutions/meeting-minutes) (MIT, attribuzione in `LICENSE.md`): l'app supportata è Tauri 2 + Next.js 15, con il core in Rust. La usano Vito e i colleghi, solo su Mac (Apple Silicon).

- **Nessuna dipendenza esterna:** trascrizione e riassunto girano su modelli locali, scaricati al primo avvio. Ollama e i provider cloud restano solo come opzioni da esperto.
- **Interfaccia bilingue:** italiano e inglese (`frontend/src/i18n/`).
- **Design system:** `DESIGN.md`. Prodotto e scelte: `PRODUCT.md`.

## Cosa fa

| Funzione | Dove vive |
|---|---|
| Avvio e stop automatici quando inizia o finisce una call Teams, Zoom, Meet | `src-tauri/src/autopilot/` |
| Nomi dei parlanti e sottotitoli da Teams; trascrizione nativa Teams solo se la chiedi dal pulsante del banner (è visibile a tutti) | `src-tauri/src/teams/`, `src-tauri/src/speakers/`, `public/banner.html` |
| Trascrizione live con Parakeet, definitiva con Whisper large-v3 a fine registrazione | `src-tauri/src/parakeet_engine/`, `src-tauri/src/whisper_engine/`, `audio/transcription/` |
| Attribuzione dei parlanti (wespeaker) | `src-tauri/src/speakers/` |
| Memoria delle voci: una voce già nominata viene riconosciuta nelle riunioni successive (dopo i nomi da Teams, prima di «Parlante X») | `src-tauri/src/speakers/voices.rs`; taratura con `cargo test --lib bench_voice_memory -- --ignored` |
| Note personali durante la registrazione, con l'istante | `src-tauri/src/notes.rs`, `frontend/src/components/RecordingNoteInput.tsx` |
| Riassunto automatico per tipo di call (SAL, analisi, mix, altro) | `src-tauri/src/summary/` (`auto.rs`, `classification.rs`) |
| Server MCP locale per gli agenti (trascrizione live, riunioni, ricerca) | `src-tauri/src/mcp.rs`, `mcp_tools.rs`, `library.rs`, `search.rs`, `embedding.rs` |
| Cartella per riunione in `~/Documents/Dixit/` | `src-tauri/src/audio/` (salvataggio), `notes.rs` |

## Modelli (configurazione base)

La configurazione di default è quella di Vito e non va cambiata senza motivo. I modelli **non sono nel pacchetto** (le release GitHub accettano file fino a 2 GB): al primo avvio `BaseModelsGate` li scarica (circa 9 GB) nella cartella modelli dei dati utente. Solo un utente esperto li cambia, da Impostazioni › Avanzate › Modalità esperto.

| Uso | Modello | File in `models/` |
|---|---|---|
| Trascrizione live | Parakeet TDT 0.6B v3 int8 (multilingue) | `parakeet/parakeet-tdt-0.6b-v3-int8/` |
| Trascrizione definitiva | Whisper large-v3 | `ggml-large-v3.bin` |
| Parlanti | wespeaker ResNet34 | `speaker/` |
| Riassunto | Gemma 4 E4B Q4_K_M, motore interno `llama-helper` | `summary/gemma-4-E4B-it-Q4_K_M.gguf` |
| Ricerca semantica | EmbeddingGemma 300M (ONNX, `ort`) | `embedding/embeddinggemma-300m/` |

- **Dove li cerca l'app:** nella cartella modelli dei dati utente (`~/Library/Application Support/com.vitomanuguerra.trascrivi/models/`), poi nel pacchetto se ce n'è uno vecchio che li include (`src-tauri/src/bundled_models.rs`).
- **Download al primo avvio:** `src-tauri/src/base_models.rs` elenca i file con revisione Hugging Face fissata, dimensione e SHA-256; riprende i `.part` interrotti. Per cambiare un modello di default aggiorna lì URL, dimensione e hash. Prova di rete: `cargo test --lib base_models_download_real -- --ignored`.
- **Perché Gemma 4 E4B:** confronto in `docs/confronto-modelli-riassunto.md`. Gemma 4 26B-A4B è migliore ma pesa 16 GB e usa circa 24 GB di RAM: è un'opzione esperto da scaricare, non il default. Qwen 3.5 4B è sconsigliato (inventa dati).
- Gemma 4 a volte apre la risposta con un canale di pensiero vuoto: il client lo toglie (`strip_gemma4_thought`).

## Riassunto automatico

1. A fine registrazione gira large-v3, poi l'attribuzione dei parlanti (`speakers/mod.rs`), poi `summary::auto::after_final_transcript`.
2. Un passaggio breve con l'LLM classifica la call: `sal`, `analisi`, `mix`, `altro` (JSON con tipo e motivazione). Testi sotto le 40 parole vanno su `altro` senza chiamare il modello.
3. Il tipo sceglie il template in `frontend/src-tauri/templates/`: `sal.json`, `analisi.json`, `sal_analisi.json` (mix), `standard_meeting.json` (altro), più `daily_standup`, `project_sync`, `retrospective`, `sales_marketing_client_call`.
4. Il tipo si salva in `meetings.meeting_type` e `meeting_type_reason`; nella vista riunione si cambia a mano e il riassunto si rigenera.
5. Le note dell'utente entrano nel prompt come `<user_notes>` (`[mm:ss] testo`) e pesano come segnale forte.

Regole dei template: nomi, date e cifre non presenti nella trascrizione diventano «Non indicato», le sezioni vuote «Non emerso». Le etichette generiche di diarizzazione («Parlante A», «Altri») non arrivano all'LLM.

**Concorrenza:** il riassunto automatico parte una sola volta per riunione, e mentre si aspetta mostra «Riassunto automatico in arrivo» al posto di Genera. Le generazioni sul motore interno passano una alla volta (`GENERATION_TURN` in `summary_engine/client.rs`): un annullamento spegne il sidecar e la richiesta successiva lo rilancia.

## Note durante la registrazione

- Campo note sempre visibile in registrazione; Invio salva la singola nota con `offset_ms` dall'inizio e orario. Tabella `meeting_note_entries`. La vecchia `meeting_notes` (un blocco markdown per riunione) è eredità di Meetily.
- Le note compaiono dentro la trascrizione al loro istante, si modificano e si eliminano.
- Nel `.md` della cartella sono intercalate come `> Nota [mm:ss] testo` e salvate anche in `notes.json`.
- La casella «Includi le mie note» vale per «Copia», ricorda l'ultima scelta ed è attiva di default.

## Server MCP per agenti

Server `dixit`, HTTP streamable su `http://127.0.0.1:7337/mcp`, solo lettura. Si attiva con `autopilot.mcp_enabled` in `trascrivi.json` (dati utente). `DIXIT_MCP_ADDR` cambia indirizzo, utile per una seconda istanza.

| Tool | Cosa restituisce |
|---|---|
| `live_transcript` | La trascrizione della registrazione in corso |
| `list_meetings` | Riunioni, più recenti prima: filtri `from`, `to`, `updated_since` (ingest incrementale), `limit`, `offset` |
| `get_meeting` | Metadati, tipo di call, partecipanti, riassunto, note, trascrizione. `format: json\|markdown` (pronto per una wiki), `include_notes`, `include_transcript`, `transcript_offset`/`transcript_limit` (pagine di circa 18k caratteri) |
| `search_transcripts` | `mode: classic` (SQLite FTS5), `smart` (embedding, nessun LLM), `hybrid` (fusione RRF, default). Cerca anche in note e riassunti |

Le date senza fuso orario sono ora locale. I vettori stanno in SQLite e il confronto è a forza bruta: basta fino a migliaia di riunioni, poi sqlite-vec. Per provarli con un client vero:

```bash
claude -p --strict-mcp-config --mcp-config mcp.json --allowedTools "mcp__dixit__list_meetings mcp__dixit__get_meeting mcp__dixit__search_transcripts" "elenca le riunioni"
```

con `mcp.json` = `{"mcpServers":{"dixit":{"type":"http","url":"http://127.0.0.1:7337/mcp"}}}`.

## Dati e cartelle

- **Dati app:** `~/Library/Application Support/com.vitomanuguerra.trascrivi/` (`meeting_minutes.sqlite`, `trascrivi.json`, `models/`, `voices.json` con le impronte vocali dei colleghi, solo locale). Sono dati reali: per le prove distruttive lavora su copie.
- **Riunioni:** `~/Documents/Dixit/<AAAA-MM-GG HH.MM Titolo>/` con `audio.mp4`, trascrizione `.md`, `transcripts.json`, `metadata.json`, `diarization.json`, `notes.json`, `captions.json` (solo Teams).
- **Migrazioni:** `frontend/src-tauri/migrations/` (sqlx). Ogni migrazione deve avere una versione unica: due file con lo stesso prefisso bloccano l'avvio.
- Alcuni nomi interni restano «meetily» (cartelle di riserva dei modelli, user agent, sorgente audio): non rinominarli, si perderebbero dati già salvati.

## Comandi

Tutti da `frontend/`.

```bash
pnpm install                      # dipendenze
pnpm run tauri:dev                # sviluppo (rileva la GPU, su Mac usa --features metal)
pnpm run tauri:build              # .app e .dmg in target/release/bundle/
cd src-tauri && cargo test --lib  # test Rust (circa 370)
npx tsc --noEmit                  # controllo dei tipi del frontend
```

Log di debug: `RUST_LOG=app_lib=debug pnpm run tauri:dev`. L'app installata scrive in `~/Library/Logs/Dixit/dixit.log` (livello `info`, il precedente diventa `dixit.log.1` oltre 20 MB). Il confronto tra modelli e la verifica dei riassunti sono test ignorati: `cargo test --lib bench_summary_model -- --ignored` e `cargo test --lib verifica_riassunti -- --ignored --nocapture` (istruzioni in testa ai file).

### Sandbox di prova (accanto alla Dixit installata)

```bash
DIXIT_MCP_ADDR=127.0.0.1:7338 npx tauri dev --config '{"identifier":"com.vitomanuguerra.trascrivi.dev"}' -- --features metal
```

Usa i dati in `…/com.vitomanuguerra.trascrivi.dev/` (copia il DB reale con `sqlite3 … ".backup …"`; `models` può essere un symlink a quella reale).

- **Da Claude Code** va lanciata fuori dalla sandbox dei comandi: dentro, il processo web della finestra va in crash («Couldn't connect to launchservicesd») e la finestra resta bianca.
- **`ChunkLoadError … app/layout … timeout`** al primo avvio: Next stava ancora compilando. Quando `curl 127.0.0.1:3118/_next/static/chunks/app/layout.js` risponde, riavvia.
- **La finestra è nascosta nella barra dei menu:** riaprila da «Apri Dixit» nel menu dell'icona.

### Build e distribuzione

- **Release pubblica:** `scripts/release.sh <versione>` compila, firma, scrive `latest.json` e pubblica su `cannoloops/dixit` (dettagli in `docs/RELEASE.md`). L'updater legge `releases/latest/download/latest.json` dello stesso repo.
- **Changelog:** ogni release ha un changelog breve in testa a `docs/release-notes.md` («**Novità in <versione>**», una riga per novità, pensata per i colleghi). Senza, `release.sh` si ferma.
- Solo il `.dmg`: `npx tauri build --bundles dmg -- --features metal`.
- L'eseguibile si chiama `Dixit` (`mainBinaryName` e `[[bin]]` in `Cargo.toml`); il package Cargo resta `meetily`.
- **Firma.** Con `APPLE_SIGNING_IDENTITY="Dixit Local Signing"` (in `~/.zshrc` di Vito) Tauri firma con il certificato autofirmato «Dixit Local Signing» del portachiavi: il requisito designato si basa sul certificato e non sul cdhash, quindi macOS conserva microfono, registrazione schermo e Accessibilità da una build all'altra. Controllo: `codesign -dr - /Applications/Dixit.app` mostra `certificate leaf`, non `cdhash`. Senza la variabile vale `signingIdentity: "-"` di `tauri.conf.json` (firma ad hoc, per i colleghi): a ogni installazione l'Accessibilità va tolta e riaggiunta.
- Il certificato si crea una volta: Accesso Portachiavi › Assistente certificato › Crea un certificato, «Radice autofirmata», tipo «Firma codice», validità 3650 giorni. Alla prima build firmata codesign chiede l'accesso alla chiave («Consenti sempre») e i permessi vanno ridati una volta.
- Il pacchetto non è notarizzato. Sui Mac dei colleghi: Fine nell'avviso, poi Impostazioni di Sistema › Privacy e sicurezza › Apri comunque (da macOS 15 «tasto destro › Apri» non basta più), oppure `xattr -dr com.apple.quarantine /Applications/Dixit.app`. Procedura per gli utenti in `docs/GUIDA.md`. Si scarica dalle release di GitHub.
- Installare sul proprio Mac: chiudi Dixit, sposta `/Applications/Dixit.app` nel Cestino (reversibile), copia la nuova con `ditto`. Al primo avvio partono le migrazioni; macOS può richiedere di nuovo microfono e registrazione schermo.

## Architettura (Rust ↔ frontend)

- **Comandi:** il frontend chiama `invoke('nome', {...})`; i comandi sono `#[tauri::command]` registrati in `src-tauri/src/lib.rs` (`generate_handler!`).
- **Eventi:** il Rust emette (`app.emit("transcript-update", …)`, `speakers-updated`, …) e il frontend ascolta con `listen`.
- **Stato condiviso:** `Arc<RwLock<T>>` tra task async, `Arc<AtomicBool>` per i flag.
- **Pipeline audio** (`audio/pipeline.rs`): mic e sistema vengono mixati per la registrazione (ducking RMS) e, in parallelo, filtrati con VAD per la trascrizione. Si aspetta 48 kHz; il ricampionamento avviene in cattura.
- **Dispositivi audio:** `audio/devices/` (discovery, microfono, speaker, piattaforma). Su macOS (14.4 o superiore) l’audio di sistema passa da ScreenCaptureKit (`audio/capture/core_audio.rs`) e servono i permessi microfono e registrazione schermo.
- **Log nei punti caldi:** `perf_debug!` e `perf_trace!` spariscono in release.
- **File e percorsi:** usa le API di percorso di Tauri, mai percorsi scritti a mano.

## Convenzioni

- **Lingua:** testi dell'interfaccia in italiano e inglese; documentazione del progetto in italiano.
- **Nomi:** «microphone» e «system» per i dispositivi audio, non «input» e «output».
- **Errori:** `anyhow::Result` in Rust; try/catch con messaggi comprensibili nel frontend.
- **Semplificazioni volute:** commenti `ponytail:` con il limite e la strada per superarlo.
- **Git:**
  - commit brevi in italiano (`feat:`, `fix:`, `chore:`, `docs:`), senza riferimenti a Claude;
  - `main` è il branch principale: stabile, da qui partono le build da distribuire;
  - `develop` è il branch di sviluppo: ogni attività nasce da `develop` su un feature branch (`feat/*`, `fix/*`) e ci torna con un merge;
  - quando `develop` è pronto per una build, si mergia in `main`. Mai commit diretti su `main`.
- **Documenti di verifica:** `docs/confronto-modelli-riassunto.md`.
