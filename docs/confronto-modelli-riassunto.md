# Confronto modelli per il riassunto

24 ottobre 2026, Mac M4 Pro 48 GB, motore interno `llama-helper` (llama.cpp via `llama-cpp-2` 0.1.146, Metal).

## Esito

- **Default: Gemma 4 E4B Q4_K_M**, incluso nel pacchetto. Italiano corretto, nessuna invenzione sulle tre riunioni, stessi pesi del `gemma4:latest` di Ollama usato finora.
- **Gemma 4 26B-A4B (MoE) vince nettamente sulla qualità**: decisioni e azioni più precise, citazioni giuste, nessun errore di fatto. Ed è anche più veloce. Pesa però 16 GB su disco e circa 24 GB di RAM residente. Quindi è un'**opzione esperto da scaricare** (`gemma4:26b-a4b`), non il default: il pacchetto resterebbe a 25 GB e i Mac da 16 GB dei colleghi non ce la fanno.
- **Qwen 3.5 4B** è il più lento e fa errori di fatto (per esempio scambia una cifra in euro per un numero di giorni) e di italiano («Reporto», «contenrà», «è stato spento molto tempo»). Resta disponibile ma non è più consigliato.
- Large-v3 resta il modello di trascrizione definitivo (non era in discussione).

## Taglie reali di Gemma 4 (Hugging Face, unsloth)

| Modello | Parametri | File usato | Dimensione |
|---|---|---|---|
| Gemma 4 E4B | 4B «effettivi» (~8B con le per-layer embeddings), 42 layer, contesto 128K | `gemma-4-E4B-it-Q4_K_M.gguf` | 4,7 GB |
| Gemma 4 26B-A4B | 26B MoE, 4B attivi per token, 30 layer, contesto 256K | `gemma-4-26B-A4B-it-UD-Q4_K_M.gguf` | 16,2 GB |
| Qwen 3.5 4B | 4B | `Qwen3.5-4B-Q4_K_M.gguf` | 2,6 GB |

`gemma4:latest` di Ollama corrisponde all'E4B (Ollama lo indica come 7.5B Q4_K_M, perché conta anche le embeddings).

## Metodo

- Una copia del DB (`meeting_minutes.sqlite`), le tre riunioni più lunghe:

  | Riunione | Durata | Testo |
  |---|---|---|
  | Teams 23-10 15.18 | 79 min | 56k caratteri |
  | Teams 23-10 16.49 | 14,5 min | 10k caratteri |
  | Meeting 24-10 21.24 | 0,7 min | 0,6k caratteri (verifica che non inventi) |

- La pipeline è quella dell'app (`generate_meeting_summary`, template `standard_meeting`, riassunto in italiano: prima un riassunto base in inglese, poi la traduzione). Il test è ignorato di default e si rilancia con `cargo test --lib bench_summary_model -- --ignored`: il comando completo è nell'intestazione di `src/summary/summary_engine/bench.rs`.
- Tempo: dall'invio del testo al markdown finale, compreso il caricamento del modello alla prima riunione.
- RAM: `/usr/bin/time -l` sul processo `llama-helper`. La «RSS massima» comprende i pesi mappati in memoria; il «footprint» è la memoria che macOS conta davvero all'app.

## Risultati

| Modello | 0,7 min | 14,5 min | 79 min | RSS massima | Footprint di picco |
|---|---|---|---|---|---|
| Gemma 4 E4B (interno) | 38,6 s | 56,4 s | 113,7 s | 7,7 GB | 3,0 GB |
| Gemma 4 26B-A4B (interno) | 35,3 s | 46,9 s | 90,1 s | 24,4 GB | 7,9 GB |
| Qwen 3.5 4B (interno) | 24,0 s | 46,4 s | 180,3 s | 4,0 GB | 1,5 GB |
| *Riferimento: Ollama `gemma4:latest`* | 14,8 s | 28,0 s | 70,7 s | n/d | n/d |

L'E4B sul motore interno è più lento di Ollama sugli stessi pesi, per due motivi. Le per-layer embeddings di Gemma 4 girano su CPU in questa versione di llama.cpp (662 s di CPU sui tre riassunti). E Ollama teneva il modello già caricato. Due minuti per una call di 80 minuti restano accettabili. Se serve, si può aggiornare `llama-cpp-2`.

## Qualità (lettura dei riassunti)

- **Riunione corta**: l'E4B e il 26B riportano le cifre giuste e scrivono «nessuna decisione» e «nessuna azione», come è corretto. Qwen inventa una durata che non c'è.
- **14,5 min**: l'E4B è corretto ma lascia vuota la tabella delle azioni. Qwen trova quattro azioni, ma con errori di italiano e di termini. Il 26B dà la sintesi più chiara.
- **79 min**: tutti colgono i temi. Il 26B è l'unico che riporta correttamente le stime, chi fa cosa e quando, con citazioni pertinenti. L'E4B ha la struttura giusta ma ripete i timestamp. Qwen sbaglia una scadenza.

Nota tecnica: il 26B apre la risposta con un canale di pensiero vuoto (`<|channel>thought<channel|>`). Il client del motore interno ora lo rimuove (`strip_gemma4_thought`).
