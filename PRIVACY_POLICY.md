# Privacy di Dixit

*Ultimo aggiornamento: 2 novembre 2026*

Dixit registra, trascrive e riassume le riunioni **sul tuo Mac**. Con la configurazione di default nessun audio, trascrizione, riassunto o nota esce dal computer.

## Dove stanno i dati

- **Riunioni:** `~/Documents/Dixit/<data ora titolo>/` (audio, trascrizione, note, metadati).
- **Dati dell'app:** `~/Library/Application Support/com.vitomanuguerra.trascrivi/` (database, impostazioni, modelli, `voices.json` con le impronte vocali delle persone già nominate).

Le impronte vocali servono a riconoscere chi parla nelle riunioni successive e restano sul Mac. Si cancellano eliminando `voices.json`; le riunioni si cancellano dall'app o eliminando la loro cartella.

## Cosa esce dal Mac

Solo quello che scegli tu, e solo in questi casi:

- **Aggiornamenti:** l'app controlla se esiste una nuova versione scaricando un file da GitHub. GitHub vede l'indirizzo IP, non i tuoi dati.
- **Modelli (modalità esperto):** se scarichi un modello diverso da quelli inclusi, il file arriva da Hugging Face o dal sito del modello.
- **Provider LLM esterni (modalità esperto):** se scegli OpenAI, Anthropic, Groq, OpenRouter o un endpoint compatibile per i riassunti, la trascrizione della riunione viene inviata a quel servizio, secondo la sua privacy policy. Ollama in locale non invia nulla fuori dal Mac.
- **Server MCP:** se lo attivi, gli agenti sul tuo Mac (per esempio Claude Code) possono leggere riunioni e trascrizioni tramite `127.0.0.1`. Quello che l'agente fa poi con il testo dipende dall'agente.

Dixit non raccoglie statistiche d'uso: il codice di analytics ereditato da Meetily è disattivato e non invia nulla.

## Registrare gli altri

Registrare una riunione può richiedere il consenso dei partecipanti, a seconda del paese e del contesto (per esempio il GDPR in Europa). Avvisa chi partecipa prima di registrare: la responsabilità è di chi usa l'app.

## Contatti

Domande sulla privacy: apri una issue su GitHub. Per un problema di sicurezza segui [SECURITY.md](SECURITY.md).
