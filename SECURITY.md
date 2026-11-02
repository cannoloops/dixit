# Sicurezza

## Segnalare una vulnerabilità

Non aprire una issue pubblica. Usa **Security › Report a vulnerability** nella pagina GitHub del progetto (segnalazione privata): descrivi il problema, la versione di Dixit e i passi per riprodurlo.

Rispondiamo di solito entro una settimana. Ti teniamo aggiornato fino alla correzione e, se vuoi, ti citiamo nelle note di rilascio.

## Versioni supportate

Solo l'ultima versione pubblicata riceve correzioni di sicurezza.

## Cosa rientra

Dixit gira in locale: registra il microfono e l'audio di sistema, legge le finestre di Teams/Zoom/Meet tramite Accessibilità e salva tutto sul Mac. Ci interessano in particolare:

- file non fidati che l'app apre da sola o su richiesta (audio importati, trascrizioni Teams in Download);
- il server MCP locale (`127.0.0.1:7337`, spento di default);
- l'aggiornamento automatico e i binari inclusi nel pacchetto (ffmpeg, llama-helper);
- qualunque modo per usare i permessi di Dixit (microfono, registrazione schermo, Accessibilità) da un altro processo.

Non rientrano gli attacchi che richiedono già il controllo dell'account macOS dell'utente oltre a quanto sopra.
