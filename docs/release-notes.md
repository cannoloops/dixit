**Novità in 0.2.11**
- Le registrazioni occupano un terzo dello spazio: l'audio si salva a una qualità pensata per la voce, senza differenze nella trascrizione.
- Dixit comprime da solo, in background, l'audio delle riunioni già registrate: libera alcuni GB senza perdere niente.
- In Impostazioni › Registrazioni puoi far spostare nel Cestino l'audio delle riunioni più vecchie di 3, 6 o 12 mesi: trascrizione, riassunto e note restano. Di default è spento.

**Novità in 0.2.10**
- Dixit si collega anche a Codex, Antigravity, Gemini CLI, Cursor e VS Code: in Impostazioni › Automazione › Agenti scegli il tuo agente e copi il comando pronto.
- Su Google Meet nel browser i parlanti hanno il loro nome: Dixit accende i sottotitoli di Meet (li vedi solo tu) e ne prende i nomi. Se li spegni, restano spenti.
- Su Meet in Chrome una call non si spezza più in tante registrazioni quando accendi la camera o condividi lo schermo, e il titolo della riunione è solo il suo nome.

**Novità in 0.2.9**
- «Attiva e configura» nella presentazione degli agenti apre davvero Impostazioni › Automazione, dove trovi il server MCP già acceso.

**Novità in 0.2.8**
- Al primo avvio Dixit ti presenta gli agenti: un'animazione mostra Claude che cerca nelle tue riunioni. Puoi rivederla in Impostazioni › Automazione › Agenti.
- Nella barra a sinistra, «Riassunti in corso» mostra i riassunti che stanno girando, a che punto sono (anche la percentuale della trascrizione definitiva) e una X per fermarli.
- Il riassunto automatico di una singola riunione si può annullare: trascrizione e parlanti vanno avanti, e il riassunto lo generi quando vuoi.
- Gli avvisi di Teams nei sottotitoli («Transcription has started…») non compaiono più come parlante.
- Le maniglie delle tabelle del riassunto non escono più dal pannello quando scorre.

**Novità in 0.2.7**
- Puoi selezionare più riunioni ed eliminarle insieme; la cartella di una riunione eliminata finisce nel Cestino, così si può recuperare.
- Con Teams in inglese, passare alla vista compatta non spezza più la registrazione in due riunioni.
- Se elimini una riunione mentre Dixit la sta ancora elaborando, la sua cartella resta su disco e l'elaborazione arriva in fondo.

Dixit registra, trascrive e riassume le riunioni di Teams, Zoom e Meet, tutto sul tuo Mac.

**Requisiti:** Mac con Apple Silicon, macOS 14.4 o successivo, 16 GB di RAM, circa 10 GB liberi.

**Installazione**
1. Scarica `Dixit_*_aarch64.dmg` qui sotto e trascina Dixit in Applicazioni.
2. Dixit non è notarizzato da Apple: se macOS dice «Apple non è in grado di verificare…», premi **Fine**, poi **Impostazioni di Sistema › Privacy e sicurezza › Apri comunque** e riapri il file. Oppure `xattr -dr com.apple.quarantine /Applications/Dixit.app`. Guida completa: [GUIDA.md](https://github.com/cannoloops/dixit/blob/main/docs/GUIDA.md).
3. Al primo avvio scarica i modelli (circa 9 GB) e chiede i permessi di microfono, registrazione dello schermo e Accessibilità.

Gli aggiornamenti successivi arrivano da soli. Privacy: [PRIVACY_POLICY.md](https://github.com/cannoloops/dixit/blob/main/PRIVACY_POLICY.md).
